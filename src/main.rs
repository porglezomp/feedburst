#[macro_use]
extern crate nom;
extern crate reqwest;
extern crate syndication;
extern crate xdg;
extern crate chrono;
extern crate clap;

use std::io::Read;
use std::str::FromStr;

use clap::{Arg, App};

mod parser;
mod feed;

use feed::{Feed, FeedInfo};

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
    let matches = App::new("feedburst")
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
        let mut file = std::fs::File::open(config_path)?;
        let mut text = String::new();
        file.read_to_string(&mut text)?;
        parser::parse_config(&text)?
    };
    let mut prepared_feeds = Vec::new();

    // @Performance: Use hyper to fetch streams concurrently
    for feed_info in feeds {
        let feed = match fetch_feed(&feed_info) {
            Ok(feed) => feed,
            Err(err) => {
                println!("Error in feed {}: {}", feed_info.name, err);
                continue;
            }
        };

        if feed.is_ready() {
            prepared_feeds.push(feed);
        }
    }

    if only_fetch {
        return Ok(());
    }

    for mut feed in prepared_feeds {
        if let Err(err) = read_feed(&mut feed) {
            println!("Error in feed {}: {}", feed.info.name, err);
        }
    }

    Ok(())
}

fn get_config(path: Option<&str>) -> Result<std::path::PathBuf, Error> {
    if let Some(path) = path {
        return Ok(path.into());
    }

    if let Ok(path) = std::env::var("FEEDBURST_CONFIG_PATH") {
        return Ok(path.into());
    }

    Ok(xdg::BaseDirectories::with_prefix("feedburst")?
        .place_config_file("config.feeds")?)
}

fn fetch_feed(feed_info: &FeedInfo) -> Result<Feed, Error> {
    let mut resp = reqwest::get(&feed_info.url)?;
    let mut content = String::new();
    resp.read_to_string(&mut content).expect("Read failure");
    let links: Vec<_> = {
        use syndication::Feed;
        match Feed::from_str(&content)
            .map_err(|x| Error::ParseFeed(x.into()))? {
            Feed::Atom(feed) => {
                feed.entries
                    .into_iter()
                    .filter_map(|x| x.links.first().cloned())
                    .map(|x| x.href)
                    .collect()
            }
            Feed::RSS(feed) => feed.items.into_iter().filter_map(|x| x.link).collect(),
        }
    };

    let mut file = feed_info_file(&feed_info)?;
    let mut feed = feed_info.read_feed(&mut file)?;
    feed.add_new_comics(&links);
    feed.write_changes(&mut file)?;
    Ok(feed)
}

fn feed_info_file(feed_info: &FeedInfo) -> Result<std::fs::File, Error> {
    let path = format!("feeds/{}.feed", feed_info.name);
    let path = xdg::BaseDirectories::with_prefix("feedburst")?
        .place_data_file(&path)?;

    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .map_err(From::from)
}

fn read_feed(feed: &mut Feed) -> Result<(), Error> {
    println!("{}", feed.info.name);
    let mut file = feed_info_file(&feed.info)?;
    let items = feed.get_reading_list();
    for item in items {
        println!("{}", item);
    }
    feed.read();
    feed.write_changes(&mut file)?;
    Ok(())
}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::Io(ref err) => write!(fmt, "IO error: {}", err),
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
    Parse(parser::ParseError),
    ParseFeed(String),
    Request(reqwest::Error),
    LoadFeed(feed::LoadFeedError),
    BaseDirectory(xdg::BaseDirectoriesError),
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

impl From<xdg::BaseDirectoriesError> for Error {
    fn from(err: xdg::BaseDirectoriesError) -> Error {
        Error::BaseDirectory(err)
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref err) => err.description(),
            Error::Parse(ref _err) => "Error parsing config",
            Error::Request(ref err) => err.description(),
            Error::LoadFeed(ref err) => err.description(),
            Error::ParseFeed(ref _err) => "Error parsing feed",
            Error::BaseDirectory(ref err) => err.description(),
        }
    }
}
