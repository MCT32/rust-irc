use std::net::SocketAddr;

#[derive(Debug, PartialEq, Clone)]
pub struct IrcConfig {
    pub server_address: SocketAddr,
    pub username: String,       
    pub nickname: Option<String>, // If no nickname is provided, username is used     
    pub password: Option<String>, 
}