use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Context {
    pub status: Arc<ConnectionStatus>
}


#[derive(Debug, PartialEq, Clone)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Disconnected,
}
