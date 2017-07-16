use std::collections::HashSet;
use chrono::{DateTime, Utc, MIN_DATE};
use std::io::{self, Read, Write};
use std::error;
use nom::{space, multispace};
use std::str::FromStr;

#[derive(Hash, Copy, Clone, Debug, PartialEq, Eq)]
pub enum Weekday {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

#[derive(Hash, Copy, Clone, Debug, PartialEq, Eq)]
pub enum UpdateSpec {
    On(Weekday),
    Every(usize),
    Comics(usize),
    Overlap(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedInfo {
    pub name: String,
    pub url: String,
    pub updates: HashSet<UpdateSpec>,
}

#[derive(Debug)]
pub enum LoadFeedError {
    Io(io::Error),
    ParseFailure,
}

impl ::std::fmt::Display for LoadFeedError {
    fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            LoadFeedError::Io(ref err) => write!(fmt, "{}", err),
            LoadFeedError::ParseFailure => write!(fmt, "Parse failure"),
        }
    }
}

impl From<io::Error> for LoadFeedError {
    fn from(err: io::Error) -> Self {
        LoadFeedError::Io(err)
    }
}

impl error::Error for LoadFeedError {
    fn description(&self) -> &str {
        match *self {
            LoadFeedError::Io(ref err) => err.description(),
            LoadFeedError::ParseFailure => "failed parsing feed",
        }
    }
}

impl FeedInfo {
    pub fn read_feed<R: Read>(&self, reader: &mut R) -> Result<Feed, LoadFeedError> {
        use nom::IResult;

        let mut string = String::new();
        reader.read_to_string(&mut string)?;
        let events = match parse_events(&string) {
            IResult::Done("", res) => res,
            IResult::Done(_, _) | IResult::Error(_) | IResult::Incomplete(_)
                => return Err(LoadFeedError::ParseFailure),
        };
        let mut last_read = MIN_DATE.and_hms(0, 0, 0);
        let mut new_comics = 0;
        let mut seen_comics = HashSet::new();
        for event in &events {
            match *event {
                FeedEvent::ComicUrl(ref url) => {
                    new_comics += 1;
                    seen_comics.insert(url.clone());
                }
                FeedEvent::Read(date) => {
                    last_read = date;
                    new_comics = 0;
                }
            }
        }
        Ok(Feed {
            info: self.clone(),
            new_events: Vec::new(),
            seen_comics,
            last_read,
            new_comics,
            events,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
enum FeedEvent {
    ComicUrl(String),
    Read(DateTime<Utc>),
}

pub struct Feed {
    pub info: FeedInfo,
    last_read: DateTime<Utc>,
    new_comics: usize,
    seen_comics: HashSet<String>,
    new_events: Vec<FeedEvent>,
    events: Vec<FeedEvent>,
}

impl Feed {
    pub fn add_new_comics<S: ::std::borrow::Borrow<String>>(&mut self, urls: &[S]) {
        for url in urls {
            let url = url.borrow();
            if !self.seen_comics.contains(url) {
                println!("{}", url);
                self.new_events.push(FeedEvent::ComicUrl(url.clone()));
            }
        }
    }

    pub fn is_ready(&self) -> bool {
        let elapsed_time = Utc::now().signed_duration_since(self.last_read);
        for policy in &self.info.updates {
            match *policy {
                UpdateSpec::Every(num_days) => {
                    if elapsed_time.num_days() < num_days as i64 {
                        return false;
                    }
                }
                UpdateSpec::Comics(num_comics) => {
                    if self.new_comics < num_comics {
                        return false;
                    }
                }
                UpdateSpec::On(day) => {
                    //@Todo: Implement waiting for a given day
                }
                UpdateSpec::Overlap(_) => (),
            }
        }
        true
    }

    pub fn read(&mut self) {
        self.new_events.push(FeedEvent::Read(Utc::now()))
    }

    pub fn write_changes<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        for event in &self.new_events {
            match *event {
                FeedEvent::ComicUrl(ref url) =>
                    writeln!(writer, "<{}>", url)?,
                FeedEvent::Read(date) =>
                    writeln!(writer, "read {}", date.to_rfc3339())?,
            }
        }
        Ok(())
    }
}

named!(parse_events<&str, Vec<FeedEvent>>,
    do_parse!(
        events: many0!(event) >>
        opt!(complete!(multispace)) >>
        eof!() >>

        (events)
    )
);

named!(event<&str, FeedEvent>,
    preceded!(opt!(multispace),
        complete!(alt_complete!(
            urlevent
            | readevent
        ))
    )
);

named!(urlevent<&str, FeedEvent>,
    do_parse!(
        char!('<') >>
        url: is_not!(">") >>
        char!('>') >>

        (FeedEvent::ComicUrl(url.into()))
    )
);

named!(readevent<&str, FeedEvent>,
    do_parse!(
        tag!("read") >>
        space >>
        read_date: date >>

        (FeedEvent::Read(read_date))
    )
);

named!(date<&str, DateTime<Utc>>,
    map_res!(take_until_either!("\n\r "), DateTime::from_str)
);
