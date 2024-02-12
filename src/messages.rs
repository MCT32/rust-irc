use std::{fmt, str::FromStr};


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

            Ok(Message {
                prefix: None,
                command: Command::Raw {
                    command: parts.first().unwrap().to_string(),
                    params: params,
                },
            })
        }
    }
} 

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Raw{command: String, params: Vec<String>},
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::Raw { command, params } => {
                if params.is_empty() {
                    return write!(f, "{}", command);
                }

                write!(f, "{} {}", command, params.join(" "))
            }
        }
    }
}


#[test]
fn command_fmt_with_prefix() {
    let result = Message {
        prefix: Some("tester".to_string()),
        command: Command::Raw {
            command: "NOTICE".to_string(),
            params: vec![":This is a test".to_string()],
        },
    };
    assert_eq!(format!("{}", result), ":tester NOTICE :This is a test");
}

#[test]
fn command_fmt_no_params() {
    let result = Message {
        prefix: None,
        command: Command::Raw {
            command: "QUIT".to_string(),
            params: vec![],
        },
    };
    assert_eq!(format!("{}", result), "QUIT");
}

#[test]
fn command_parse() {
    let result = Message::from_str("PRIVMSG #test :This is a test").unwrap();
    assert_eq!(result, Message {
        prefix: None,
        command: Command::Raw {
            command: "PRIVMSG".to_string(),
            params: vec!["#test".to_string(), ":This is a test".to_string()],
        },
    })
}

#[test]
fn command_parse_with_prefix() {
    let result = Message::from_str(":tester NOTICE :This is a test").unwrap();
    assert_eq!(result, Message {
        prefix: Some("tester".to_string()),
        command: Command::Raw {
            command: "NOTICE".to_string(),
            params: vec![":This is a test".to_string()],
        },
    })
}

#[test]
fn command_parse_no_params() {
    let result = Message::from_str("QUIT").unwrap();
    assert_eq!(result, Message {
        prefix: None,
        command: Command::Raw {
            command: "QUIT".to_string(),
            params: vec![],
        },
    })
}
