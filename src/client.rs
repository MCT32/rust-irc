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
use crate::event::Event;
use crate::event_handler::EventHandler;
use crate::message;
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
                motd: Arc::new(Mutex::new(Motd::Empty)),
            })
        })
    }
}

// TODO: Perhaps move to a separate file
#[derive(Debug, PartialEq, Clone)]
pub enum Motd {
    Empty,
    Building(String),
    Done(String),
}

pub struct Client {
    server: SocketAddr,
    nickname: Arc<String>,
    username: Arc<String>,
    realname: Arc<String>,

    event_handlers: Vec<Arc<dyn EventHandler>>,

    send: Arc<Mutex<Option<OwnedWriteHalf>>>,

    status: Arc<Mutex<ConnectionStatus>>,
    motd: Arc<Mutex<Motd>>,
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
            let motd = self.motd.clone();

            for event_handler in event_handlers.iter() {
                let status = status.lock().await;
                let motd = motd.lock().await;

                event_handler.on_event(Arc::new(Context {
                    status: Arc::new(status.clone()),
                    motd: Arc::new(motd.clone()),
                }), Event::StatusChange);
            }

            tokio::spawn(async move {
                let mut reader = BufReader::new(receive);
                let event_handlers = event_handlers.clone();

                loop {
                    let context = Arc::new(Context {
                        status: Arc::new(status.lock().await.clone()),
                        motd: Arc::new(motd.lock().await.clone()),
                    });

                    let mut line = String::new();
                    reader.read_line(&mut line).await.unwrap();
                    
                    let message = IrcMessage::try_from(line.as_str()).unwrap();

                    // TODO: Make error handling happen after message parsing
                    // TODO: Keep track of some data sent from server
                    for event_handler in event_handlers.iter() {
                        event_handler.on_event(context.clone(), Event::RawMessage(message.clone()));

                        match message.clone().command {
                            IrcCommand::Notice(target, message) => {
                                // TODO: Improve target matching
                                if target == username.as_str() || target == "*" {
                                    event_handler.on_event(context.clone(), Event::Notice(message));
                                }
                            },
                            IrcCommand::ErrorMsg(message) => {
                                event_handler.on_event(context.clone(), Event::ErrorMsg(message));
                            },
                            IrcCommand::RplWelcome(target, message) => {
                                if target == username.as_str() {
                                    let mut status = status.lock().await;
                                    *status = ConnectionStatus::Connected;

                                    event_handler.on_event(context.clone(), Event::StatusChange);

                                    event_handler.on_event(context.clone(), Event::WelcomeMsg(message));
                                }
                            },
                            IrcCommand::RplYourHost(target, message) => {
                                if target == username.as_str() {
                                    event_handler.on_event(context.clone(), Event::WelcomeMsg(message));
                                }
                            },
                            IrcCommand::RplCreated(target, message) => {
                                if target == username.as_str() {
                                    event_handler.on_event(context.clone(), Event::WelcomeMsg(message));
                                }
                            },
                            IrcCommand::RplMyInfo{
                                client,
                                servername,
                                version,
                                umodes,
                                cmodes,
                                cmodes_params,
                            } => {
                                if client == username.as_str() {
                                    event_handler.on_event(context.clone(), Event::WelcomeMsg(format!("Server: {}, Version: {}, UModes: {}, CModes: {}, CModes Params: {}", servername, version, umodes, cmodes, cmodes_params)));
                                }
                            },
                            IrcCommand::RplISupport(target, caps) => {
                                if target == username.as_str() {
                                    event_handler.on_event(context.clone(), Event::WelcomeMsg(format!("Supported capabilities: {}", caps.join(", "))));
                                }
                            },
                            IrcCommand::RplLUserClient(target, message) => {
                                if target == username.as_str() {
                                    event_handler.on_event(context.clone(), Event::WelcomeMsg(format!("{}", message)));
                                }
                            },
                            IrcCommand::RplLUserOp(target, ops, message) => {
                                if target == username.as_str() {
                                    event_handler.on_event(context.clone(), Event::WelcomeMsg(format!("{} {}", ops.to_string(), message)));
                                }
                            },
                            IrcCommand::RplLUserUnknown(target, connections, message) => {
                                if target == username.as_str() {
                                    event_handler.on_event(context.clone(), Event::WelcomeMsg(format!("{} {}", connections.to_string(), message)));
                                }
                            },
                            IrcCommand::RplLUserChannels(target, channels, message) => {
                                if target == username.as_str() {
                                    event_handler.on_event(context.clone(), Event::WelcomeMsg(format!("{} {}", channels.to_string(), message)));
                                }
                            },
                            IrcCommand::RplLUserMe(target, message) => {
                                if target == username.as_str() {
                                    event_handler.on_event(context.clone(), Event::WelcomeMsg(format!("{}", message)));
                                }
                            },
                            IrcCommand::RplLocalUsers(target, _users, message) => {
                                if target == username.as_str() {
                                    event_handler.on_event(context.clone(), Event::WelcomeMsg(format!("{}", message)));
                                }
                            },
                            IrcCommand::RplGlobalUsers(target, _users, message) => {
                                if target == username.as_str() {
                                    event_handler.on_event(context.clone(), Event::WelcomeMsg(format!("{}", message)));
                                }
                            },
                            // TODO: This code is executed per handler, which might fuck with the MOTD
                            IrcCommand::RplMotdStart(target, message) => {
                                if target == username.as_str() {
                                    let mut motd = motd.lock().await;

                                    if let Motd::Empty = *motd {
                                        let mut message = message.clone();
                                        message.push_str("\n");
                                        *motd = Motd::Building(message);
                                    } else {
                                        // TODO: Better error handling
                                        panic!("MOTD already started");
                                    }
                                }
                            },
                            IrcCommand::RplMotd(target, message) => {
                                if target == username.as_str() {
                                    let mut motd = motd.lock().await;

                                    if let Motd::Building(buffer) = motd.clone() {
                                        let mut buffer = buffer.clone();
                                        buffer.push_str(&message);
                                        buffer.push_str("\n");
                                        *motd = Motd::Building(buffer);
                                    } else {
                                        // TODO: Better error handling
                                        panic!("MOTD not started");
                                    }
                                }
                            },
                            IrcCommand::RplEndOfMotd(target, message) => {
                                if target == username.as_str() {
                                    let mut motd = motd.lock().await;

                                    if let Motd::Building(buffer) = motd.clone() {
                                        let mut buffer = buffer.clone();
                                        buffer.push_str(&message);
                                        *motd = Motd::Done(buffer);

                                        // TODO: New context has to be made bacause data has changed, not ideal
                                        event_handler.on_event(Arc::new(Context {
                                            status: Arc::new(status.lock().await.clone()),
                                            motd: Arc::new(motd.clone()),
                                        }), Event::Motd);
                                    } else {
                                        // TODO: Better error handling
                                        panic!("MOTD not started");
                                    }
                                }
                            },
                            IrcCommand::RplHostHidden(target, host, message) => {
                                if target == username.as_str() {
                                    event_handler.on_event(context.clone(), Event::WelcomeMsg(format!("{} {}", host, message)));
                                }
                            },
                            IrcCommand::Ping(_) => {},
                            _ => {
                                #[cfg(debug_assertions)]
                                {
                                    eprintln!("Unhandled message: {:?}", message.command);
                                }

                                event_handler.on_event(context.clone(), Event::UnhandledMessage(message.clone()));
                            },
                        }
                    }

                    match message.command {
                        IrcCommand::Ping(message) => {
                            send.lock().await.as_mut().unwrap().write(String::try_from(IrcMessage{
                                tags: vec![],
                                prefix: None,
                                command: IrcCommand::Pong(message),
                        }).unwrap().as_bytes()).await.unwrap();
                        },
                        _ => {},
                    }
                };
            });
        }
        
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
