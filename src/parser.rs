use std::collections::HashSet;
use std::iter::FromIterator;

use nom::{space, digit, IResult};
use chrono::Weekday;

use feed::{FeedInfo, UpdateSpec};
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
    let start_col = match input[col..].find('@') {
        Some(off) => col + off,
        None => return Ok(out),
    };

    let mut col = start_col;
    for policy_chunk in input[start_col+1..].split('@') {
        out.push(parse_policy(row, col, policy_chunk)?);
        col += 1 + policy_chunk.len();
    }

    Ok(out)
}

fn parse_policy(row: usize, col: usize, input: &str) -> Result<UpdateSpec, ParseError> {
    let self_span = (col, col + input.len());
    match feed_update(input) {
        IResult::Done("", policy) => Ok(policy),
        _ => Err(ParseError::expected(r#"a policy definition. One of:
 - "@ on <weekday>"
 - "@ every # day(s)"
 - "@ # new comic(s)"
 - "@ overlap # comic(s)""#, row, self_span)),
    }
}

named!(number<&str, usize>, complete!(map_res!(digit, |x: &str| x.parse())));

named!(feed_update<&str, UpdateSpec>,
    do_parse!(
        opt!(complete!(space)) >>
        res: alt_complete!(
            do_parse!(
                tag_no_case_s!("on") >>
                space >>
                weekday: weekday >>

                (UpdateSpec::On(weekday))
            ) |
            do_parse!(
                tag_no_case_s!("every") >>
                space >>
                num_days: number >>
                space >>
                tag_no_case_s!("day") >>
                opt!(complete!(char!('s'))) >>

                (UpdateSpec::Every(num_days))
            ) |
            do_parse!(
                num_comics: number >>
                space >>
                tag_no_case!("new") >>
                space >>
                tag_no_case!("comic") >>
                opt!(complete!(char!('s'))) >>

                (UpdateSpec::Comics(num_comics))
            ) |
            do_parse!(
                tag_no_case_s!("overlap") >>
                space >>
                num_comics: number >>
                space >>
                tag_no_case!("comic") >>
                opt!(complete!(char!('s'))) >>

                (UpdateSpec::Overlap(num_comics))
            )
        ) >>
        opt!(complete!(space)) >>
        (res)
    )
);

named!(weekday<&str, Weekday>,
    alt_complete!(
        tag_no_case_s!("sunday") => { |_| Weekday::Sun } |
        tag_no_case_s!("monday") => { |_| Weekday::Mon } |
        tag_no_case_s!("tuesday") => { |_| Weekday::Tue } |
        tag_no_case_s!("wednesday") => { |_| Weekday::Wed } |
        tag_no_case_s!("thursday") => { |_| Weekday::Thu } |
        tag_no_case_s!("friday") => { |_| Weekday::Fri } |
        tag_no_case_s!("saturday") => { |_| Weekday::Sat }
    )
);

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
