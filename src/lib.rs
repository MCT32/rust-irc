mod irc_enums;
mod config;
mod error;

use tokio::sync::mpsc::{Sender, Receiver};
use irc_enums::{IrcCommand, IrcEvent};
use config::IrcConfig;

pub async fn backend(config: IrcConfig<'_>, tx: Sender<IrcEvent<'_>>, rx: Receiver<IrcCommand<'_>>) {
    let stream = tokio::net::TcpStream::connect(config.server_address).await.unwrap();
}
