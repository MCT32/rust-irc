use std::{error::Error, fmt};

use super::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    Raw(u16, Vec<String>),
}

impl fmt::Display for Reply {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reply::Raw(code, params) => {
                if params.is_empty() {
                    return write!(f, "{}", code);
                }

                write!(f, "{} {}", code, params.join(" "))
            }
            _ => Err(fmt::Error),
        }
    }
}

impl Reply {
    pub fn raw(&self) -> Self {
        match self {
            Reply::Raw(_, _) => self.clone()
        }
    }

    pub fn raw_command(self) -> Command {
        let reply = self.raw();

        match reply {
            Reply::Raw(code, params) => Command::Raw(code.to_string(), params),
            _ => panic!()
        }
    }
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
                if params.is_empty() {
                    return write!(f, "{}", code);
                }

                write!(f, "{} {}", code, params.join(" "))
            }
            _ => Err(fmt::Error),
        }
    }
}

impl ErrorReply {
    pub fn raw(&self) -> Self {
        match self {
            ErrorReply::Raw(_, _) => self.clone()
        }
    }

    pub fn raw_command(self) -> Command {
        let reply = self.raw();

        match reply {
            ErrorReply::Raw(code, params) => Command::Raw(code.to_string(), params),
            _ => panic!()
        }
    }
}
