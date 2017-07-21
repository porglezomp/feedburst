#[macro_use]
extern crate log;
extern crate pretty_env_logger;
#[macro_use]
extern crate nom;
extern crate reqwest;
extern crate syndication;
extern crate chrono;
extern crate clap;
extern crate open;

// I don't want to put config in ~/Library/... on Mac, so we use XDG there too
#[cfg(unix)]
extern crate xdg;
#[cfg(windows)]
extern crate app_dirs;

// @Polish: Change error println!() to eprintln!()

use std::io::Read;
use std::str::FromStr;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;

use clap::{Arg, App};
#[cfg(windows)]
use app_dirs::{AppInfo, AppDataType, get_app_dir, get_app_root};

mod parser;
mod feed;

use feed::{Feed, FeedInfo};

const APP_NAME: &'static str = "feedburst";
#[cfg(windows)]
const APP_INFO: AppInfo = AppInfo {
    name: APP_NAME,
    author: "porglezomp",
};

fn main() {
    std::process::exit(match run() {
        Ok(()) => 0,
        Err(err) => {
            println!("Error: {}", err);
            1
        }
    })
}

fn run() -> Result<(), Error> {
    pretty_env_logger::init().unwrap();
    let matches = App::new(APP_NAME)
        .version("0.1")
        .author("Caleb Jones <code@calebjones.net>")
        .about("Presents you your RSS feeds in chunks")
        .arg(
            Arg::with_name("config")
                .long("config")
                .value_name("FILE")
                .help("The config file to load feeds from")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("fetch")
                .long("fetch")
                .help("Only download feeds, don't view them"),
        )
        .get_matches();

    let config_path = get_config(matches.value_of("config"))?;
    let only_fetch = matches.value_of("fetch").is_some();

    let feeds = {
        let mut file = match config_path {
            ConfigPath::Central(ref path) =>
                OpenOptions::new()
                    .create(true)
                    .write(true)
                    .read(true)
                    .open(path)?,
            ConfigPath::Arg(ref path) =>
                File::open(path)
                .map_err(|_| {
                    Error::Msg(format!("Cannot open file {:?}", path))
                })?,
        };
        let mut text = String::new();
        file.read_to_string(&mut text)?;
        parser::parse_config(&text)?
    };

    if feeds.is_empty() {
        println!(
            "You're not following any comics. Add some to your config file at {:?}",
            config_path,
        );
        return Ok(());
    }

    // @Performance: Sort the feed info to put the most useful ones first

    let rx = {
        let (tx, rx) = std::sync::mpsc::channel();
        const NUM_THREADS: usize = 4;
        let mut groups = vec![vec![]; NUM_THREADS];
        for (i, feed_info) in feeds.into_iter().enumerate() {
            groups[i % NUM_THREADS].push(feed_info);
        }

        for group in groups {
            let tx = tx.clone();
            std::thread::spawn(move || {
                for info in group {
                    match fetch_feed(&info) {
                        Ok(feed) => tx.send(feed).unwrap(),
                        Err(err) => eprintln!("Error in feed {}: {}", info.name, err),
                    }
                }
            });
        }

        rx
    };

    let mut num_read = 0;
    for mut feed in rx {
        if feed.is_ready() && !only_fetch {
            if let Err(err) = read_feed(&mut feed) {
                eprintln!("Error in feed {}: {}", feed.info.name, err);
            } else {
                num_read += 1;
            }
        }
    }

    if num_read == 0 && !only_fetch {
        // @Todo: Provide a better estimate of when new comics will be available.
        println!("No new comics. Check back tomorrow!");
    }

    Ok(())
}

#[derive(Debug)]
enum ConfigPath {
    Central(PathBuf),
    Arg(PathBuf),
}

impl std::fmt::Display for ConfigPath {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            ConfigPath::Central(ref path) |
            ConfigPath::Arg(ref path) => write!(fmt, "{:?}", path),
        }
    }
}

fn get_config(path: Option<&str>) -> Result<ConfigPath, Error> {
    if let Some(path) = path {
        debug!("Using config specified on command line: {}", path);
        return Ok(ConfigPath::Arg(path.into()));
    }

    if let Ok(path) = std::env::var("FEEDBURST_CONFIG_PATH") {
        debug!("Using config specified as FEEDBURST_CONFIG_PATH: {}", path);
        return Ok(ConfigPath::Central(path.into()));
    }

    #[cfg(unix)]
    fn fallback() -> Result<PathBuf, Error> {
        Ok(xdg::BaseDirectories::with_prefix(APP_NAME)?
           .place_config_file("config.feeds")?)
    }

    #[cfg(windows)]
    fn fallback() -> Result<PathBuf, Error> {
        let mut dir = get_app_root(AppDataType::UserConfig, &APP_INFO)?;
        dir.push("config.feeds");
        Ok(dir)
    }

    let path = fallback()?;
    debug!("Using config found from the XDG config dir: {:?}", path);
    Ok(ConfigPath::Central(path))
}

fn fetch_feed(feed_info: &FeedInfo) -> Result<Feed, Error> {
    debug!("Fetching \"{}\" from <{}>", feed_info.name, feed_info.url);
    let client = reqwest::ClientBuilder::new()?
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    let mut resp = client.get(&feed_info.url)?.send()?;
    let mut content = String::new();
    resp.read_to_string(&mut content)?;
    let links: Vec<_> = {
        use syndication::Feed;
        match Feed::from_str(&content)
            .map_err(|x| Error::ParseFeed(x.into()))? {
            Feed::Atom(feed) => {
                debug!("Parsed feed <{}> as Atom", feed_info.url);
                feed.entries
                    .into_iter()
                    .rev()
                    .filter_map(|x| x.links.first().cloned())
                    .map(|x| x.href)
                    .collect()
            }
            Feed::RSS( feed) => {
                debug!("Parsed feed <{}> as RSS", feed_info.url);
                feed.items
                    .into_iter()
                    .rev()
                    .filter_map(|x| x.link)
                    .collect()
            }
        }
    };

    let mut file = feed_info_file(&feed_info)?;
    let mut feed = feed_info.read_feed(&mut file)?;
    feed.add_new_comics(&links);
    feed.write_changes(&mut file)?;
    Ok(feed)
}

fn feed_info_file(feed_info: &FeedInfo) -> Result<File, Error> {
    #[cfg(unix)]
    fn get_path(name: &str) -> Result<PathBuf, Error> {
        let path = format!("feeds/{}.feed", name);
        Ok(xdg::BaseDirectories::with_prefix(APP_NAME)?
           .place_data_file(&path)?)
    }

    #[cfg(windows)]
    fn get_path(name: &str) -> Result<PathBuf, Error> {
        let mut path = get_app_dir(AppDataType::UserData, &APP_INFO, "feeds")?;
        path.push(format!("{}.feed", name));
        Ok(path)
    }

    let path = get_path(&feed_info.name)?;

    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .map_err(From::from)
}

fn read_feed(feed: &mut Feed) -> Result<(), Error> {
    let mut file = feed_info_file(&feed.info)?;
    let items = feed.get_reading_list();
    if items.len() == 0 {
        return Ok(());
    }
    let plural_feeds = if items.len() == 1 {
        "comic"
    } else {
        "comics"
    };
    println!("{} ({} {})", feed.info.name, items.len(), plural_feeds);
    open::that(items.first().unwrap())?;
    feed.read();
    feed.write_changes(&mut file)?;
    Ok(())
}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::Io(ref err) => write!(fmt, "IO error: {}", err),
            Error::Msg(ref err) => write!(fmt, "Error: {}", err),
            Error::Parse(ref err) => write!(fmt, "Parse error: {:?}", err),
            Error::Request(ref err) => write!(fmt, "Request error: {}", err),
            Error::LoadFeed(ref err) => write!(fmt, "Error loading feed: {}", err),
            Error::ParseFeed(ref err) => write!(fmt, "Error parsing feed: {}", err),
            Error::BaseDirectory(ref err) => write!(fmt, "Error getting base dir: {}", err),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Msg(String),
    Parse(parser::ParseError),
    ParseFeed(String),
    Request(reqwest::Error),
    LoadFeed(feed::LoadFeedError),
    #[cfg(unix)]
    BaseDirectory(xdg::BaseDirectoriesError),
    #[cfg(windows)]
    BaseDirectory(app_dirs::AppDirsError),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<parser::ParseError> for Error {
    fn from(err: parser::ParseError) -> Error {
        Error::Parse(err)
    }
}

impl From<feed::LoadFeedError> for Error {
    fn from(err: feed::LoadFeedError) -> Error {
        Error::LoadFeed(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Error {
        Error::Request(err)
    }
}

#[cfg(unix)]
impl From<xdg::BaseDirectoriesError> for Error {
    fn from(err: xdg::BaseDirectoriesError) -> Error {
        Error::BaseDirectory(err)
    }
}

#[cfg(windows)]
impl From<app_dirs::AppDirsError> for Error {
    fn from(err: app_dirs::AppDirsError) -> Error {
        Error::BaseDirectory(err)
    }
}

impl From<String> for Error {
    fn from(err: String) -> Error {
        Error::Msg(err)
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref err) => err.description(),
            Error::Msg(ref err) => &err,
            Error::Parse(ref _err) => "Error parsing config",
            Error::Request(ref err) => err.description(),
            Error::LoadFeed(ref err) => err.description(),
            Error::ParseFeed(ref _err) => "Error parsing feed",
            #[cfg(unix)]
            Error::BaseDirectory(ref err) => err.description(),
            #[cfg(windows)]
            Error::BaseDirectory(ref err) => err.description(),
        }
    }
}
