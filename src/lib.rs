mod irc_enums;
mod config;
mod error;

use tokio::sync::mpsc::{Sender, Receiver};
use irc_enums::{IrcCommand, IrcEvent};
use config::IrcConfig;

pub fn backend(config: IrcConfig, tx: Sender<IrcEvent>, rx: Receiver<IrcCommand>) {

}
