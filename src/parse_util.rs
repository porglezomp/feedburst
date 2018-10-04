use crate::error::ParseError;

pub type ParseResult<'a, T> = Result<(Buffer<'a>, T), ParseError>;
pub type ParseSuccess<'a> = Result<Buffer<'a>, ParseError>;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Buffer<'a> {
    pub text: &'a str,
    pub row: usize,
    pub col: usize,
}

// Note: These implementations aren't fully general and assume that text is one line only
impl<'a> Buffer<'a> {
    pub fn advance(&self, offset: usize) -> Buffer<'a> {
        let offset = ::std::cmp::min(offset, self.text.len());
        Buffer {
            text: &self.text[offset..],
            row: self.row,
            col: self.col + offset,
        }
    }

    pub fn peek(&self) -> Option<char> {
        self.text.chars().next()
    }

    pub fn trim_left(&self) -> Buffer<'a> {
        match self.text.find(|x: char| !x.is_whitespace()) {
            Some(offset) => self.advance(offset),
            None => self.advance(self.text.len()),
        }
    }

    pub fn trim_right(&self) -> Buffer<'a> {
        Buffer {
            text: self.text.trim_right(),
            ..*self
        }
    }

    pub fn trim(&self) -> Buffer<'a> {
        self.trim_left().trim_right()
    }

    pub fn space(&self) -> ParseSuccess<'a> {
        let new_input = self.trim_left();
        if new_input == *self {
            Err(self.expected("whitespace"))
        } else {
            Ok(new_input)
        }
    }

    pub fn space_or_end(&self) -> ParseSuccess<'a> {
        if self.text.is_empty() {
            Ok(*self)
        } else {
            self.space()
        }
    }

    pub fn token<S: AsRef<str>>(&self, token: S) -> ParseSuccess<'a> {
        let token = token.as_ref();
        if self.starts_with(token) {
            Ok(self.advance(token.len()))
        } else {
            Err(self.expected(format!("\"{}\"", token)))
        }
    }

    pub fn token_no_case<S: AsRef<str>>(&self, token: S) -> ParseSuccess<'a> {
        let token = token.as_ref();
        if self.starts_with_no_case(token) {
            Ok(self.advance(token.len()))
        } else {
            Err(self.expected(format!("\"{}\"", token)))
        }
    }

    #[allow(unused)]
    pub fn first_token_of(&self, tokens: &[&str]) -> ParseResult<'a, &'a str> {
        if tokens.is_empty() {
            return Ok((*self, ""));
        }

        for token in tokens {
            if self.starts_with(token) {
                return Ok((self.advance(token.len()), &self.text[..token.len()]));
            }
        }

        Err(self.first_token_err(tokens))
    }

    pub fn first_token_of_no_case<'b>(&self, tokens: &[&'b str]) -> ParseResult<'a, &'b str> {
        if tokens.is_empty() {
            return Ok((*self, ""));
        }

        for token in tokens {
            if self.starts_with_no_case(token) {
                return Ok((self.advance(token.len()), token));
            }
        }

        Err(self.first_token_err(tokens))
    }

    fn first_token_err(&self, tokens: &[&str]) -> ParseError {
        if tokens.len() == 1 {
            self.expected(format!("\"{}\"", tokens[0]))
        } else if tokens.len() == 2 {
            self.expected(format!("either \"{}\" or \"{}\"", tokens[0], tokens[1]))
        } else {
            let prefix = tokens[..tokens.len() - 1]
                .iter()
                .map(|x| format!("\"{}\"", x))
                .collect::<Vec<_>>()
                .join(", ");
            let last = tokens.last().unwrap();
            self.expected(format!("one of {}, or \"{}\"", prefix, last))
        }
    }

    pub fn starts_with<S: AsRef<str>>(&self, prefix: S) -> bool {
        self.text.starts_with(prefix.as_ref())
    }

    pub fn starts_with_no_case<S: AsRef<str>>(&self, prefix: S) -> bool {
        let prefix = prefix.as_ref();
        if !self.text.is_char_boundary(prefix.len()) {
            return false;
        }

        let beginning = self.text[..prefix.len()].to_lowercase();
        beginning == prefix.to_lowercase()
    }

    pub fn read_between(&self, begin: char, end: char) -> ParseResult<'a, &'a str> {
        if !self.text.starts_with(begin) {
            return Err(self.expected(format!("character '{}'", begin)));
        }

        let input = self.advance(begin.len_utf8());
        if let Some(offset) = input.text.find(end) {
            Ok((
                input.advance(offset + end.len_utf8()),
                &input.text[..offset],
            ))
        } else {
            let span = (self.col, self.col + self.text.len());
            Err(ParseError::expected(
                format!("closing '{}'", end),
                self.row,
                span,
            ))
        }
    }

    pub fn expected<S: Into<String>>(&self, message: S) -> ParseError {
        ParseError::expected(message, self.row, self.col)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_advance() {
        let input = Buffer {
            row: 1,
            col: 0,
            text: "Hello",
        };

        assert_eq!(
            input.advance(1),
            Buffer {
                row: 1,
                col: 1,
                text: "ello",
            }
        );
        assert_eq!(input.advance(0), input);
        assert_eq!(
            input.advance(5),
            Buffer {
                row: 1,
                col: 5,
                text: "",
            }
        );
        assert_eq!(
            input.advance(10),
            Buffer {
                row: 1,
                col: 5,
                text: "",
            }
        );
    }

    #[test]
    fn test_trim() {
        let input = Buffer {
            row: 1,
            col: 2,
            text: "  Hello  ",
        };

        assert_eq!(
            input.trim_left(),
            Buffer {
                row: 1,
                col: 4,
                text: "Hello  ",
            }
        );
        assert_eq!(
            input.trim_right(),
            Buffer {
                row: 1,
                col: 2,
                text: "  Hello",
            }
        );
        assert_eq!(
            input.trim(),
            Buffer {
                row: 1,
                col: 4,
                text: "Hello",
            }
        );

        // Idempotent
        assert_eq!(input.trim_left(), input.trim_left().trim_left());
        assert_eq!(input.trim_right(), input.trim_right().trim_right());
        assert_eq!(input.trim(), input.trim().trim());
    }

    #[test]
    fn test_space() {
        let good_input = Buffer {
            row: 0,
            col: 0,
            text: "  Consume the space",
        };

        assert_eq!(
            good_input.space(),
            Ok(Buffer {
                row: 0,
                col: 2,
                text: "Consume the space",
            })
        );

        let bad_input = Buffer {
            row: 0,
            col: 0,
            text: "No space here",
        };

        assert!(bad_input.space().is_err());
        assert!(bad_input.space_or_end().is_err());

        let is_end = Buffer {
            row: 7,
            col: 42,
            text: "",
        };

        assert!(is_end.space().is_err());
        assert_eq!(is_end.space_or_end(), Ok(is_end));
    }

    #[test]
    fn test_token() {
        let input = Buffer {
            row: 0,
            col: 0,
            text: "Token",
        };
        let empty = Buffer {
            row: 0,
            col: 5,
            text: "",
        };
        let en_input = Buffer {
            row: 0,
            col: 3,
            text: "en",
        };

        assert_eq!(input.token("Token"), Ok(empty));
        assert!(input.token("token").is_err());
        assert_eq!(input.token("Tok"), Ok(en_input));

        assert_eq!(input.token_no_case("Token"), Ok(empty));
        assert_eq!(input.token_no_case("token"), Ok(empty));
        assert_eq!(input.token_no_case("TOKEN"), Ok(empty));
        assert_eq!(input.token_no_case("tOKeN"), Ok(empty));
        assert_eq!(input.token_no_case("Tok"), Ok(en_input));
        assert_eq!(input.token_no_case("tok"), Ok(en_input));
    }

    #[test]
    fn test_first_token() {
        let input = Buffer {
            row: 0,
            col: 0,
            text: "Tokens",
        };
        let empty = Buffer {
            row: 0,
            col: 6,
            text: "",
        };
        let s_input = Buffer {
            row: 0,
            col: 5,
            text: "s",
        };

        assert_eq!(
            input.first_token_of(&["Tokens", "Token"]),
            Ok((empty, "Tokens"))
        );
        assert_eq!(
            input.first_token_of(&["Token", "Tokens"]),
            Ok((s_input, "Token"))
        );
        assert_eq!(input.first_token_of(&[]), Ok((input, "")));

        // Error messages should be correct
        assert_eq!(
            input.first_token_of(&["Meow"]).unwrap_err(),
            input.expected("\"Meow\"")
        );
        assert_eq!(
            input.first_token_of(&["Meow", "Bark"]).unwrap_err(),
            input.expected("either \"Meow\" or \"Bark\"")
        );
        assert_eq!(
            input.first_token_of(&["Meow", "Bark", "Moo"]).unwrap_err(),
            input.expected("one of \"Meow\", \"Bark\", or \"Moo\"")
        );

        assert_eq!(
            input.first_token_of_no_case(&["TOKENS", "token"]),
            Ok((empty, "TOKENS"))
        );
        assert_eq!(
            input.first_token_of_no_case(&["Token", "tOkEns"]),
            Ok((s_input, "Token"))
        );
        assert_eq!(input.first_token_of_no_case(&[]), Ok((input, "")));
        assert!(input.first_token_of_no_case(&["ZOOP"]).is_err());
    }

    #[test]
    fn test_starts_with() {
        let input = Buffer {
            row: 0,
            col: 0,
            text: "Starts with",
        };

        assert!(input.starts_with("Starts"));
        assert!(!input.starts_with("starts"));
        assert!(input.starts_with("Starts with"));
        assert!(input.starts_with("Star"));
        assert!(!input.starts_with("X"));

        assert!(input.starts_with_no_case("Starts"));
        assert!(input.starts_with_no_case("starts"));
        assert!(input.starts_with_no_case("STARTS WITH"));
        assert!(!input.starts_with_no_case("X"));
    }

    #[test]
    fn test_starts_with_multibyte() {
        const HEART: &'static str = "\u{1F49C}";
        let heart_emoji = Buffer {
            row: 0,
            col: 0,
            text: HEART,
        };

        assert!(heart_emoji.starts_with(HEART));
        assert!(heart_emoji.starts_with_no_case(HEART));
        assert!(!heart_emoji.starts_with("x"));
        assert!(!heart_emoji.starts_with_no_case("x"));

        let heart = Buffer {
            row: 0,
            col: 0,
            text: "heart",
        };

        assert!(!heart.starts_with(HEART));
        assert!(!heart.starts_with_no_case(HEART));
    }

    #[test]
    fn test_read_between() {
        let input = Buffer {
            row: 0,
            col: 0,
            text: "<Hello>",
        };

        assert_eq!(
            input.read_between('<', '>'),
            Ok((
                Buffer {
                    row: 0,
                    col: 7,
                    text: "",
                },
                "Hello",
            ))
        );
        assert!(input.read_between('<', '!').is_err());
        assert!(input.read_between('!', '>').is_err());

        let input = Buffer {
            row: 0,
            col: 0,
            text: "\"Hello\"",
        };

        assert_eq!(
            input.read_between('"', '"'),
            Ok((
                Buffer {
                    row: 0,
                    col: 7,
                    text: "",
                },
                "Hello",
            ))
        );
    }
}
