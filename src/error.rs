use reqwest;
use std::{fmt, io};

#[derive(Debug)]
pub enum Error {
    Msg(String),
    Io(io::Error),
    Request(reqwest::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref err) => write!(fmt, "Error performing IO: {}", err),
            Error::Msg(ref err) => write!(fmt, "{}", err),
            Error::Request(ref err) => write!(fmt, "Error making request: {}", err),
        }
    }
}

pub type Span = Option<(usize, usize)>;

#[derive(Clone, Debug, PartialEq)]
pub enum ParseError {
    Expected { msg: String, row: usize, span: Span },
}

impl ParseError {
    pub fn expected<St: Into<String>, Sp: IntoSpan>(msg: St, row: usize, span: Sp) -> Self {
        ParseError::Expected {
            msg: msg.into(),
            row,
            span: span.into_span(),
        }
    }
}

pub trait IntoSpan {
    fn into_span(self) -> Span;
}

impl IntoSpan for usize {
    fn into_span(self) -> Span {
        Some((self, self))
    }
}

impl IntoSpan for (usize, usize) {
    fn into_span(self) -> Span {
        Some(self)
    }
}

impl IntoSpan for Option<()> {
    fn into_span(self) -> Span {
        None
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Error {
        Error::Request(err)
    }
}
