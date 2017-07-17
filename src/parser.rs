use std::collections::HashSet;
use std::iter::FromIterator;

use nom::{multispace, space, digit, IResult};

use feed::{FeedInfo, UpdateSpec, Weekday};

#[derive(Clone, Debug)]
pub enum ParseError {
    Unknown,
}

pub fn parse_config(input: &str) -> Result<Vec<FeedInfo>, ParseError> {
    match config(input) {
        IResult::Done("", out) => Ok(out),
        IResult::Done(_, _) => Err(ParseError::Unknown),
        IResult::Error(_) => Err(ParseError::Unknown),
        IResult::Incomplete(_) => Err(ParseError::Unknown),
    }
}

named!(config<&str, Vec<FeedInfo>>,
    do_parse!(
        lines: complete!(many0!(line)) >>
        eof!() >>

        (lines.into_iter().filter_map(|x| x).collect())
    )
);
named!(line<&str, Option<FeedInfo>>,
    alt_complete!(
        multispace => { |_| None } |
        feed_info => { |f| Some(f) } |
        comment => { |_| None }
    )
);

named!(feed_info<&str, FeedInfo>,
    do_parse!(
        name: feed_name >>
        opt!(space) >>
        url: feed_url >>
        opt!(space) >>
        updates: separated_nonempty_list!(space, update_spec) >>

        (FeedInfo {
            name: name.into(),
            url: url.into(),
            updates: HashSet::from_iter(updates),
        })
    )
);

named!(comment<&str, ()>,
    value!((),
        tuple!(
            char!('#'),
            take_until_and_consume_s!("\n")
        )
    )
);

named!(feed_name<&str, &str>, complete!(delimited!(char!('"'), is_not!("\""), char!('"'))));
named!(feed_url<&str, &str>, complete!(delimited!(char!('<'), is_not!(">"), char!('>'))));
named!(number<&str, usize>, complete!(map_res!(digit, |x: &str| x.parse())));

named!(update_spec<&str, UpdateSpec>,
    do_parse!(
        char!('@') >>
        opt!(space) >>
        update: feed_update >>

        (update)
    )
);

named!(feed_update<&str, UpdateSpec>,
    alt_complete!(
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
            tag_no_case_s!("days") >>

            (UpdateSpec::Every(num_days))
        ) |
        do_parse!(
            num_comics: number >>
            space >>
            tag_no_case!("new") >>
            space >>
            tag_no_case!("comics") >>

            (UpdateSpec::Comics(num_comics))
        ) |
        do_parse!(
            tag_no_case_s!("overlap") >>
            space >>
            num_comics: number >>
            space >>
            tag_no_case!("comics") >>

            (UpdateSpec::Overlap(num_comics))
        )
    )
);

named!(weekday<&str, Weekday>,
    alt_complete!(
        tag_no_case_s!("sunday") => { |_| Weekday::Sunday } |
        tag_no_case_s!("monday") => { |_| Weekday::Monday } |
        tag_no_case_s!("tuesday") => { |_| Weekday::Tuesday } |
        tag_no_case_s!("wednesday") => { |_| Weekday::Wednesday } |
        tag_no_case_s!("thursday") => { |_| Weekday::Thursday } |
        tag_no_case_s!("friday") => { |_| Weekday::Friday } |
        tag_no_case_s!("saturday") => { |_| Weekday::Saturday }
    )
);

#[test]
fn test_config_parser() {
    use nom::IResult;

    let input = r#"
"Questionable Content" <http://questionablecontent.net/QCRSS.xml> @ on Saturday
"#;
    assert_eq!(
        config(input),
        IResult::Done(
            "",
            vec![
                FeedInfo {
                    name: "Questionable Content".into(),
                    url: "http://questionablecontent.net/QCRSS.xml".into(),
                    updates: HashSet::from_iter(vec![UpdateSpec::On(Weekday::Saturday)]),
                },
            ],
        )
    );

    let input = r#"

# Good and cute
"Goodbye To Halos" <http://goodbyetohalos.com/feed/> @ 3 new comics @ on Monday @ overlap 2 comics
# pe'i xamgu
"Electrum" <https://electrum.cubemelon.net/feed> @ On Thursday @ 5 new Comics

"Gunnerkrigg Court" <http://gunnerkrigg.com/rss.xml> @ 4 new comics @ on tuesday

"#;
    assert_eq!(
        config(input),
        IResult::Done(
            "",
            vec![
                FeedInfo {
                    name: "Goodbye To Halos".into(),
                    url: "http://goodbyetohalos.com/feed/".into(),
                    updates: HashSet::from_iter(vec![
                        UpdateSpec::Comics(3),
                        UpdateSpec::On(Weekday::Monday),
                        UpdateSpec::Overlap(2),
                    ]),
                },
                FeedInfo {
                    name: "Electrum".into(),
                    url: "https://electrum.cubemelon.net/feed".into(),
                    updates: HashSet::from_iter(vec![
                        UpdateSpec::Comics(5),
                        UpdateSpec::On(Weekday::Thursday),
                    ]),
                },
                FeedInfo {
                    name: "Gunnerkrigg Court".into(),
                    url: "http://gunnerkrigg.com/rss.xml".into(),
                    updates: HashSet::from_iter(vec![
                        UpdateSpec::Comics(4),
                        UpdateSpec::On(Weekday::Tuesday),
                    ]),
                },
            ],
        )
    )
}
