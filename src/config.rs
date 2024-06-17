use std::net::{SocketAddr, ToSocketAddrs};

use crate::error::IrcConfigBuilderError;

pub struct IrcConfig {
    server_address: SocketAddr,
}

pub struct IrcConfigBuilder {
    server_address: Option<SocketAddr>,
}

impl IrcConfigBuilder {
    pub fn new() -> Self {
        IrcConfigBuilder {
            server_address: None,
        }
    }

    pub fn build(self) -> Result<IrcConfig, IrcConfigBuilderError> {
        let server_address = match self.server_address {
            Some(server_address) => server_address,
            None => return Err(IrcConfigBuilderError::ServerAddressMissing),
        };

        Ok(IrcConfig {
            server_address,
        })
    }

    pub fn server_address<T: ToSocketAddrs>(&mut self, server_address: T) -> Result<(), std::io::Error> {
        self.server_address = Some(server_address.to_socket_addrs()?.next().unwrap());
        Ok(())
    }
}