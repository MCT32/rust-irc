use crate::messages::{Command, Message};

#[derive(Clone)]
pub struct User {
    pub nickname: String,
    pub username: String,
    pub hostname: String,
    pub servername: String,
    pub realname: String,

    pub flags: UserFlags,
}

impl User {
    pub fn nick_command(&self) -> Message {
        Message {
            prefix: None,
            command: Command::Raw {
                command: "NICK".to_string(),
                params: vec![self.nickname.clone()],
            },
        }
    }

    pub fn user_command(&self) -> Message {
        Message {
            prefix: None,
            command: Command::Raw {
                command: "USER".to_string(),
                params: vec![self.username.clone(), self.hostname.clone(), self.servername.clone(), self.realname.clone()],
            },
        }
    }
}

#[derive(Default, Clone)]
pub struct UserFlags {
    pub invisible: bool,
    pub server_notices: bool,
    pub wallops: bool,
    pub operator: bool,
}
