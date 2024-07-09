use regex::Regex;

use crate::error::Error;



#[derive(Debug, PartialEq, Clone)]
pub struct IrcMessage {
    pub tags: Vec<(String, Option<String>)>,
    pub prefix: Option<String>,
    pub command: IrcCommand,
}

impl TryFrom<&str> for IrcMessage {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Error> {
        let re = Regex::new("^(?:@([^\\n\\r\\x00 ]+) )?(?::([^\\r\\n\\x00 ]+) )?((?:[A-Z]+|[0-9]{3})( [^\\n\\r\\x00]+)?)\\r\\n$").unwrap();

        let Some(caps) = re.captures(value) else {
            return Err(Error::NoMatch(value.to_string()));
        };

        let tags = match caps.get(1).map(|m| m.as_str().to_string()) {
            None => vec![],
            Some(tags) => {
                tags.split(';').into_iter().map(|m| {
                    match m.split_once("=") {
                        Some((key, value)) => {
                            (key.to_string(), Some(value.to_string()))
                        },
                        None => {
                            (m.to_string(), None)
                        }
                    }
                }).collect::<Vec<_>>()
            }
        };

        let prefix = caps.get(2).map(|m| m.as_str().to_string());

        let Some(command) = caps.get(3).map(|m| m.as_str()) else {
            return Err(Error::NoCommand(value.to_string()));
        };

        let Ok(command) = IrcCommand::try_from(command) else {
            return Err(Error::Invalid);
        };

        // let params = match caps.get(4).map(|m| m.as_str()) {
        //     None => vec![],
        //     Some(params) => {
        //         match params.split_once(" :") {
        //             Some((params, trailing)) => {
        //                 // TODO: Surely better way to do this
        //                 let mut params = params.split(' ').into_iter().collect::<Vec<_>>();
        //                 params.append(&mut vec![trailing]);
        //                 params
        //             },
        //             None => {
        //                 params.split(' ').into_iter().collect::<Vec<_>>()
        //             }
        //         }
        //     },
        // }.into_iter().map(|m| m.to_string()).collect();

        Ok(IrcMessage {
            tags,
            prefix,
            command,
        })
    }
}

impl TryFrom<IrcMessage> for String {
    type Error = Error;

    fn try_from(value: IrcMessage) -> Result<Self, Error> {
        let mut buffer = String::new();

        if !value.tags.is_empty() {
            buffer.push_str("@");

            let length = value.tags.len();

            for (index, tag) in value.tags.into_iter().enumerate() {
                if let Some(value) = tag.1 {
                    buffer.push_str(format!("{}={}", tag.0.as_str(), &value).as_str());
                } else {
                    buffer.push_str(tag.0.as_str());
                }
                
                if !(index == length - 1) {
                    buffer.push_str(";");
                }
            }

            buffer.push_str(" ");
        }

        if let Some(prefix) = value.prefix {
            buffer.push_str(format!(":{} ", prefix).as_str());
        };

        buffer.push_str(String::try_from(GenericIrcCommand::from(value.command))?.as_str());

        buffer.push_str("\r\n");

        Ok(buffer)
    }
}



#[derive(Debug, PartialEq, Clone)]
pub enum IrcCommand {
    Pass(String),
    Nick(String),
    // username, realname
    User(String, String),
    Ping(String),
    Pong(String),
    Notice(String, String),
    // had to add Msg to stop compiler from complaining
    ErrorMsg(String),

    RplWelcome(String, String), // 001 RPL_WELCOME
    RplYourHost(String, String), // 002 RPL_YOURHOST

    Generic(GenericIrcCommand),
}

impl TryFrom<GenericIrcCommand> for IrcCommand {
    type Error = Error;

    fn try_from(value: GenericIrcCommand) -> Result<Self, Error> {
        match &value.command {
            GenericIrcCommandType::Text(command) => {
                match command.as_str() {
                    "PASS" => Ok(Self::Pass(value.params.get(0).unwrap().clone())),
                    "NICK" => Ok(Self::Nick(value.params.get(0).unwrap().clone())),
                    "USER" => Ok(Self::User(value.params.get(0).unwrap().clone(),
                        value.params.get(1).unwrap().clone())),
                    "PING" => Ok(Self::Ping(value.params.get(0).unwrap().clone())),
                    "PONG" => Ok(Self::Pong(value.params.get(0).unwrap().clone())),
                    "NOTICE" => Ok(Self::Notice(value.params.get(0).unwrap().clone(), value.params.get(1).unwrap().clone())),
                    "ERROR" => Ok(Self::ErrorMsg(value.params.get(0).unwrap().clone())),
                    _ => Ok(Self::Generic(value)),
                }
            },
            GenericIrcCommandType::Number(command) => {
                match command {
                    001 => Ok(Self::RplWelcome(value.params.get(0).unwrap().clone(), value.params.get(1).unwrap().clone())),
                    002 => Ok(Self::RplYourHost(value.params.get(0).unwrap().clone(), value.params.get(1).unwrap().clone())),
                    _ => Ok(Self::Generic(value)),
                }
            },
        }
    }
}

impl TryFrom<&str> for IrcCommand {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        GenericIrcCommand::try_from(value)?.try_into()
    }
}

impl From<IrcCommand> for GenericIrcCommand {
    fn from(value: IrcCommand) -> Self {
        match value {
            IrcCommand::Pass(password) => GenericIrcCommand {
                command: GenericIrcCommandType::Text("PASS".to_string()),
                params: vec![password],
            },
            IrcCommand::Nick(nickname) => GenericIrcCommand {
                command: GenericIrcCommandType::Text("NICK".to_string()),
                params: vec![nickname],
            },
            IrcCommand::User(username, realname) => GenericIrcCommand {
                command: GenericIrcCommandType::Text("USER".to_string()),
                params: vec![username, "0".to_string(), "*".to_string(), realname],
            },
            IrcCommand::Ping(message) => GenericIrcCommand {
                command: GenericIrcCommandType::Text("PING".to_string()),
                params: vec![message],
            },
            IrcCommand::Pong(message) => GenericIrcCommand {
                command: GenericIrcCommandType::Text("PONG".to_string()),
                params: vec![message],
            },
            IrcCommand::Notice(target, message) => GenericIrcCommand {
                command: GenericIrcCommandType::Text("NOTICE".to_string()),
                params: vec![target, message],
            },
            IrcCommand::ErrorMsg(message) => GenericIrcCommand {
                command: GenericIrcCommandType::Text("ERROR".to_string()),
                params: vec![message],
            },

            IrcCommand::RplWelcome(client, message) => GenericIrcCommand {
                command: GenericIrcCommandType::Number(001),
                params: vec![client, message],
            },
            IrcCommand::RplYourHost(client, message) => GenericIrcCommand {
                command: GenericIrcCommandType::Number(002),
                params: vec![client, message],
            },

            IrcCommand::Generic(generic) => generic,
        }
    }
}

impl TryFrom<IrcCommand> for String {
    type Error = Error;

    fn try_from(value: IrcCommand) -> Result<Self, Error> {
        GenericIrcCommand::from(value).try_into()
    }
}



#[derive(Debug, PartialEq, Clone)]
pub enum GenericIrcCommandType {
    Text(String),
    Number(u16),
}

impl TryFrom<&str> for GenericIrcCommandType {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.chars().next().unwrap() {
            '0'..='9' => {
                if value.len() == 3 && value.chars().all(|c| c.is_numeric()) {
                    Ok(Self::Number(value.parse::<u16>().unwrap()))
                } else {
                    Err(Error::Invalid)
                }
            },
            'A'..='Z' => {
                if value.chars().all(|c| c.is_ascii_uppercase()) {
                    Ok(Self::Text(value.to_string()))
                } else {
                    Err(Error::Invalid)
                }
            },
            _ => {
                Err(Error::Invalid)
            }
        }
    }
}

impl From<GenericIrcCommandType> for String {
    fn from(value: GenericIrcCommandType) -> Self {
        match value {
            GenericIrcCommandType::Number(number) => format!("{:03}", number),
            GenericIrcCommandType::Text(text) => text,
        }
    }
}



#[derive(Debug, PartialEq, Clone)]
pub struct GenericIrcCommand {
    pub command: GenericIrcCommandType,
    pub params: Vec<String>,
}

impl TryFrom<&str> for GenericIrcCommand {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let re = Regex::new("^([A-Z]+|[0-9]{3})(?: ([^\\n\\r\\x00]+))?$").unwrap();

        let Some(caps) = re.captures(value) else {
            return Err(Error::NoMatch(value.to_string()));
        };

        let Some(command) = caps.get(1).map(|m| m.as_str()) else {
            return Err(Error::NoCommand(value.to_string()));
        };

        let command = GenericIrcCommandType::try_from(command)?;

        let params = match caps.get(2).map(|m| m.as_str()) {
            None => vec![],
            Some(params) => {
                match params.split_once(":") {
                    Some((params, trailing)) => {
                        // TODO: Surely better way to do this
                        let mut params = if params.is_empty() {
                            vec![]
                        } else {
                            params.get(0..params.len() - 1).unwrap().split(' ').into_iter().collect::<Vec<_>>()
                        };
                        params.append(&mut vec![trailing]);
                        params
                    },
                    None => {
                        params.split(' ').into_iter().collect::<Vec<_>>()
                    }
                }
            },
        }.into_iter().map(|m| m.to_string()).collect();
        
        Ok(GenericIrcCommand {
            command,
            params,
        })
    }
}

impl TryFrom<GenericIrcCommand> for String {
    type Error = Error;

    fn try_from(value: GenericIrcCommand) -> Result<Self, Error> {
        let mut buffer = String::new();

        // if !value.tags.is_empty() {
        //     buffer.push_str("@");

        //     let length = value.tags.len();

        //     for (index, tag) in value.tags.into_iter().enumerate() {
        //         if let Some(value) = tag.1 {
        //             buffer.push_str(format!("{}={}", tag.0.as_str(), &value).as_str());
        //         } else {
        //             buffer.push_str(tag.0.as_str());
        //         }
                
        //         if !(index == length - 1) {
        //             buffer.push_str(";");
        //         }
        //     }

        //     buffer.push_str(" ");
        // }

        // if let Some(prefix) = value.prefix {
        //     buffer.push_str(format!(":{} ", prefix).as_str());
        // };

        buffer.push_str(String::from(value.command).as_str());

        if !value.params.is_empty() {
            let last = value.params.last().unwrap();

            let params = value.params.iter().take(value.params.len() - 1);

            if !params.clone().all(|p| !p.contains(' ')) { return Err(Error::Invalid) };

            for param in params {
                buffer.push_str(format!(" {}", param.as_str()).as_str());
            };

            if last.contains(' ') {
                buffer.push_str(format!(" :{}", last).as_str());
            } else {
                buffer.push_str(format!(" {}", last).as_str());
            }
        }

        // buffer.push_str("\r\n");

        Ok(buffer)
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_string() {
        assert_eq!("LEAVE\r\n".try_into(), Ok(IrcMessage {
            tags: vec![],
            prefix: None,
            command: IrcCommand::Generic(GenericIrcCommand {
                command: GenericIrcCommandType::Text("LEAVE".to_string()),
                params: vec![],
            }),
        }));

        assert_eq!(":server PRIVMSG #meme :11/10 cock\r\n".try_into(), Ok(IrcMessage {
            tags: vec![],
            prefix: Some("server".to_string()),
            command: IrcCommand::Generic(GenericIrcCommand {
                command: GenericIrcCommandType::Text("PRIVMSG".to_string()),
                params: vec!["#meme", "11/10 cock"].into_iter().map(|m| m.to_string()).collect(),
            }),
        }));

        assert_eq!(":server 404 shit\r\n".try_into(), Ok(IrcMessage {
            tags: vec![],
            prefix: Some("server".to_string()),
            command: IrcCommand::Generic(GenericIrcCommand {
                command: GenericIrcCommandType::Number(404),
                params: vec!["shit".to_string()],
            }),
        }));

        assert_eq!("@foo;bar;test_tag=plumbus :127.0.0.1 MSG #rust :rustaceans rise!\r\n".try_into(), Ok(IrcMessage {
            tags: vec![("foo".to_string(), None), ("bar".to_string(), None), ("test_tag".to_string(), Some("plumbus".to_string()))],
            prefix: Some("127.0.0.1".to_string()),
            command: IrcCommand::Generic(GenericIrcCommand {
                command: GenericIrcCommandType::Text("MSG".to_string()),
                params: vec!["#rust", "rustaceans rise!"].into_iter().map(|m| m.to_string()).collect(),
            }),
        }));

        assert_eq!(":*.freenode.net NOTICE * :*** Looking up your ident...\r\n".try_into(), Ok(IrcMessage {
            tags: vec![],
            prefix: Some("*.freenode.net".to_string()),
            command: IrcCommand::Notice("*".to_string(), "*** Looking up your ident...".to_string()),
        }));

        assert_eq!("ERROR :Closing link: (~mct33@220.233.11.197) [Registration timeout]\r\n".try_into(), Ok(IrcMessage {
            tags: vec![],
            prefix: None,
            command: IrcCommand::ErrorMsg("Closing link: (~mct33@220.233.11.197) [Registration timeout]".to_string()),
        }));
    }

    #[test]
    fn to_string() {
        assert_eq!("LEAVE\r\n".to_string(), String::try_from(IrcMessage {
            tags: vec![],
            prefix: None,
            command: IrcCommand::Generic(GenericIrcCommand {
                command: GenericIrcCommandType::Text("LEAVE".to_string()),
                params: vec![],
            }),
        }).unwrap());

        assert_eq!(":server MSG #meme :11/10 cock\r\n".to_string(), String::try_from(IrcMessage {
            tags: vec![],
            prefix: Some("server".to_string()),
            command: IrcCommand::Generic(GenericIrcCommand {
                command: GenericIrcCommandType::Text("MSG".to_string()),
                params: vec!["#meme", "11/10 cock"].into_iter().map(|m| m.to_string()).collect(),
            }),
        }).unwrap());

        assert_eq!(":server 404 shit\r\n".to_string(), String::try_from(IrcMessage {
            tags: vec![],
            prefix: Some("server".to_string()),
            command: IrcCommand::Generic(GenericIrcCommand {
                command: GenericIrcCommandType::Number(404),
                params: vec!["shit".to_string()],
            }),
        }).unwrap());

        assert_eq!("@foo;bar;test_tag=plumbus :127.0.0.1 MSG #rust :rustaceans rise!\r\n".to_string(), String::try_from(IrcMessage {
            tags: vec![("foo".to_string(), None), ("bar".to_string(), None), ("test_tag".to_string(), Some("plumbus".to_string()))],
            prefix: Some("127.0.0.1".to_string()),
            command: IrcCommand::Generic(GenericIrcCommand {
                command: GenericIrcCommandType::Text("MSG".to_string()),
                params: vec!["#rust", "rustaceans rise!"].into_iter().map(|m| m.to_string()).collect(),
            }),
        }).unwrap());
    }

    #[test]
    fn message_variants() {
        assert_eq!(IrcCommand::Pass("password123".to_string()), GenericIrcCommand {
            command: GenericIrcCommandType::Text("PASS".to_string()),
            params: vec!["password123".to_string()],
        }.try_into().unwrap());

        assert_eq!(IrcCommand::Nick("Jimmy".to_string()), GenericIrcCommand {
            command: GenericIrcCommandType::Text("NICK".to_string()),
            params: vec!["Jimmy".to_string()],
        }.try_into().unwrap());

        assert_eq!(IrcCommand::User("Jim1982".to_string(), "James Bond".to_string()), GenericIrcCommand {
            command: GenericIrcCommandType::Text("USER".to_string()),
            params: vec!["Jim1982".to_string(), "James Bond".to_string()],
        }.try_into().unwrap());

        assert_eq!(String::try_from(IrcCommand::Pass("password123".to_string())).unwrap(), "PASS password123".to_string());

        assert_eq!(String::try_from(IrcCommand::Nick("Jimmy".to_string())).unwrap(), "NICK Jimmy".to_string());

        assert_eq!(String::try_from(IrcCommand::User("Jim1982".to_string(), "James Bond".to_string())).unwrap(), "USER Jim1982 0 * :James Bond".to_string());
    }
}
