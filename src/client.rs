use std::future::Future;
use std::future::IntoFuture;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::pin::Pin;
use std::sync::Arc;

use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use crate::context::ConnectionStatus;
use crate::context::Context;
use crate::event_handler::EventHandler;
use crate::message::IrcCommand;
use crate::message::IrcMessage;

pub struct ClientBuilder {
    server: SocketAddr,
    nickname: String,
    username: String,
    realname: String,

    event_handlers: Vec<Arc<dyn EventHandler>>,
}

impl ClientBuilder {
    pub fn new<A: ToSocketAddrs>(server: A, nickname: String, username: Option<String>, realname: Option<String>) -> Result<Self, std::io::Error> {
        Ok(Self {
            server: match server.to_socket_addrs()?.next() {
                Some(addr) => addr,
                None => return Err(std::io::Error::new(std::io::ErrorKind::AddrNotAvailable, "Could not resolve server address")),
            },
            nickname: nickname.clone(),
            username: username.unwrap_or(nickname.clone()),
            realname: realname.unwrap_or(nickname.clone()),

            event_handlers: Vec::new(),
        })
    }

    pub fn with_event_handler<H: EventHandler + 'static>(mut self, event_handler: H) -> Self {
        self.event_handlers.push(Arc::new(event_handler));
        self
    }
}

impl IntoFuture for ClientBuilder {
    type Output = Result<Client, std::io::Error>;

    type IntoFuture = Pin<Box<dyn Future<Output = Result<Client, std::io::Error>> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            Ok(Client {
                server: self.server,
                nickname: Arc::new(self.nickname),
                username: Arc::new(self.username),
                realname: Arc::new(self.realname),

                event_handlers: self.event_handlers,

                send: Arc::new(Mutex::new(None)),

                status: Arc::new(Mutex::new(ConnectionStatus::Connecting)),
            })
        })
    }
}

pub struct Client {
    server: SocketAddr,
    nickname: Arc<String>,
    username: Arc<String>,
    realname: Arc<String>,

    event_handlers: Vec<Arc<dyn EventHandler>>,

    send: Arc<Mutex<Option<OwnedWriteHalf>>>,

    status: Arc<Mutex<ConnectionStatus>>,
}

impl Client {
    pub fn builder<A: ToSocketAddrs>(server: A, nickname: String, username: Option<String>, realname: Option<String>) -> Result<ClientBuilder, std::io::Error> {
        ClientBuilder::new(server, nickname, username, realname)
    }

    pub async fn connect(&mut self) -> Result<(), std::io::Error> {
        let connection = TcpStream::connect(self.server).await?;

        let (receive, send) = connection.into_split();
        self.send = Arc::new(Mutex::new(Some(send)));
        
        {
            let username = self.username.clone();

            let send = self.send.clone();
            let event_handlers = self.event_handlers.clone();

            let status = self.status.clone();

            for event_handler in event_handlers.iter() {
                let status = status.lock().await;

                event_handler.on_status_change(Context {
                    status: Arc::new(status.clone()),
                });
            }

            tokio::spawn(async move {
                let mut reader = BufReader::new(receive);
                let event_handlers = event_handlers.clone();

                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).await.unwrap();
                    
                    let message = IrcMessage::try_from(line.as_str()).unwrap();

                    for event_handler in event_handlers.iter() {
                        event_handler.on_raw_message(message.clone());

                        match message.clone().command {
                            IrcCommand::Notice(target, message) => {
                                // TODO: Improve target matching
                                if target == username.as_str() || target == "*" {
                                    event_handler.on_notice(message);
                                }
                            },
                            IrcCommand::ErrorMsg(message) => {
                                event_handler.on_error(message);
                            },
                            IrcCommand::RplWelcome(target, message) => {
                                if target == username.as_str() {
                                    let mut status = status.lock().await;
                                    *status = ConnectionStatus::Connected;

                                    event_handler.on_status_change(Context {
                                        status: Arc::new(status.clone()),
                                    });

                                    event_handler.on_welcome(message);
                                }
                            },
                            IrcCommand::RplYourHost(target, message) => {
                                if target == username.as_str() {
                                    event_handler.on_your_host(message);
                                }
                            },
                            _ => {},
                        }
                    }

                    match message.command {
                        IrcCommand::Ping(message) => {
                            send.lock().await.as_mut().unwrap().write(String::try_from(IrcCommand::Pong(message)).unwrap().as_bytes()).await.unwrap();
                        },
                        _ => {},
                    }
                };
            });
        }

        println!("{:?}", String::try_from(IrcMessage{
            tags: vec![],
            prefix: None,
            command: IrcCommand::User(self.username.to_string(), self.realname.to_string()),
        }).unwrap());
        
        self.send.lock().await.as_mut().unwrap().write(String::try_from(IrcMessage{
            tags: vec![],
            prefix: None,
            command: IrcCommand::Nick(self.nickname.to_string()),
        }).unwrap().as_bytes()).await?;
        self.send.lock().await.as_mut().unwrap().write(String::try_from(IrcMessage{
            tags: vec![],
            prefix: None,
            command: IrcCommand::User(self.username.to_string(), self.realname.to_string()),
        }).unwrap().as_bytes()).await?;

        loop {}

        Ok(())
    }
}
