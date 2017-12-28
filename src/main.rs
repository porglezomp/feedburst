extern crate chrono;
extern crate clap;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
extern crate regex;
extern crate reqwest;
extern crate syndication;
extern crate xdg;

use std::io::Read;
use std::str::FromStr;

use clap::{App, Arg};

mod parser;
mod parse_util;
mod feed;
mod error;
mod config;
mod platform;

use feed::Feed;
use error::{Error, ParseError, Span};

const APP_NAME: &str = env!("CARGO_PKG_NAME");

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Error> {
    pretty_env_logger::init().unwrap();
    let matches = App::new(APP_NAME)
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("Presents you your RSS feeds in chunks")
        .arg(
            Arg::with_name("config")
                .long("config")
                .value_name("FILE")
                .help("The config file to load feeds from")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("feeds")
                .long("feeds")
                .value_name("PATH")
                .help("The folder where feeds are stored")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("open-with")
                .long("open-with")
                .value_name("COMMAND")
                .help(concat!(
                    "The command to open the comic with. Any instance of @URL ",
                    "will be replaced with the comic URL, and if @URL isn't ",
                    "mentioned, the URL will be placed at the end of the command.",
                ))
                .takes_value(true),
        )
        .arg(
            Arg::with_name("fetch")
                .long("fetch")
                .help("Only download feeds, don't view them"),
        )
        .max_term_width(120)
        .get_matches();

    let only_fetch = matches.value_of("fetch").is_some();
    let args = config::Args::new(
        only_fetch,
        matches.value_of("feeds"),
        matches.value_of("config"),
        matches.value_of("open-with"),
    )?;

    let feeds = {
        let mut file = args.config_file()?;
        let mut text = String::new();
        file.read_to_string(&mut text)?;

        let make_error_message = |row: usize, span: Span, msg: &str| -> Error {
            let mut message = format!("Line {}: Error parsing {:?}\n\n", row, args.config_path(),);
            let line = text.lines().nth(row - 1).unwrap_or_default();
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
            Err(ParseError::Expected { msg, row, span }) => {
                return Err(make_error_message(row, span, &msg));
            }
        }
    };

    if feeds.is_empty() {
        println!(
            "You're not following any comics. Add some to your config file at {:?}",
            args.config_path(),
        );
        return Ok(());
    }

    let mut feeds: Vec<_> = feeds
        .into_iter()
        .map(|info| {
            let mut feed_file = args.feed_file(&info)?;
            info.read_feed(&mut feed_file)
        })
        .filter_map(|feed| match feed {
            Ok(feed) => Some(feed),
            Err(err) => {
                eprintln!("{}", err);
                None
            }
        })
        .collect();

    // Fetch the feeds that are currently scheduled, not those that are unscheduled
    feeds.sort_by_key(|feed| !feed.is_scheduled());

    let rx = {
        let (tx, rx) = std::sync::mpsc::channel();
        const NUM_THREADS: usize = 4;
        let mut groups: Vec<Vec<Feed>> = vec![vec![]; NUM_THREADS];
        for (i, feed) in feeds.into_iter().enumerate() {
            groups[i % NUM_THREADS].push(feed);
        }

        for group in groups {
            let tx = tx.clone();
            let args = args.clone();
            std::thread::spawn(move || {
                for feed in group {
                    let name = feed.info.name.clone();
                    match fetch_feed(&args, feed) {
                        Ok(feed) => tx.send(feed).unwrap(),
                        Err(Error::Msg(err)) => eprintln!("{}", err),
                        Err(err) => eprintln!("Error in feed {}: {}", name, err),
                    }
                }
            });
        }

        rx
    };

    let mut num_read = 0;
    for mut feed in rx {
        if feed.is_ready() && !only_fetch {
            if let Err(err) = read_feed(&args, &mut feed) {
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

fn fetch_feed(args: &config::Args, mut feed: Feed) -> Result<Feed, Error> {
    debug!("Fetching \"{}\" from <{}>", feed.info.name, feed.info.url);
    let client = reqwest::ClientBuilder::new()?
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    let mut resp = client.get(&feed.info.url)?.send()?;
    if !resp.status().is_success() {
        debug!(
            "Error \"{}\" fetching feed {} from {}",
            resp.status(),
            feed.info.name,
            feed.info.url,
        );
        return Err(Error::Msg(format!(
            "{} (Failed to download: \"{}\")",
            feed.info.name,
            resp.status(),
        )));
    }
    let mut content = String::new();
    resp.read_to_string(&mut content)?;
    let links: Vec<_> = {
        use syndication::Feed;
        let feed_info = &feed.info;
        match Feed::from_str(&content).map_err(|x| Error::Msg(x.into()))? {
            Feed::Atom(feed) => {
                debug!("Parsed feed <{}> as Atom", feed_info.url);
                feed.entries
                    .into_iter()
                    .rev()
                    .filter(|x| {
                        let keep = feed_info.filter_title(&x.title);
                        if !keep {
                            println!("skipping by title: {}", x.title);
                        }
                        keep
                    })
                    .filter_map(|x| x.links.first().cloned())
                    .map(|x| x.href)
                    .filter(|url| feed_info.filter_url(&url))
                    .collect()
            }
            Feed::RSS(feed) => {
                debug!("Parsed feed <{}> as RSS", feed_info.url);
                feed.items
                    .into_iter()
                    .rev()
                    .filter(|x| {
                        let title = &x.title;
                        let title = title.as_ref().map(|x| &x[..]).unwrap_or("");
                        let keep = feed_info.filter_title(&title);
                        if !keep {
                            println!("skipping by title: {:?}", x.title);
                        }
                        keep
                    })
                    .filter_map(|x| x.link)
                    .filter(|url| feed_info.filter_url(&url))
                    .collect()
            }
        }
    };

    let mut feed_file = args.feed_file(&feed.info)?;
    feed.add_new_comics(&links);
    feed.write_changes(&mut feed_file)?;
    Ok(feed)
}

fn read_feed(args: &config::Args, feed: &mut Feed) -> Result<(), Error> {
    let mut feed_file = args.feed_file(&feed.info)?;
    let items = feed.get_reading_list();
    if items.is_empty() {
        return Ok(());
    }
    let plural_feeds = if items.len() == 1 { "comic" } else { "comics" };
    println!("{} ({} {})", feed.info.name, items.len(), plural_feeds);
    args.open_url(&feed.info, items.first().unwrap())?;
    feed.read();
    feed.write_changes(&mut feed_file)?;
    Ok(())
}
