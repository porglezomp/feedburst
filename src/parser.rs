use std::collections::HashSet;
use std::iter::FromIterator;

use chrono::Weekday;

use feed::{FeedInfo, UpdateSpec, FeedEvent};
use error::ParseError;

fn not_space(x: char) -> bool {
    !x.is_whitespace()
}

pub fn parse_config(input: &str) -> Result<Vec<FeedInfo>, ParseError> {
    let mut out = Vec::new();
    for (row, line) in input.lines().enumerate() {
        if let Some(col) = line.find(|x| x != ' ' && x != '\t') {
            if line[col..].chars().next() != Some('#') {
                out.push(parse_line(row, col, line)?);
            }
        }
    }
    Ok(out)
}

pub fn parse_line(row: usize, col: usize, input: &str) -> Result<FeedInfo, ParseError> {
    let (col, name) = parse_name(row, col, input)?;
    let (col, url) = parse_url(row, col, input)?;
    let policies = parse_policies(row, col, input)?;
    Ok(FeedInfo {
        name,
        url,
        updates: HashSet::from_iter(policies),
    })
}

fn find_char(row: usize, col: usize, input: &str, c: char) -> Result<usize, ParseError> {
    match input[col..].find(c) {
        Some(off) => Ok(off + col),
        None => Err(ParseError::expected_char(c, row, None)),
    }
}

fn parse_name(row: usize, col: usize, input: &str) -> Result<(usize, String), ParseError> {
    let start_col = match input[col..].find(not_space) {
        Some(off) => off + col,
        None => return Err(ParseError::expected_char('"', row, None)),
    };
    if input[start_col..].chars().next() != Some('"') {
        return Err(ParseError::expected_char('"', row, start_col));
    }
    let end_col = find_char(row, start_col+1, input, '"')?;
    Ok((end_col+1, input[start_col+1..end_col].into()))
}

fn parse_url(row: usize, col: usize, input: &str) -> Result<(usize, String), ParseError> {
    let start_col = match input[col..].find(not_space) {
        Some(off) => off + col,
        None => return Err(ParseError::expected_char('<', row, None)),
    };
    if input[start_col..].chars().next() != Some('<') {
        return Err(ParseError::expected_char('<', row, start_col));
    }
    let end_col = find_char(row, start_col+1, input, '>')?;
    Ok((end_col+1, input[start_col+1..end_col].into()))
}

fn parse_policies(row: usize, col: usize, input: &str) -> Result<Vec<UpdateSpec>, ParseError> {
    let mut out = Vec::new();
    let start_col = match input[col..].find(not_space) {
        Some(off) => col + off,
        None => return Ok(out),
    };
    if input[start_col..].chars().next() != Some('@') {
        return Err(ParseError::expected_char('@', row, start_col));
    }

    let mut col = start_col;
    for policy_chunk in input[start_col+1..].split('@') {
        out.push(parse_policy(row, col, &policy_chunk.to_lowercase())?);
        col += 1 + policy_chunk.len();
    }

    Ok(out)
}

fn parse_policy(row: usize, col: usize, input: &str) -> Result<UpdateSpec, ParseError> {
    let self_span = (col, col + input.len());
    let error = ParseError::expected(r#"a policy definition. One of:
 - "@ on WEEKDAY"
 - "@ every # day(s)"
 - "@ # new comic(s)"
 - "@ overlap # comic(s)""#, row, self_span);

    let input = input.trim();
    if input.starts_with("on ") {
        let input = input["on ".len()..].trim_left();
        let weekday = match parse_weekday(input) {
            Ok((weekday, input)) if input.trim().is_empty() => weekday,
            _ => return Err(error),
        };
        Ok(UpdateSpec::On(weekday))
    } else if input.starts_with("every ") {
        let input = input["every ".len()..].trim_left();
        let (count, input) = match parse_number(input) {
            Ok(pair) => pair,
            Err(()) => return Err(error),
        };
        let input = input.trim();
        if !(input == "day" || input == "days") {
            return Err(error);
        }
        Ok(UpdateSpec::Every(count))
    } else if input.starts_with("overlap ") {
        let input = input["overlap ".len()..].trim_left();
        let (count, input) = match parse_number(input) {
            Ok(pair) => pair,
            Err(()) => return Err(error),
        };
        let input = input.trim();
        if !(input == "comic" || input == "comics") {
            return Err(error);
        }
        Ok(UpdateSpec::Overlap(count))
    } else if input.chars().next().map(|x| x.is_digit(10)).unwrap_or(false) {
        let (count, input) = match parse_number(input) {
            Ok(pair) => pair,
            Err(()) => return Err(error),
        };
        let input = input.trim();
        if !(input == "new comic" || input == "new comics") {
            return Err(error);
        }
        Ok(UpdateSpec::Comics(count))
    } else {
        Err(error)
    }
}

fn parse_number(input: &str) -> Result<(usize, &str), ()> {
    let split_point = input.find(char::is_whitespace).ok_or(())?;
    let (prefix, suffix) = input.split_at(split_point);
    let value = prefix.parse().map_err(|_| ())?;
    Ok((value, suffix))
}

fn parse_weekday(input: &str) -> Result<(Weekday, &str), ()> {
    if input.starts_with("sunday") {
        Ok((Weekday::Sun, &input["sunday".len()..]))
    } else if input.starts_with("monday") {
        Ok((Weekday::Mon, &input["monday".len()..]))
    } else if input.starts_with("tuesday") {
        Ok((Weekday::Tue, &input["tuesday".len()..]))
    } else if input.starts_with("wednesday") {
        Ok((Weekday::Wed, &input["wednesday".len()..]))
    } else if input.starts_with("thursday") {
        Ok((Weekday::Thu, &input["thursday".len()..]))
    } else if input.starts_with("friday") {
        Ok((Weekday::Fri, &input["friday".len()..]))
    } else if input.starts_with("saturday") {
        Ok((Weekday::Sat, &input["saturday".len()..]))
    } else {
        Err(())
    }
}

#[test]
fn test_config_parser() {
    let input = r#"
"Questionable Content" <http://questionablecontent.net/QCRSS.xml> @ on Saturday
"#;
    assert_eq!(
        parse_config(input),
        Ok(vec![
            FeedInfo {
                name: "Questionable Content".into(),
                url: "http://questionablecontent.net/QCRSS.xml".into(),
                updates: HashSet::from_iter(vec![UpdateSpec::On(Weekday::Sat)]),
            },
        ])
    );

    let input = r#"

# Good and cute
"Goodbye To Halos" <http://goodbyetohalos.com/feed/> @ 3 new comics @ on Monday @ overlap 2 comics
# pe'i xamgu
"Electrum" <https://electrum.cubemelon.net/feed> @ On Thursday @ 5 new Comics

"Gunnerkrigg Court" <http://gunnerkrigg.com/rss.xml> @ 4 new comics @ on tuesday

"#;
    assert_eq!(
        parse_config(input),
        Ok(vec![
            FeedInfo {
                name: "Goodbye To Halos".into(),
                url: "http://goodbyetohalos.com/feed/".into(),
                updates: HashSet::from_iter(vec![
                    UpdateSpec::Comics(3),
                    UpdateSpec::On(Weekday::Mon),
                    UpdateSpec::Overlap(2),
                ]),
            },
            FeedInfo {
                name: "Electrum".into(),
                url: "https://electrum.cubemelon.net/feed".into(),
                updates: HashSet::from_iter(vec![
                    UpdateSpec::Comics(5),
                    UpdateSpec::On(Weekday::Thu),
                ]),
            },
            FeedInfo {
                name: "Gunnerkrigg Court".into(),
                url: "http://gunnerkrigg.com/rss.xml".into(),
                updates: HashSet::from_iter(vec![
                    UpdateSpec::Comics(4),
                    UpdateSpec::On(Weekday::Tue),
                ]),
            },
        ])
    )
}

pub fn parse_events(input: &str) -> Result<Vec<FeedEvent>, ParseError> {
    let mut result = Vec::new();
    for (row, line) in input.lines().enumerate() {
        let line = line.trim_right();
        let start_pos = match line.find(not_space) {
            Some(pos) => pos,
            None => continue,
        };

        if line[start_pos..].starts_with("read ") {
            let date = match line[start_pos + "read ".len()..].parse() {
                Ok(date) => date,
                Err(_) => {
                    let span = (start_pos + "read ".len(), line.len());
                    return Err(ParseError::expected("a valid date.", row, span));
                }
            };
            result.push(FeedEvent::Read(date))
        } else if line[start_pos..].starts_with("<") {
            if !line.ends_with(">") {
                return Err(ParseError::expected_char('>', row, line.len()));
            }
            let url = &line[start_pos+1..line.len()-1];
            result.push(FeedEvent::ComicUrl(url.into()));
        } else {
            return Err(ParseError::expected(r#"a feed event. One of:
 - "<url>"
 - "read DATE""#, row, None));
        }
    }
    Ok(result)
}
