use std::net::{SocketAddr, ToSocketAddrs};

use crate::error::IrcConfigBuilderError;

pub struct IrcConfig<'a> {
    pub server_address: SocketAddr,
    pub username: &'a str,
    pub nickname: &'a str,
    pub password: Option<&'a str>,
}

pub struct IrcConfigBuilder<'a> {
    server_address: Option<SocketAddr>,
    username: Option<&'a str>,
    nickname: Option<&'a str>,
    password: Option<&'a str>,
}

impl<'a> IrcConfigBuilder<'a> {
    pub fn new() -> Self {
        IrcConfigBuilder {
            server_address: None,
            username: None,
            nickname: None,
            password: None,
        }
    }

    pub fn build(self) -> Result<IrcConfig<'a>, IrcConfigBuilderError> {
        let server_address = match self.server_address {
            Some(server_address) => server_address,
            None => return Err(IrcConfigBuilderError::ServerAddressMissing),
        };

        let username = match self.username {
            Some(username) => username,
            None => return Err(IrcConfigBuilderError::UsernameMissing),
        };

        let nickname = match self.nickname {
            Some(nickname) => nickname,
            None => username,
        };

        let password = self.password;

        Ok(IrcConfig {
            server_address,
            username,
            nickname,
            password,
        })
    }

    pub fn server_address<T: ToSocketAddrs>(&mut self, server_address: T) -> Result<(), std::io::Error> {
        self.server_address = Some(server_address.to_socket_addrs()?.next().unwrap());
        Ok(())
    }
}