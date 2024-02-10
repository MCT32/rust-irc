use tokio::{io, net::TcpStream, sync::Mutex};
use std::{io::Error, net::{IpAddr, Ipv4Addr, SocketAddr}, sync::Arc};


type MessageHandler = fn(&str);

#[derive(Clone)]
pub struct IrcConfig {
    host: SocketAddr,
    raw_receive_handler: Option<MessageHandler>,
}

impl IrcConfig {
    pub fn new() -> Self {
        IrcConfig {
            host: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6667),
            raw_receive_handler: None,
        }
    }

    pub fn host(&mut self, host: SocketAddr) -> &mut Self {
        self.host = host;
        self
    }

    pub fn receive_handler(&mut self, handler: MessageHandler) -> &mut Self {
        self.raw_receive_handler = Some(handler);
        self
    }

    pub async fn connect(&self) -> Result<IrcConnection, Error> {
        match TcpStream::connect(self.host).await {
            Ok(stream) => {
                let connection = IrcConnection {
                    stream: Arc::new(Mutex::new(stream)),
                    config: self.clone(),
                };

                connection.init();

                Ok(connection)
            }
            Err(err) => Err(err)
        }
    }
}


pub struct IrcConnection {
    stream: Arc<Mutex<TcpStream>>,
    config: IrcConfig,
}

impl IrcConnection {
    pub async fn send_raw<T: Into<String>>(&mut self, msg: T) -> Result<usize, Error> {
        let msg = msg.into();
        println!("{}", msg);
        self.stream.lock().await.try_write(msg.as_bytes())
    }

    pub fn init(&self) {
        if self.config.raw_receive_handler.is_some() {
            tokio::spawn(Self::receive_loop(self.config.clone(), self.stream.clone()));
        }
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
                Some(func) => func(std::str::from_utf8(buf_str).unwrap()),
                _ => ()
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::{net::lookup_host, time::sleep};

    use crate::IrcConfig;

    #[tokio::test]
    async fn connecting() {
        let mut config = IrcConfig::new();
        let config = config.host(lookup_host("irc.libera.chat:6667").await.unwrap().next().expect("Failed to resolve"))
            .receive_handler(|msg| {
                println!("{}", msg);
            });

        let mut connection = config.connect().await.unwrap();

        sleep(Duration::from_secs(5)).await;

        connection.send_raw("NICK mct33\n").await.unwrap();
        sleep(Duration::from_secs(1)).await;
        connection.send_raw("USER mct33 mct33 mct33 mct33\n").await.unwrap();
        sleep(Duration::from_secs(3)).await;
        connection.send_raw("JOIN #test\n").await.unwrap();
        sleep(Duration::from_secs(1)).await;
        connection.send_raw("PRIVMSG #test :testing..\n").await.unwrap();
        sleep(Duration::from_secs(1)).await;
        connection.send_raw("QUIT\n").await.unwrap();
    }
}
