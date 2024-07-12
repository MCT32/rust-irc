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

                server_name: Arc::new(Mutex::new(String::new())),
                server_version: Arc::new(Mutex::new(String::new())),
                umodes: Arc::new(Mutex::new(String::new())),
                cmodes: Arc::new(Mutex::new(String::new())),
                cmodes_params: Arc::new(Mutex::new(String::new())),
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

    server_name: Arc<Mutex<String>>,
    server_version: Arc<Mutex<String>>,
    umodes: Arc<Mutex<String>>,
    cmodes: Arc<Mutex<String>>,
    cmodes_params: Arc<Mutex<String>>,
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

            let client_server_name = self.server_name.clone();
            let client_server_version = self.server_version.clone();
            let client_umodes = self.umodes.clone();
            let client_cmodes = self.cmodes.clone();
            let client_cmodes_params = self.cmodes_params.clone();

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
                    let mut line = String::new();
                    reader.read_line(&mut line).await.unwrap();
                    
                    let message = IrcMessage::try_from(line.as_str()).unwrap();

                    let events = match message.clone().command {
                        IrcCommand::Notice(target, message) => {
                            // TODO: Improve target matching
                            if target == username.as_str() || target == "*" {
                                vec![Event::Notice(message)]
                            } else {
                                vec![]
                            }
                        },
                        IrcCommand::ErrorMsg(message) => {
                            vec![Event::ErrorMsg(message)]
                        },
                        IrcCommand::RplWelcome(target, message) => {
                            if target == username.as_str() {
                                let mut status = status.lock().await;
                                *status = ConnectionStatus::Connected;

                                vec![Event::StatusChange, Event::WelcomeMsg(message)]
                            } else {
                                vec![]
                            }
                        },
                        IrcCommand::RplYourHost(target, message) => {
                            if target == username.as_str() {
                                vec![Event::WelcomeMsg(message)]
                            } else {
                                vec![]
                            }
                        },
                        IrcCommand::RplCreated(target, message) => {
                            if target == username.as_str() {
                                vec![Event::WelcomeMsg(message)]
                            } else {
                                vec![]
                            }
                        },
                        IrcCommand::RplMyInfo{
                            client,
                            server_name,
                            server_version,
                            umodes,
                            cmodes,
                            cmodes_params,
                        } => {
                            if client == username.as_str() {
                                let mut client_server_name = client_server_name.lock().await;
                                let mut client_server_version = client_server_version.lock().await;
                                let mut client_umodes = client_umodes.lock().await;
                                let mut client_cmodes = client_cmodes.lock().await;

                                *client_server_name = server_name.clone();
                                *client_server_version = server_version.clone();
                                *client_umodes = umodes.clone();
                                *client_cmodes = cmodes.clone();
                                
                                if let Some(cmodes_params) = cmodes_params.clone() {
                                    let mut client_cmodes_params = client_cmodes_params.lock().await;
                                    *client_cmodes_params = cmodes_params.clone();
                                }

                                // TODO: Message doesn't need to be printed to the user, but it might be a good idea to add an event for it
                                vec![]
                            } else {
                                vec![]
                            }
                        },
                        IrcCommand::RplISupport(target, caps, message) => {
                            if target == username.as_str() {
                                vec![Event::WelcomeMsg(format!("{} {}", caps.join(", "), message))]
                            } else {
                                vec![]
                            }
                        },
                        IrcCommand::RplLUserClient(target, message) => {
                            if target == username.as_str() {
                                vec![Event::WelcomeMsg(format!("{}", message))]
                            } else {
                                vec![]
                            }
                        },
                        IrcCommand::RplLUserOp(target, ops, message) => {
                            if target == username.as_str() {
                                vec![Event::WelcomeMsg(format!("{} {}", ops.to_string(), message))]
                            } else {
                                vec![]
                            }
                        },
                        IrcCommand::RplLUserUnknown(target, connections, message) => {
                            if target == username.as_str() {
                                vec![Event::WelcomeMsg(format!("{} {}", connections.to_string(), message))]
                            } else {
                                vec![]
                            }
                        },
                        IrcCommand::RplLUserChannels(target, channels, message) => {
                            if target == username.as_str() {
                                vec![Event::WelcomeMsg(format!("{} {}", channels.to_string(), message))]
                            } else {
                                vec![]
                            }
                        },
                        IrcCommand::RplLUserMe(target, message) => {
                            if target == username.as_str() {
                                vec![Event::WelcomeMsg(format!("{}", message))]
                            } else {
                                vec![]
                            }
                        },
                        IrcCommand::RplLocalUsers(target, _users, message) => {
                            if target == username.as_str() {
                                vec![Event::WelcomeMsg(format!("{}", message))]
                            } else {
                                vec![]
                            }
                        },
                        IrcCommand::RplGlobalUsers(target, _users, message) => {
                            if target == username.as_str() {
                                vec![Event::WelcomeMsg(format!("{}", message))]
                            } else {
                                vec![]
                            }
                        },
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

                            vec![]
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

                            vec![]
                        },
                        IrcCommand::RplEndOfMotd(target, message) => {
                            if target == username.as_str() {
                                let mut motd = motd.lock().await;

                                if let Motd::Building(buffer) = motd.clone() {
                                    let mut buffer = buffer.clone();
                                    buffer.push_str(&message);
                                    *motd = Motd::Done(buffer);

                                    vec![Event::Motd]
                                } else {
                                    // TODO: Better error handling
                                    panic!("MOTD not started");
                                }
                            } else {
                                vec![]
                            }
                        },
                        IrcCommand::RplHostHidden(target, host, message) => {
                            if target == username.as_str() {
                                vec![Event::WelcomeMsg(format!("{} {}", host, message))]
                            } else {
                                vec![]
                            }
                        },
                        IrcCommand::Ping(_) => vec![],
                        _ => {
                            #[cfg(debug_assertions)]
                            {
                                eprintln!("Unhandled message: {:?}", message.command);
                            }

                            vec![Event::UnhandledMessage(message.clone())]
                        },
                    };

                    let context = Arc::new(Context {
                        status: Arc::new(status.lock().await.clone()),
                        motd: Arc::new(motd.lock().await.clone()),
                    });

                    // TODO: Make error handling happen after message parsing
                    // TODO: Keep track of some data sent from server
                    for event_handler in event_handlers.iter() {
                        event_handler.on_event(context.clone(), Event::RawMessage(message.clone()));

                        for event in events.iter() {
                            event_handler.on_event(context.clone(), event.clone());
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
