use std::fmt::Display;

#[derive(Debug, PartialEq, Clone)]
pub enum Error {
    NoMatch(String),
    NoCommand(String),
    Invalid,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoMatch(msg) => {
                write!(f, "Message \"{}\" did not match regex expression!", msg)
            },
            Error::NoCommand(msg) => {
                write!(f, "Message \"{}\" is missing command!", msg)
            },
            Error::Invalid => write!(f, "Invalid string!")
        }
    }
}

impl std::error::Error for Error {}
