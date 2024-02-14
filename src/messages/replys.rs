use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    Raw(u16, Vec<String>),
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorReply {
    Raw(u16, Vec<String>),
}
impl Error for ErrorReply {}

impl fmt::Display for ErrorReply {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorReply::Raw(code, params) => {
                write!(f, "{} {}", code, params.join(" "))
            }
        }
    }
}
