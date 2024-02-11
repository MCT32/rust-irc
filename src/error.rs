use std::{error::Error, fmt::{self, Debug}};

use tokio::io;


#[derive(Debug)]
pub enum IrcConnectError {
    UserInfoMissing,
    TcpConnectionError(io::Error),
    IrcInitError(IrcInitError),
}

impl Error for IrcConnectError {}
impl fmt::Display for IrcConnectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::UserInfoMissing => "missing user info".to_string(),
            Self::TcpConnectionError(err) => format!("tcp connection error: {:#?}", err),
            Self::IrcInitError(err) => format!("irc init error: {:#?}", err)
        };

        write!(f, "{}", msg)
    }
}

#[derive(Debug)]
pub enum IrcSendError {
    TcpSendError(io::Error),
}

impl Error for IrcSendError {}
impl fmt::Display for IrcSendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::TcpSendError(err) => format!("tcp error: {:#?}", err)
        };

        write!(f, "{}", msg)
    }
}

#[derive(Debug)]
pub enum IrcInitError {
    TcpConnectionError(io::Error),
    IrcSendError(IrcSendError),
}

impl Error for IrcInitError {}
impl fmt::Display for IrcInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::TcpConnectionError(err) => format!("tcp error: {:#?}", err),
            Self::IrcSendError(err) => format!("irc send error: {:#?}", err)
        };

        write!(f, "{}", msg)
    }
}