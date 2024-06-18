pub enum IrcEvent {
    ReceiveMessage(String),
}

pub enum IrcCommand {
    SendMessage(String),
}
