pub mod config;
pub mod irc_enums;

use config::IrcConfig;
use irc_enums::{IrcCommand, IrcEvent};
use tokio::sync::mpsc::{Receiver, Sender};

pub async fn backend(config: IrcConfig, tx: Sender<IrcEvent>, rx: Receiver<IrcCommand>) {
    let stream = tokio::net::TcpStream::connect(config.server_address)
        .await
        .unwrap();
}
