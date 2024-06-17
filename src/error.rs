use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum IrcConfigBuilderError {
    ServerAddressMissing,
    UsernameMissing,
}

impl Display for IrcConfigBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IrcConfigBuilderError::ServerAddressMissing => write!(f, "Config builder is missing a server address! Set it using `config_builder.server_address(...)`"),
            IrcConfigBuilderError::UsernameMissing => write!(f, "Config builder is missing a username! Set it using `config_builder.username(...)`"),
        }
    }
}

impl Error for IrcConfigBuilderError {}
