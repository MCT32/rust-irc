use tokio::{net::TcpStream, sync::Mutex};
use std::{net::SocketAddr, sync::Arc};

use crate::{error::{IrcConfigBuilderError, IrcConnectError}, messages::Message, users::User, IrcConnection};

type RawMessageHandler = fn(&str);
type MessageHandler = fn(Message);

#[derive(Clone)]
pub struct IrcConfig {
    host: SocketAddr,

    pub user: User,

    pub password: Option<String>,

    pub raw_receive_handler: Option<RawMessageHandler>,
    pub receive_handler: Option<MessageHandler>,
}

impl IrcConfig {
    pub fn builder() -> IrcConfigBuilder {
        IrcConfigBuilder::default()
    }

    pub async fn connect(&self) -> Result<IrcConnection, IrcConnectError> {
        match TcpStream::connect(self.host).await {
            Ok(stream) => {
                let mut connection = IrcConnection {
                    stream: Arc::new(Mutex::new(stream)),
                    config: self.clone(),
                };

                match connection.init().await {
                    Ok(_) => Ok(connection),
                    Err(err) => Err(IrcConnectError::IrcInitError(err))
                }
            }
            Err(err) => Err(IrcConnectError::TcpConnectionError(err))
        }
    }
}

#[derive(Default)]
pub struct IrcConfigBuilder {
    // Required, no default
    user: Option<User>,

    password: Option<String>,

    raw_receive_handler: Option<RawMessageHandler>,
    receive_handler: Option<MessageHandler>,
}

impl IrcConfigBuilder {
    pub fn new() -> IrcConfigBuilder {
        IrcConfigBuilder {
            user: None,

            password: None,

            raw_receive_handler: None,
            receive_handler: None,
        }
    }

    pub fn user(mut self, user: User) -> IrcConfigBuilder {
        self.user = Some(user);
        self
    }

    pub fn password(mut self, password: String) -> IrcConfigBuilder {
        self.password = Some(password);
        self
    }

    pub fn set_receive_handler(mut self, handler: MessageHandler) -> IrcConfigBuilder {
        self.receive_handler = Some(handler);
        self
    }

    pub fn set_raw_receive_handler(mut self, handler: RawMessageHandler) -> IrcConfigBuilder {
        self.raw_receive_handler = Some(handler);
        self
    }

    pub fn host(self, host: SocketAddr) -> Result<IrcConfig, IrcConfigBuilderError> {
        if self.user.is_none() {
            return Err(IrcConfigBuilderError);
        }
        
        Ok(IrcConfig {
            user: self.user.unwrap(),

            password: self.password,

            raw_receive_handler: self.raw_receive_handler,
            receive_handler: self.receive_handler,

            host,
        })
    }
}