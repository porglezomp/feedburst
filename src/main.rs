#[macro_use]
extern crate nom;
extern crate reqwest;
extern crate syndication;
extern crate xdg;
extern crate chrono;

use std::io::Read;
use std::str::FromStr;

use syndication::Feed;

mod parser;
mod feed;

fn main() {
    let fname = match std::env::args().nth(1) {
        Some(fname) => fname,
        None => {
            println!("Please pass a filename");
            return;
        }
    };

    let mut file = match std::fs::File::open(fname) {
        Ok(file) => file,
        Err(err) => {
            println!("Error opening file {}", err);
            return;
        }
    };

    let mut text = String::new();
    file.read_to_string(&mut text)
        .expect("IO Error");

    let feeds = match parser::parse_config(&text) {
        Ok(feeds) => feeds,
        Err(err) => {
            println!("Error parsing config: {:?}", err);
            return;
        }
    };

    let xdg = xdg::BaseDirectories::with_prefix("feedburst")
        .expect("Failed to get xdg directories");

    // @Performance: Use hyper to fetch streams concurrently
    for feed in feeds {
        let mut resp = match reqwest::get(&feed.url) {
            Ok(resp) => resp,
            Err(err) => {
                println!("Error fetching feed {}: {:?}", feed.url, err);
                continue;
            }
        };

        let mut content = String::new();
        resp.read_to_string(&mut content)
            .expect("Read failure");
        let links: Vec<_> = match Feed::from_str(&content) {
            Ok(Feed::Atom(feed)) => {
                feed.entries
                    .into_iter()
                    .filter_map(|x| x.links.first().cloned())
                    .map(|x| x.href)
                    .collect()
            }
            Ok(Feed::RSS(feed)) => {
                feed.items
                    .into_iter()
                    .filter_map(|x| x.link)
                    .collect()
            }
            Err(err) => {
                println!("Error parsing feed {}: {:?}", feed.url, err);
                continue;
            }
        };

        let path = format!("feeds/{}.feed", feed.name);
        let path = match xdg.place_data_file(&path) {
            Ok(path) => path,
            Err(err) => {
                println!("Error creating feed {}: {}", path, err);
                continue;
            }
        };

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path);

        let mut file = match file {
            Ok(file) => file,
            Err(err) => {
                println!("Error opening feed {:?}: {}", path, err);
                continue;
            }
        };

        let mut feed = match feed.read_feed(&mut file) {
            Ok(feed) => feed,
            Err(err) => {
                println!("Error reading feed: {}", err);
                continue;
            }
        };
        feed.add_new_comics(&links);
        if feed.is_ready() {
            println!("Read the feed {}!", feed.info.url);
            feed.read();
        }
        if let Err(err) = feed.write_changes(&mut file) {
            println!("Error writing feed {}: {}", feed.info.url, err);
            continue;
        }
    }
}

