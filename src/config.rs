use tokio::{net::TcpStream, sync::Mutex};
use std::{net::SocketAddr, sync::Arc};

type RawMessageHandler = fn(&str);
type MessageHandler = fn(super::messages::Message);

#[derive(Clone)]
pub struct IrcConfig {
    host: SocketAddr,

    pub nickname: String,
    pub username: String,
    pub hostname: String,
    pub servername: String,
    pub realname: String,

    pub password: Option<String>,

    pub raw_receive_handler: Option<RawMessageHandler>,
    pub receive_handler: Option<MessageHandler>,
}

impl IrcConfig {
    pub fn builder() -> IrcConfigBuilder {
        IrcConfigBuilder::default()
    }

    pub async fn connect(&self) -> Result<super::IrcConnection, super::error::IrcConnectError> {
        match TcpStream::connect(self.host).await {
            Ok(stream) => {
                let mut connection = super::IrcConnection {
                    stream: Arc::new(Mutex::new(stream)),
                    config: self.clone(),
                };

                match connection.init().await {
                    Ok(_) => Ok(connection),
                    Err(err) => Err(super::error::IrcConnectError::IrcInitError(err))
                }
            }
            Err(err) => Err(super::error::IrcConnectError::TcpConnectionError(err))
        }
    }
}

#[derive(Default)]
pub struct IrcConfigBuilder {
    // Required, no default
    nickname: Option<String>,
    username: Option<String>,
    hostname: Option<String>,
    servername: Option<String>,
    realname: Option<String>,

    password: Option<String>,

    raw_receive_handler: Option<RawMessageHandler>,
    receive_handler: Option<MessageHandler>,
}

impl IrcConfigBuilder {
    pub fn new() -> IrcConfigBuilder {
        IrcConfigBuilder {
            nickname: None,
            username: None,
            hostname: None,
            servername: None,
            realname: None,

            password: None,

            raw_receive_handler: None,
            receive_handler: None,
        }
    }

    pub fn nickname(mut self, nickname: String) -> IrcConfigBuilder {
        self.nickname = Some(nickname);
        self
    }

    pub fn username(mut self, username: String) -> IrcConfigBuilder {
        self.username = Some(username);
        self
    }

    pub fn hostname(mut self, hostname: String) -> IrcConfigBuilder {
        self.hostname = Some(hostname);
        self
    }

    pub fn servername(mut self, servername: String) -> IrcConfigBuilder {
        self.servername = Some(servername);
        self
    }

    pub fn realname(mut self, realname: String) -> IrcConfigBuilder {
        self.realname = Some(realname);
        self
    }

    pub fn host(self, host: SocketAddr) -> Result<IrcConfig, super::error::IrcConfigBuilderError> {
        if self.nickname.is_none()
            || self.username.is_none()
            || self.hostname.is_none()
            || self.servername.is_none()
            || self.realname.is_none() {
                return Err(super::error::IrcConfigBuilderError);
            }
        
        Ok(IrcConfig {
            nickname: self.nickname.unwrap(),
            username: self.username.unwrap(),
            hostname: self.hostname.unwrap(),
            servername: self.servername.unwrap(),
            realname: self.realname.unwrap(),

            password: self.password,

            raw_receive_handler: self.raw_receive_handler,
            receive_handler: self.receive_handler,

            host,
        })
    }
}