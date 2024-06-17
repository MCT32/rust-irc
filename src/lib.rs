pub mod config;
pub mod error;
pub mod irc_enums;

use config::IrcConfig;
use irc_enums::{IrcCommand, IrcEvent};
use tokio::sync::mpsc::{Receiver, Sender};

pub async fn backend(
    config: IrcConfig<'_>,
    tx: Sender<IrcEvent<'_>>,
    rx: Receiver<IrcCommand<'_>>,
) {
    let stream = tokio::net::TcpStream::connect(config.server_address)
        .await
        .unwrap();
}
