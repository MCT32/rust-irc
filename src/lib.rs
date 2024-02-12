pub mod messages;
pub mod error;
pub mod config;
pub mod users;


use config::IrcConfig;
use error::{IrcInitError, IrcSendError};
use messages::{Command, Message};
use tokio::{io::{self, Interest}, net::TcpStream, sync::Mutex};
use std::{str::FromStr, sync::Arc};


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
            Err(err) => Err(IrcSendError::TcpSendError(err))
        }
    }

    pub async fn send(&mut self, msg: Message) -> Result<usize, IrcSendError> {
        let mut msg = msg.to_string();
        msg.push_str("\n");
        print!("{}", msg);
        match self.stream.lock().await.try_write(msg.as_bytes()) {
            Ok(bytes_sent) => Ok(bytes_sent),
            Err(err) => Err(IrcSendError::TcpSendError(err))
        }
    }

    pub async fn init(&mut self) -> Result<(), IrcInitError> {
        if self.config.raw_receive_handler.is_some() || self.config.receive_handler.is_some() {
            tokio::spawn(Self::receive_loop(self.config.clone(), self.stream.clone()));
        }

        if let Err(err) = self.stream.lock().await.ready(Interest::READABLE | Interest::WRITABLE).await {
            return Err(IrcInitError::TcpConnectionError(err));
        }

        if let Some(password) = &self.config.password {
            if let Err(err) = self.send(Message {
                prefix: None,
                command: Command::Raw {
                    command: "PASS".to_string(),
                    params: vec![password.to_string()],
                },
            }).await {
                return Err(IrcInitError::IrcSendError(err));
            }
        }

        if let Err(err) = self.send(self.config.user.nick_command()).await {
            return Err(IrcInitError::IrcSendError(err));
        }

        if let Err(err) = self.send(self.config.user.user_command()).await {
            return Err(IrcInitError::IrcSendError(err));
        }

        Ok(())
    }

    pub async fn quit(&mut self) -> Result<usize, IrcSendError> {
        self.send(Message {
            prefix: None,
            command: Command::Raw {
                command: "QUIT".to_string(),
                params: vec![],
            },
        }).await
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