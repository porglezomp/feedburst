use error::ParseError;

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
        Buffer {
            text: &self.text[offset..],
            row: self.row,
            col: self.col + offset,
        }
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
    pub fn first_token_of(&self, tokens: &[&str]) -> ParseSuccess<'a> {
        if tokens.is_empty() {
            return Ok(*self);
        }

        for token in tokens {
            if self.starts_with(token) {
                return Ok(self.advance(token.len()));
            }
        }

        Err(self.first_token_err(tokens))
    }

    pub fn first_token_of_no_case(&self, tokens: &[&str]) -> ParseSuccess<'a> {
        if tokens.is_empty() {
            return Ok(*self);
        }

        for token in tokens {
            if self.starts_with_no_case(token) {
                return Ok(self.advance(token.len()));
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
            let prefix = tokens[..tokens.len() - 2]
                .iter()
                .map(|x| format!("\"{}\"", x))
                .collect::<Vec<_>>()
                .join(", ");
            let last = tokens.last().unwrap();
            self.expected(format!("one of: {}, or \"{}\"", prefix, last))
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
        )
    }
}
