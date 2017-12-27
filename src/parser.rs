use std::collections::HashSet;
use std::iter::FromIterator;

use chrono::Weekday;

use feed::{FeedEvent, FeedInfo, UpdateSpec};
use error::ParseError;
use parse_util::{Buffer, ParseResult};

pub fn parse_config(input: &str) -> Result<Vec<FeedInfo>, ParseError> {
    let mut out = Vec::new();
    let mut root_path = None;
    for (row, line) in input.lines().enumerate() {
        let buf = Buffer {
            row: row + 1,
            col: 0,
            text: line,
        }.trim();

        if buf.starts_with("#") || buf.text.is_empty() {
            continue;
        }

        if buf.starts_with("root") {
            let buf = buf.token_no_case("root")?;
            if buf.trim().text.is_empty() {
                root_path = None;
            } else {
                root_path = Some(buf.space()?.trim().text);
            }
        } else {
            let (_, mut feed) = parse_line(&buf)?;
            feed.root = root_path.map(From::from);
            out.push(feed);
        }
    }
    Ok(out)
}

fn parse_line<'a>(buf: &Buffer<'a>) -> ParseResult<'a, FeedInfo> {
    let (buf, name) = parse_name(buf)?;
    let buf = buf.trim_left();
    let (buf, url) = parse_url(&buf)?;
    let buf = buf.trim_left();
    let (buf, policies) = parse_policies(&buf)?;
    Ok((
        buf,
        FeedInfo {
            name: name.into(),
            url: url.into(),
            updates: HashSet::from_iter(policies),
            root: None,
        },
    ))
}

fn parse_name<'a>(buf: &Buffer<'a>) -> ParseResult<'a, &'a str> {
    buf.trim_left().read_between('"', '"')
}

fn parse_url<'a>(buf: &Buffer<'a>) -> ParseResult<'a, &'a str> {
    buf.trim_left().read_between('<', '>')
}

fn parse_policies<'a>(buf: &Buffer<'a>) -> ParseResult<'a, Vec<UpdateSpec>> {
    let mut policies = Vec::new();
    let mut buf = buf.trim_left();
    while buf.starts_with("@") {
        let (inp, policy) = parse_policy(&buf)?;
        policies.push(policy);
        buf = inp.trim_left();
    }
    Ok((buf, policies))
}

fn parse_policy<'a>(buf: &Buffer<'a>) -> Result<(Buffer<'a>, UpdateSpec), ParseError> {
    let buf = buf.trim_left().token("@")?.space()?;

    if buf.starts_with_no_case("on") {
        let buf = buf.token_no_case("on")?.space()?;
        let (buf, weekday) = parse_weekday(&buf)?;
        let buf = buf.space_or_end()?;
        Ok((buf, UpdateSpec::On(weekday)))
    } else if buf.starts_with_no_case("every") {
        let buf = buf.token_no_case("every")?.space()?;
        let (buf, count) = parse_number(&buf)?;
        let buf = buf.space()?
            .first_token_of_no_case(&["days", "day"])?
            .space_or_end()?;
        Ok((buf, UpdateSpec::Every(count)))
    } else if buf.starts_with_no_case("overlap") {
        let buf = buf.token_no_case("overlap")?.space()?;
        let (buf, count) = parse_number(&buf)?;
        let buf = buf.space()?
            .first_token_of_no_case(&["comics", "comic"])?
            .space_or_end()?;
        Ok((buf, UpdateSpec::Overlap(count)))
    } else if buf.text
        .chars()
        .next()
        .map(|x| x.is_digit(10))
        .unwrap_or_default()
    {
        let (buf, count) = parse_number(&buf)?;
        let buf = buf.trim_left()
            .token_no_case("new")?
            .space()?
            .first_token_of_no_case(&["comics", "comic"])?;
        Ok((buf, UpdateSpec::Comics(count)))
    } else {
        let error = ParseError::expected(
            r#"a policy definition. One of:
 - "@ on WEEKDAY"
 - "@ every # day(s)"
 - "@ # new comic(s)"
 - "@ overlap # comic(s)""#,
            buf.row,
            (buf.col, buf.col + buf.text.len()),
        );
        Err(error)
    }
}

fn parse_number<'a>(buf: &Buffer<'a>) -> ParseResult<'a, usize> {
    let buf = buf.trim_left();
    let end = buf.text
        .find(|c: char| !c.is_digit(10))
        .unwrap_or_else(|| buf.text.len());
    if end == 0 {
        return Err(buf.expected("digit"));
    }
    let value = buf.text[..end].parse().expect("Should only contain digits");
    let buf = buf.advance(end);
    Ok((buf, value))
}

fn parse_weekday<'a>(buf: &Buffer<'a>) -> ParseResult<'a, Weekday> {
    if buf.starts_with_no_case("sunday") {
        let buf = buf.advance("sunday".len());
        Ok((buf, Weekday::Sun))
    } else if buf.starts_with_no_case("monday") {
        let buf = buf.advance("monday".len());
        Ok((buf, Weekday::Mon))
    } else if buf.starts_with_no_case("tuesday") {
        let buf = buf.advance("tuesday".len());
        Ok((buf, Weekday::Tue))
    } else if buf.starts_with_no_case("wednesday") {
        let buf = buf.advance("wednesday".len());
        Ok((buf, Weekday::Wed))
    } else if buf.starts_with_no_case("thursday") {
        let buf = buf.advance("thursday".len());
        Ok((buf, Weekday::Thu))
    } else if buf.starts_with_no_case("friday") {
        let buf = buf.advance("friday".len());
        Ok((buf, Weekday::Fri))
    } else if buf.starts_with_no_case("saturday") {
        let buf = buf.advance("saturday".len());
        Ok((buf, Weekday::Sat))
    } else {
        Err(buf.expected("a weekday"))
    }
}

pub fn parse_events(input: &str) -> Result<Vec<FeedEvent>, ParseError> {
    let mut result = Vec::new();
    for (row, line) in input.lines().enumerate() {
        let line = Buffer {
            row: row + 1,
            col: 0,
            text: line,
        }.trim();
        if line.text.is_empty() {
            continue;
        }

        if line.starts_with_no_case("read") {
            let line = line.token_no_case("read")?.space()?;
            let date = match line.text.parse() {
                Ok(date) => date,
                Err(_) => {
                    return Err(line.expected("a valid date"));
                }
            };
            result.push(FeedEvent::Read(date))
        } else if line.starts_with("<") {
            let (line, url) = line.read_between('<', '>')?;
            line.space_or_end()?;
            result.push(FeedEvent::ComicUrl(url.into()));
        } else {
            return Err(ParseError::expected(
                r#"a feed event. One of:
 - "<url>"
 - "read DATE""#,
                row,
                None,
            ));
        }
    }
    Ok(result)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_config_parser() {
        let buf = r#"
"Questionable Content" <http://questionablecontent.net/QCRSS.xml> @ on Saturday @ every 10 days
"#;
        assert_eq!(
            parse_config(buf),
            Ok(vec![
                FeedInfo {
                    name: "Questionable Content".into(),
                    url: "http://questionablecontent.net/QCRSS.xml".into(),
                    updates: HashSet::from_iter(vec![
                        UpdateSpec::On(Weekday::Sat),
                        UpdateSpec::Every(10),
                    ]),
                    root: None,
                },
            ])
        );
    }

    #[test]
    fn test_multi_feeds() {
        let buf = r#"

# Good and cute
"Goodbye To Halos" <http://goodbyetohalos.com/feed/> @ 3 new comics @ on Monday @ overlap 2 comics
# pe'i xamgu
"Electrum" <https://electrum.cubemelon.net/feed> @ On Thursday @ 5 new Comics

"Gunnerkrigg Court" <http://gunnerkrigg.com/rss.xml> @ 4 new comics @ on tuesday

"#;
        assert_eq!(
            parse_config(buf),
            Ok(vec![
                FeedInfo {
                    name: "Goodbye To Halos".into(),
                    url: "http://goodbyetohalos.com/feed/".into(),
                    updates: HashSet::from_iter(vec![
                        UpdateSpec::Comics(3),
                        UpdateSpec::On(Weekday::Mon),
                        UpdateSpec::Overlap(2),
                    ]),
                    root: None,
                },
                FeedInfo {
                    name: "Electrum".into(),
                    url: "https://electrum.cubemelon.net/feed".into(),
                    updates: HashSet::from_iter(vec![
                        UpdateSpec::Comics(5),
                        UpdateSpec::On(Weekday::Thu),
                    ]),
                    root: None,
                },
                FeedInfo {
                    name: "Gunnerkrigg Court".into(),
                    url: "http://gunnerkrigg.com/rss.xml".into(),
                    updates: HashSet::from_iter(vec![
                        UpdateSpec::Comics(4),
                        UpdateSpec::On(Weekday::Tue),
                    ]),
                    root: None,
                },
            ])
        )
    }

    #[test]
    fn test_feed_root() {
        let buf = concat!(
            r#"

"Eth's Skin" <http://www.eths-skin.com/rss>

root /hello/world
"Witchy" <http://feeds.feedburner.com/WitchyComic?format=xml> @ on Wednesday
"Cucumber Quest" <http://cucumber.gigidigi.com/feed/> @ on Sunday
root /oops/this/is/another/path
"Imogen Quest" <http://imogenquest.net/?feed=rss2> @ on Friday
root
root "#,
            r#"

"Balderdash" <http://www.balderdashcomic.com/rss.php>
"#
        );

        assert_eq!(
            parse_config(buf),
            Ok(vec![
                FeedInfo {
                    name: "Eth's Skin".into(),
                    url: "http://www.eths-skin.com/rss".into(),
                    updates: HashSet::new(),
                    root: None,
                },
                FeedInfo {
                    name: "Witchy".into(),
                    url: "http://feeds.feedburner.com/WitchyComic?format=xml".into(),
                    updates: HashSet::from_iter(vec![UpdateSpec::On(Weekday::Wed)]),
                    root: Some("/hello/world".into()),
                },
                FeedInfo {
                    name: "Cucumber Quest".into(),
                    url: "http://cucumber.gigidigi.com/feed/".into(),
                    updates: HashSet::from_iter(vec![UpdateSpec::On(Weekday::Sun)]),
                    root: Some("/hello/world".into()),
                },
                FeedInfo {
                    name: "Imogen Quest".into(),
                    url: "http://imogenquest.net/?feed=rss2".into(),
                    updates: HashSet::from_iter(vec![UpdateSpec::On(Weekday::Fri)]),
                    root: Some("/oops/this/is/another/path".into()),
                },
                FeedInfo {
                    name: "Balderdash".into(),
                    url: "http://www.balderdashcomic.com/rss.php".into(),
                    updates: HashSet::new(),
                    root: None,
                },
            ])
        )
    }

    #[test]
    fn test_invalid_configs() {
        let bad_weekday = r#"
"Boozle" <http://boozle.sgoetter.com/feed/> @ on wendsday
"#;
        assert_eq!(
            parse_config(bad_weekday),
            Err(ParseError::expected("a weekday", 2, 49))
        );

        let bad_policy = r#"
"Boozle" <http://boozle.sgoetter.com/feed/> @ foo
"#;

        let ParseError::Expected { msg, row, .. } = parse_config(bad_policy).unwrap_err();
        assert!(msg.starts_with("a policy definition"));
        assert_eq!(row, 2);
    }

    #[test]
    fn test_parse_events() {
        use chrono::{TimeZone, Utc};
        let input = r#"
<http://www.goodbyetohalos.com/comic/01137>

<http://www.goodbyetohalos.com/comic/01138-139>
 read 2017-07-17T03:21:21.492180+00:00
 <http://www.goodbyetohalos.com/comic/01140>
read 2017-07-18T23:41:58.130248+00:00
"#;
        assert_eq!(
            parse_events(input),
            Ok(vec![
                FeedEvent::ComicUrl("http://www.goodbyetohalos.com/comic/01137".into()),
                FeedEvent::ComicUrl("http://www.goodbyetohalos.com/comic/01138-139".into()),
                FeedEvent::Read(Utc.ymd(2017, 07, 17).and_hms_micro(03, 21, 21, 492180)),
                FeedEvent::ComicUrl("http://www.goodbyetohalos.com/comic/01140".into()),
                FeedEvent::Read(Utc.ymd(2017, 07, 18).and_hms_micro(23, 41, 58, 130248)),
            ])
        );

        assert!(parse_events("invalid").is_err());
    }
}
