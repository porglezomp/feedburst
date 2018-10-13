use chrono::{DateTime, Local, Utc, Weekday};
use regex::Regex;
use std::collections::HashSet;
use std::io::{self, Read, Seek, Write};
use std::path::PathBuf;

use crate::error::{Error, ParseError, Span};
use crate::parser::parse_events;

#[derive(Hash, Clone, Debug, PartialEq, Eq)]
pub enum UpdateSpec {
    On(Weekday),
    Every(usize),
    Comics(usize),
    Overlap(usize),
    Filter(FilterType, String),
    OpenAll,
}

#[derive(Hash, Clone, Debug, PartialEq, Eq)]
pub enum FilterType {
    KeepTitle,
    IgnoreTitle,
    KeepUrl,
    IgnoreUrl,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedInfo {
    pub name: String,
    pub url: String,
    pub update_policies: HashSet<UpdateSpec>,
    pub root: Option<PathBuf>,
    pub command: Option<Vec<String>>,
}

impl FeedInfo {
    pub fn read_feed<R: Read>(&self, reader: &mut R) -> Result<Feed, Error> {
        let mut string = String::new();
        reader.read_to_string(&mut string)?;

        let make_error_message = |row: usize, span: Span, msg: &str| -> Error {
            let mut message = format!("Line {}: Error parsing feed \"{}\"\n\n", row, self.name);
            let line = string.lines().nth(row).unwrap_or_default();
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

        let events = match parse_events(&string) {
            Ok(events) => events,
            Err(ParseError::Expected { msg, row, span }) => {
                return Err(make_error_message(row, span, &msg));
            }
        };

        let mut last_read = None;
        let mut new_comics = 0;
        let mut seen_comics = HashSet::new();
        for event in &events {
            match *event {
                FeedEvent::ComicUrl(ref url) => {
                    new_comics += 1;
                    seen_comics.insert(url.clone());
                }
                FeedEvent::Read(date) => {
                    last_read = Some(date);
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

    pub fn filter_title(&self, title: &str) -> bool {
        // @Performance: Avoid compiling so many regexes
        for policy in &self.update_policies {
            match *policy {
                UpdateSpec::Filter(FilterType::KeepTitle, ref pat) => {
                    if !Regex::new(pat).unwrap().is_match(title) {
                        return false;
                    }
                }
                UpdateSpec::Filter(FilterType::IgnoreTitle, ref pat) => {
                    if Regex::new(pat).unwrap().is_match(title) {
                        return false;
                    }
                }
                _ => (),
            }
        }
        true
    }

    pub fn filter_url(&self, url: &str) -> bool {
        // @Performance: Avoid compiling so many regexes
        for policy in &self.update_policies {
            match *policy {
                UpdateSpec::Filter(FilterType::KeepUrl, ref pat) => {
                    if !Regex::new(pat).unwrap().is_match(url) {
                        return false;
                    }
                }
                UpdateSpec::Filter(FilterType::IgnoreUrl, ref pat) => {
                    if Regex::new(pat).unwrap().is_match(url) {
                        return false;
                    }
                }
                _ => (),
            }
        }
        true
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FeedEvent {
    ComicUrl(String),
    Read(DateTime<Utc>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Feed {
    pub info: FeedInfo,
    last_read: Option<DateTime<Utc>>,
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
                self.new_events.push(FeedEvent::ComicUrl(url.clone()));
                self.new_comics += 1;
            }
        }
    }

    pub fn is_scheduled(&self, datetime: DateTime<Local>) -> bool {
        let last_read = match self.last_read {
            Some(last_read) => last_read,
            None => return true,
        };

        let last_read = last_read.with_timezone(&Local);
        let elapsed_time = datetime.signed_duration_since(last_read);
        let mut day_passed = false;
        let mut day_relevant = false;

        for policy in &self.info.update_policies {
            match *policy {
                UpdateSpec::Every(num_days) => {
                    trace!(
                        "Rule for \"{}\": @ every {} days (has been {})",
                        self.info.name,
                        num_days,
                        elapsed_time.num_days()
                    );
                    if elapsed_time.num_days() < num_days as i64 {
                        debug!("Skipping \"{}\" because of @every", self.info.name);
                        return false;
                    }
                    trace!("Rule passed!");
                }
                UpdateSpec::On(day) => {
                    trace!("Rule for \"{}\": @ on {:?}", self.info.name, day);
                    day_relevant = true;
                    use chrono::Datelike;
                    let mut last_day = last_read.weekday();
                    for _ in 0..elapsed_time.num_days() {
                        last_day = last_day.succ();
                        if last_day == day {
                            day_passed = true;
                            trace!("Rule passed!");
                            break;
                        }
                    }
                }
                UpdateSpec::Overlap(_)
                | UpdateSpec::Comics(_)
                | UpdateSpec::Filter(_, _)
                | UpdateSpec::OpenAll => (),
            }
        }

        if day_relevant && !day_passed {
            debug!("Skipping \"{}\" because of @on", self.info.name);
            false
        } else {
            true
        }
    }

    pub fn is_ready(&self) -> bool {
        if self.new_comics < 1 {
            return false;
        }

        if !self.is_scheduled(Local::now()) {
            return false;
        }

        for policy in &self.info.update_policies {
            match *policy {
                UpdateSpec::Comics(num_comics) => {
                    trace!(
                        "Rule for \"{}\": @ {} new comics (has {})",
                        self.info.name,
                        num_comics,
                        self.new_comics
                    );
                    if self.new_comics < num_comics {
                        debug!("Skipping \"{}\" because of @comics", self.info.name);
                        return false;
                    }
                    trace!("Rule passed!");
                }
                UpdateSpec::Every(_)
                | UpdateSpec::On(_)
                | UpdateSpec::Overlap(_)
                | UpdateSpec::Filter(_, _)
                | UpdateSpec::OpenAll => (),
            }
        }
        true
    }

    pub fn read(&mut self) {
        self.new_events.push(FeedEvent::Read(Utc::now()))
    }

    pub fn write_changes<W: Write + Seek>(&mut self, writer: &mut W) -> io::Result<()> {
        writer.seek(io::SeekFrom::End(0))?;
        for event in &self.new_events {
            match *event {
                FeedEvent::ComicUrl(ref url) => writeln!(writer, "<{}>", url)?,
                FeedEvent::Read(date) => writeln!(writer, "read {}", date.to_rfc3339())?,
            }
        }
        trace!(
            "Wrote changes for \"{}\", new events moved to old",
            self.info.name
        );
        self.events.append(&mut self.new_events);
        Ok(())
    }

    pub fn get_reading_list(&self) -> Vec<String> {
        let mut additional = 0;
        for policy in &self.info.update_policies {
            if let UpdateSpec::Overlap(n) = *policy {
                additional = ::std::cmp::max(n, additional);
            }
        }
        trace!(
            "Reading list for \"{}\", overlap {}",
            self.info.name,
            additional
        );
        let mut finishing = false;
        let mut result = Vec::new();
        for event in self.events.iter().chain(&self.new_events).rev() {
            match *event {
                FeedEvent::ComicUrl(ref url) => {
                    if finishing {
                        if additional == 0 {
                            break;
                        }
                        additional -= 1;
                    }
                    result.push(url.clone());
                }
                FeedEvent::Read(when) => {
                    finishing = true;
                    trace!("Read at {}", when);
                }
            }
        }
        debug!(
            "Reading list for \"{}\" has {}",
            self.info.name,
            result.len()
        );
        result.reverse();
        result
    }
}
