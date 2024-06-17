use std::net::{SocketAddr, ToSocketAddrs};

use crate::error::IrcConfigBuilderError;

pub struct IrcConfig {
    server_address: SocketAddr,
}

pub struct IrcConfigBuilder {
    server_address: Option<SocketAddr>,
}

impl IrcConfigBuilder {
    fn new() -> Self {
        IrcConfigBuilder {
            server_address: None,
        }
    }

    fn build(self) -> Result<IrcConfig, IrcConfigBuilderError> {
        let server_address = match self.server_address {
            Some(server_address) => server_address,
            None => return Err(IrcConfigBuilderError::ServerAddressMissing),
        };

        Ok(IrcConfig {
            server_address,
        })
    }
}