use crate::message::IrcMessage;

pub enum Event {
    RawMessage(IrcMessage),

    StatusChange,
    WelcomeMsg(String),
    ErrorMsg(String),
    Notice(String),

    UnhandledMessage(IrcMessage), 
}
