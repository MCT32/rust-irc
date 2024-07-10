use std::sync::Arc;

use crate::client::Motd;

#[derive(Debug, Clone)]
pub struct Context {
    pub status: Arc<ConnectionStatus>,
    pub motd: Arc<Motd>,
}


#[derive(Debug, PartialEq, Clone)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Disconnected,
}
