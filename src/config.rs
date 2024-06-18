use std::net::{SocketAddr, ToSocketAddrs};

use crate::error::IrcConfigBuilderError;

#[derive(Debug, PartialEq, Clone)]
pub struct IrcConfig {
    pub server_address: SocketAddr,
    pub username: String,       
    pub nickname: String,     
    pub password: Option<String>, 
}

#[derive(Debug, PartialEq, Clone)]
pub struct IrcConfigBuilder { 
    pub server_address: Option<SocketAddr>,
    pub username: Option<String>,       
    pub nickname: Option<String>,      
    pub password: Option<String>, 
}

impl IrcConfigBuilder { 
    pub fn new() -> Self {
        IrcConfigBuilder {
            server_address: None,
            username: None,
            nickname: None,
            password: None,
        }
    }
    
    pub fn username(&mut self, username: String) { 
        self.username = Some(username);
    }
    
    pub fn nickname(&mut self, nickname: String) { 
        self.nickname = Some(nickname);
    }

    pub fn password(&mut self, password: Option<String>) { 
        self.password = password;
    }

    pub fn build(self) -> Result<IrcConfig, IrcConfigBuilderError> {
        let server_address = match self.server_address {
            Some(server_address) => server_address,
            None => return Err(IrcConfigBuilderError::ServerAddressMissing),
        };

        let username = match self.username {
            Some(username) => username,
            None => return Err(IrcConfigBuilderError::UsernameMissing),
        };

        let nickname = self.nickname.unwrap_or(username.clone()); 

        let password = self.password;

        Ok(IrcConfig {
            server_address,
            username,
            nickname,
            password,
        })
    }
    pub fn server_address<T: ToSocketAddrs>(
        &mut self,
        server_address: T,
    ) -> Result<(), std::io::Error> {
        self.server_address = Some(server_address.to_socket_addrs()?.next().unwrap());
        Ok(())
    }
}