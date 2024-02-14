mod replys;


use std::{fmt::{self, Error}, str::FromStr};

use replys::{Reply, ErrorReply};


#[derive(Debug, PartialEq, Eq)]
pub struct Message {
    pub prefix: Option<String>,
    pub command: Command,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.prefix {
            Some(prefix) => write!(f, ":{} {}", prefix, self.command),
            None => write!(f, "{}", self.command),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseCommandError;

impl FromStr for Message {
    type Err = ParseCommandError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split_whitespace().collect();

        if parts.is_empty() {
            return Err(ParseCommandError);
        }

        if parts.first().unwrap().starts_with(":") {
            let prefix = parts.first().unwrap()[1..].to_string();

            let mut message = Message::from_str(parts[1..].join(" ").as_str()).unwrap();
            message.prefix = Some(prefix);
            Ok(message)
        } else {
            let mut params: Vec<String> = Vec::with_capacity(15);

            let mut combining = false;
            let mut combined_string = String::new();

            for x in parts[1..].into_iter() {
                if combining {
                    combined_string.push_str(" ");
                    combined_string.push_str(x);
                } else if x.starts_with(":") {
                    combining = true;
                    combined_string = x.to_string();
                } else {
                    params.append(&mut vec![x.to_string()]);
                }
            }

            if combining {
                params.append(&mut vec![combined_string])
            }

            let command = parts.first().unwrap().to_string();

            Ok(Message {
                prefix: None,
                command: match command.as_str() {
                    "PASS" => Command::Pass(params[0].clone()),
                    "NICK" => Command::Nick(params[0].clone()),
                    "USER" => Command::User(params[0].clone(), params[1].clone(), params[2].clone(), params[3].clone()),
                    "QUIT" => Command::Quit,
                    "NOTICE" => Command::Notice(params[0].clone(), params[1].clone()),
                    "PRIVMSG" => Command::PrivMsg(params[0].clone(), params[1].clone()),
                    "JOIN" => Command::Join(params[0].clone()),
                    _ => Command::Raw(command, params)
                }
            })
        }
    }
} 

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Command {
    Pass(String),
    Nick(String),
    User(String, String, String, String),
    Quit,
    Notice(String, String),
    PrivMsg(String, String),
    Join(String),
    Reply(Result<Reply, ErrorReply>),
    Raw(String, Vec<String>),
}

impl Command {
    pub fn raw(&self) -> Command { // borrowed self for u <3 - sam (for me not u)
        match self {
            Command::Pass(pass) => Command::Raw("PASS".to_string(), vec![pass.clone()]),
            Command::Nick(nickname) => Command::Raw("NICK".to_string(), vec![nickname.clone()]),
            Command::User(username, hostname, servername, realname) => {
                Command::Raw("USER".to_string(), vec![username.clone(), hostname.clone(), servername.clone(), realname.clone()])
            },
            Command::Quit => Command::Raw("QUIT".to_string(), vec![]),
            Command::Notice(nickname, notice) => Command::Raw("NOTICE".to_string(), vec![nickname.clone(), notice.clone()]),
            Command::PrivMsg(receiver, message) => Command::Raw("PRIVMSG".to_string(), vec![receiver.clone(), message.clone()]),
            Command::Join(channel) => Command::Raw("JOIN".to_string(), vec![channel.clone()]), // and cloned fucking everything. sorry
            Command::Reply(reply) => {
                match reply {
                    Ok(reply) => ,
                    Err(reply) => ,
                }
            }
            Command::Raw(_, _) => self.clone(), // svelte says u dont know how to write rust. also i cloned self
        }
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.clone().raw() {
            Command::Raw(command, params) => {
                if params.is_empty() {
                    return write!(f, "{}", command);
                }
        
                write!(f, "{} {}", command, params.join(" "))
            },
            _ => Err(Error),
        }
    }
}


#[test]
fn command_fmt_with_prefix() {
    let result = Message {
        prefix: Some("tester".to_string()),
        command: Command::Notice("tester".to_string(), ":This is a test".to_string()),
    };
    assert_eq!(format!("{}", result), ":tester NOTICE tester :This is a test");
}

#[test]
fn command_fmt_no_params() {
    let result = Message {
        prefix: None,
        command: Command::Quit,
    };
    assert_eq!(format!("{}", result), "QUIT");
}

#[test]
fn command_parse() {
    let result = Message::from_str("PRIVMSG #test :This is a test").unwrap();
    assert_eq!(result, Message {
        prefix: None,
        command: Command::PrivMsg("#test".to_string(), ":This is a test".to_string()),
    })
}

#[test]
fn command_parse_with_prefix() {
    let result = Message::from_str(":tester NOTICE tester :This is a test").unwrap();
    assert_eq!(result, Message {
        prefix: Some("tester".to_string()),
        command: Command::Notice("tester".to_string(), ":This is a test".to_string()),
    })
}

#[test]
fn command_parse_no_params() {
    let result = Message::from_str("QUIT").unwrap();
    assert_eq!(result, Message {
        prefix: None,
        command: Command::Quit,
    })
}
