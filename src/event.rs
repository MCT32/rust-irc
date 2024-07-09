use std::sync::Arc;

use crate::{context::Context, message::IrcMessage};

pub enum Event {
    RawMessage(Arc<Context>, IrcMessage),
    StatusChange(Arc<Context>),
    WelcomeMsg(Arc<Context>, String),
    ErrorMsg(Arc<Context>, String),
    Notice(Arc<Context>, String),
}
