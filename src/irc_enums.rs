#[derive(Debug, PartialEq, Clone)]
pub enum IrcEvent {
    ReceiveMessage(String),
}

#[derive(Debug, PartialEq, Clone)]
pub enum IrcCommand {
    SendMessage(String),
}
