mod messages;


use messages::{Message, Params};
use tokio::{io::{self, Interest}, net::TcpStream, sync::Mutex};
use core::fmt;
use std::{error::Error, net::{IpAddr, Ipv4Addr, SocketAddr}, str::FromStr, sync::Arc};


type RawMessageHandler = fn(&str);
type MessageHandler = fn(messages::Message);

// TODO: Improve error verbosity
#[derive(Debug)]
pub struct IrcConnectError;
impl Error for IrcConnectError {}
impl fmt::Display for IrcConnectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "could not connect to irc server")
    }
}

// TODO: Improve error verbosity
#[derive(Debug)]
pub struct IrcSendError;
impl Error for IrcSendError {}
impl fmt::Display for IrcSendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "could not send message")
    }
}

#[derive(Clone)]
pub struct IrcConfig {
    host: SocketAddr,

    nickname: String,
    username: String,
    hostname: String,
    servername: String,
    realname: String,

    password: Option<String>,

    raw_receive_handler: Option<RawMessageHandler>,
    receive_handler: Option<MessageHandler>,
}

impl IrcConfig {
    pub fn new() -> Self {
        IrcConfig {
            host: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6667),

            nickname: "".to_string(),
            username: "".to_string(),
            hostname: "".to_string(),
            servername: "".to_string(),
            realname: "".to_string(),

            password: None,

            raw_receive_handler: None,
            receive_handler: None,
        }
    }

    pub fn host(&mut self, host: SocketAddr) -> &mut Self {
        self.host = host;
        self
    }

    pub fn set_raw_receive_handler(&mut self, handler: RawMessageHandler) -> &mut Self {
        self.raw_receive_handler = Some(handler);
        self
    }

    pub fn set_receive_handler(&mut self, handler: MessageHandler) -> &mut Self {
        self.receive_handler = Some(handler);
        self
    }

    fn check_info(&self) -> bool {
        !self.nickname.is_empty()
            && !self.username.is_empty()
            && !self.hostname.is_empty()
            && !self.servername.is_empty()
            && !self.realname.is_empty()
    }

    pub async fn connect(&self) -> Result<IrcConnection, IrcConnectError> {
        if !self.check_info() {
            return Err(IrcConnectError);
        }

        match TcpStream::connect(self.host).await {
            Ok(stream) => {
                let mut connection = IrcConnection {
                    stream: Arc::new(Mutex::new(stream)),
                    config: self.clone(),
                };

                connection.init().await;

                Ok(connection)
            }
            Err(_) => Err(IrcConnectError)
        }
    }
}


pub struct IrcConnection {
    stream: Arc<Mutex<TcpStream>>,
    config: IrcConfig,
}

impl IrcConnection {
    pub async fn send_raw<T: Into<String>>(&mut self, msg: T) -> Result<usize, IrcSendError> {
        let mut msg: String = msg.into();
        msg.push_str("\n");
        match self.stream.lock().await.try_write(msg.as_bytes()) {
            Ok(bytes_sent) => Ok(bytes_sent),
            Err(_) => Err(IrcSendError)
        }
    }

    pub async fn send(&mut self, msg: Message) -> Result<usize, IrcSendError> {
        let mut msg = msg.to_string();
        msg.push_str("\n");
        print!("{}", msg);
        match self.stream.lock().await.try_write(msg.as_bytes()) {
            Ok(bytes_sent) => Ok(bytes_sent),
            Err(_) => Err(IrcSendError)
        }
    }

    pub async fn init(&mut self) {
        if self.config.raw_receive_handler.is_some() || self.config.receive_handler.is_some() {
            tokio::spawn(Self::receive_loop(self.config.clone(), self.stream.clone()));
        }

        self.stream.lock().await.ready(Interest::READABLE | Interest::WRITABLE).await.unwrap();

        if let Some(password) = &self.config.password {
            self.send(Message {
                prefix: None,
                command: "PASS".to_string(),
                params: Params(vec![password.to_string()])
            }).await.unwrap();
        }

        self.send(Message {
            prefix: None,
            command: "NICK".to_string(),
            params: Params(vec![self.config.nickname.clone()])
        }).await.unwrap();

        self.send(Message {
            prefix: None,
            command: "USER".to_string(),
            params: Params(vec![self.config.username.clone(), self.config.hostname.clone(), self.config.servername.clone(), self.config.realname.clone()])
        }).await.unwrap();
    }

    pub async fn quit(&mut self) {
        self.send(Message {
            prefix: None,
            command: "QUIT".to_string(),
            params: Params(vec![])
        }).await.unwrap();
    }

    async fn receive_loop(config: IrcConfig, stream: Arc<Mutex<TcpStream>>) {
        let mut buf = [0; 1024];

        loop {
            let bytes_read = match stream.lock().await.try_read(&mut buf) {
                Ok(n) if n == 0 => {
                    println!("Connection closed by server.");
                    break;
                }
                Ok(n) => n,
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    eprintln!("Error reading socket: {}", e);
                    break;
                }
            };

            let buf_str = &buf[0..bytes_read];
            match config.raw_receive_handler {
                Some(func) => func(std::str::from_utf8(&buf_str).unwrap()),
                _ => ()
            }

            match config.receive_handler {
                Some(func) => func(messages::Message::from_str(std::str::from_utf8(&buf_str).unwrap()).unwrap()),
                _ => ()
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::messages::{self, Message, Params};

    #[test]
    fn command_fmt() {
        let result = messages::Message {
            prefix: None,
            command: "NOTICE".to_string(),
            params: messages::Params(vec![":This is a test".to_string()])
        };
        assert_eq!(format!("{}", result), "NOTICE :This is a test");
    }

    #[test]
    fn command_fmt_with_prefix() {
        let result = messages::Message {
            prefix: Some("tester".to_string()),
            command: "NOTICE".to_string(),
            params: messages::Params(vec![":This is a test".to_string()])
        };
        assert_eq!(format!("{}", result), ":tester NOTICE :This is a test");
    }

    #[test]
    fn command_fmt_no_params() {
        let result = messages::Message {
            prefix: None,
            command: "QUIT".to_string(),
            params: messages::Params(vec![])
        };
        assert_eq!(format!("{}", result), "QUIT");
    }

    #[test]
    fn command_parse() {
        let result = Message::from_str("PRIVMSG #test :This is a test").unwrap();
        assert_eq!(result, Message {
            prefix: None,
            command: "PRIVMSG".to_string(),
            params: Params(vec!["#test".to_string(), ":This is a test".to_string()]),
        })
    }

    #[test]
    fn command_parse_with_prefix() {
        let result = Message::from_str(":tester NOTICE :This is a test").unwrap();
        assert_eq!(result, Message {
            prefix: Some("tester".to_string()),
            command: "NOTICE".to_string(),
            params: Params(vec![":This is a test".to_string()]),
        })
    }

    #[test]
    fn command_parse_no_params() {
        let result = Message::from_str("QUIT").unwrap();
        assert_eq!(result, Message {
            prefix: None,
            command: "QUIT".to_string(),
            params: Params(vec![]),
        })
    }
}
