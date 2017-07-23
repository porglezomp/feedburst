#[macro_use]
extern crate log;
extern crate pretty_env_logger;
extern crate reqwest;
extern crate syndication;
extern crate chrono;
extern crate clap;
extern crate open;

#[cfg(unix)]
extern crate xdg;

use std::io::Read;
use std::str::FromStr;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;

use clap::{Arg, App};

mod parser;
mod feed;
mod error;
mod config;

use feed::{Feed, FeedInfo};
use error::{Error, ParseError, Span};

const APP_NAME: &'static str = "feedburst";

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Error> {
    pretty_env_logger::init().unwrap();
    let matches = App::new(APP_NAME)
        .version("0.2")
        .author("Caleb Jones <code@calebjones.net>")
        .about("Presents you your RSS feeds in chunks")
        .arg(
            Arg::with_name("config")
                .long("config")
                .value_name("FILE")
                .help("The config file to load feeds from")
                .takes_value(true),
        )
        .arg(Arg::with_name("fetch").long("fetch").help(
            "Only download feeds, don't view them",
        ))
        .get_matches();

    let config_path = get_config(matches.value_of("config"))?;
    let only_fetch = matches.value_of("fetch").is_some();

    let feeds = {
        let mut file = match config_path {
            ConfigPath::Central(ref path) => {
                OpenOptions::new()
                    .create(true)
                    .write(true)
                    .read(true)
                    .open(path)?
            }
            ConfigPath::Arg(ref path) => {
                File::open(path).map_err(|_| {
                    Error::Msg(format!("Cannot open file {:?}", path))
                })?
            }
        };
        let mut text = String::new();
        file.read_to_string(&mut text)?;

        let make_error_message = |row: usize, span: Span, msg: &str| -> Error {
            let mut message = format!("Line {}: Error parsing {:?}\n\n", row, config_path);
            let line = text.lines().nth(row).unwrap_or_default();
            message.push_str(&format!("{}\n", line));
            match span {
                None => message.push('\n'),
                Some((l, r)) => {
                    let underline = format!("{}{}\n", " ".repeat(l), "^".repeat(r - l + 1));
                    message.push_str(&underline);
                }
            }

            message.push_str(&format!("Expected {}", msg));
            Error::Msg(message)
        };

        match parser::parse_config(&text) {
            Ok(feeds) => feeds,
            Err(ParseError::Expected { chr, row, span }) => {
                let msg = format!("'{}'", chr);
                return Err(make_error_message(row, span, &msg));
            }
            Err(ParseError::ExpectedMsg { msg, row, span }) => {
                return Err(make_error_message(row, span, &msg));
            }
        }
    };

    if feeds.is_empty() {
        println!(
            "You're not following any comics. Add some to your config file at {}",
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
            std::thread::spawn(move || for info in group {
                match fetch_feed(&info) {
                    Ok(feed) => tx.send(feed).unwrap(),
                    Err(Error::Msg(err)) => eprintln!("{}", err),
                    Err(err) => eprintln!("Error in feed {}: {}", info.name, err),
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
            ConfigPath::Arg(ref path) => write!(fmt, "{}", path.to_string_lossy()),
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

    let path = config::get_config_path()?;
    debug!("Using config found from the XDG config dir: {:?}", path);
    Ok(ConfigPath::Central(path))
}

fn fetch_feed(feed_info: &FeedInfo) -> Result<Feed, Error> {
    debug!("Fetching \"{}\" from <{}>", feed_info.name, feed_info.url);
    let client = reqwest::ClientBuilder::new()?
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    let mut resp = client.get(&feed_info.url)?.send()?;
    if !resp.status().is_success() {
        debug!(
            "Error \"{}\" fetching feed {} from {}",
            resp.status(),
            feed_info.name,
            feed_info.url,
        );
        return Err(Error::Msg(format!(
            "{} (Failed to download: \"{}\")",
            feed_info.name,
            resp.status(),
        )));
    }
    let mut content = String::new();
    resp.read_to_string(&mut content)?;
    let links: Vec<_> = {
        use syndication::Feed;
        match Feed::from_str(&content).map_err(|x| Error::Msg(x.into()))? {
            Feed::Atom(feed) => {
                debug!("Parsed feed <{}> as Atom", feed_info.url);
                feed.entries
                    .into_iter()
                    .rev()
                    .filter_map(|x| x.links.first().cloned())
                    .map(|x| x.href)
                    .collect()
            }
            Feed::RSS(feed) => {
                debug!("Parsed feed <{}> as RSS", feed_info.url);
                feed.items
                    .into_iter()
                    .rev()
                    .filter_map(|x| x.link)
                    .collect()
            }
        }
    };

    let mut file = feed_info_file(feed_info)?;
    let mut feed = feed_info.read_feed(&mut file)?;
    feed.add_new_comics(&links);
    feed.write_changes(&mut file)?;
    Ok(feed)
}

fn feed_info_file(feed_info: &FeedInfo) -> Result<File, Error> {
    let path = config::get_feed_path(&feed_info.name)?;

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
    if items.is_empty() {
        return Ok(());
    }
    let plural_feeds = if items.len() == 1 { "comic" } else { "comics" };
    println!("{} ({} {})", feed.info.name, items.len(), plural_feeds);
    open::that(items.first().unwrap())?;
    feed.read();
    feed.write_changes(&mut file)?;
    Ok(())
}
