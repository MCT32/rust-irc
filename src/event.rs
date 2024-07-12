use crate::message::IrcMessage;

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    RawMessage(IrcMessage),

    StatusChange,
    WelcomeMsg(String),
    ErrorMsg(String),
    Notice(String),

    Motd,

    UnhandledMessage(IrcMessage), 
}
