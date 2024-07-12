use std::vec;

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
    RplCreated(String, String), // 003 RPL_CREATED
    RplMyInfo {
        client: String,
        server_name: String,
        server_version: String,
        umodes: String,
        cmodes: String,
        cmodes_params: Option<String>,
    }, // 004 RPL_MYINFO
    // TODO: Add struct for caps
    RplISupport(String, Vec<String>, String), // 005 RPL_ISUPPORT

    RplLUserClient(String, String), // 251 RPL_LUSERCLIENT
    RplLUserOp(String, u32, String), // 252 RPL_LUSEROPS
    RplLUserUnknown(String, u32, String), // 253 RPL_LUSERUNKNOWN
    RplLUserChannels(String, u32, String), // 254 RPL_LUSERCHANNELS
    RplLUserMe(String, String), // 255 RPL_LUSERME

    RplLocalUsers(String, Option<(u32, u32)>, String), // 265 RPL_LOCALUSERS
    RplGlobalUsers(String, Option<(u32, u32)>, String), // 266 RPL_GLOBALUSERS

    RplMotdStart(String, String), // 375 RPL_MOTDSTART
    RplMotd(String, String), // 372 RPL_MOTD
    RplEndOfMotd(String, String), // 376 RPL_ENDOFMOTD

    // TODO: Figure out what this is
    RplHostHidden(String, String, String), // 396 RPL_HOSTHIDDEN

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
                    "PING" => Ok(Self::Ping(value.trailing.unwrap())),
                    "PONG" => Ok(Self::Pong(value.trailing.unwrap())),
                    "NOTICE" => Ok(Self::Notice(value.params.get(0).unwrap().clone(), value.trailing.unwrap())),
                    "ERROR" => Ok(Self::ErrorMsg(value.trailing.unwrap())),
                    _ => {
                        #[cfg(debug_assertions)]
                        {
                            eprintln!("Unknown command: {:?}", value.command);
                        }

                        Ok(Self::Generic(value))
                    },
                }
            },
            GenericIrcCommandType::Number(command) => {
                match command {
                    001 => Ok(Self::RplWelcome(value.params.get(0).unwrap().clone(), value.trailing.unwrap())),
                    002 => Ok(Self::RplYourHost(value.params.get(0).unwrap().clone(), value.trailing.unwrap())),
                    003 => Ok(Self::RplCreated(value.params.get(0).unwrap().clone(), value.trailing.unwrap())),
                    004 => Ok(Self::RplMyInfo{
                        client: value.params.get(0).unwrap().clone(),
                        server_name: value.params.get(1).unwrap().clone(),
                        server_version: value.params.get(2).unwrap().clone(),
                        // TODO: Parse umodes and cmodes with their own struct
                        umodes: value.params.get(3).unwrap().clone(),
                        cmodes: value.params.get(4).unwrap().clone(),
                        cmodes_params: value.params.get(5).map(|m| m.clone()),
                    }),
                    005 => Ok(Self::RplISupport(value.params.get(0).unwrap().clone(), value.params.into_iter().skip(1).collect(), value.trailing.unwrap())),
                    251 => Ok(Self::RplLUserClient(value.params.get(0).unwrap().clone(), value.trailing.unwrap())),
                    252 => Ok(Self::RplLUserOp(value.params.get(0).unwrap().clone(), value.params.get(1).unwrap().parse::<u32>().unwrap(), value.trailing.unwrap())),
                    253 => Ok(Self::RplLUserUnknown(value.params.get(0).unwrap().clone(), value.params.get(1).unwrap().parse::<u32>().unwrap(), value.trailing.unwrap())),
                    254 => Ok(Self::RplLUserChannels(value.params.get(0).unwrap().clone(), value.params.get(1).unwrap().parse::<u32>().unwrap(), value.trailing.unwrap())),
                    255 => Ok(Self::RplLUserMe(value.params.get(0).unwrap().clone(), value.trailing.unwrap())),
                    265 => {
                        if value.params.len() == 1 {
                            Ok(Self::RplLocalUsers(value.params.get(0).unwrap().clone(), None, value.trailing.unwrap()))
                        } else if value.params.len() == 3 {
                            Ok(Self::RplLocalUsers(value.params.get(0).unwrap().clone(), Some((value.params.get(1).unwrap().parse::<u32>().unwrap(), value.params.get(2).unwrap().parse::<u32>().unwrap())), value.trailing.unwrap()))
                        } else {
                            Err(Error::Invalid)
                        }
                    },
                    266 => {
                        if value.params.len() == 1 {
                            Ok(Self::RplGlobalUsers(value.params.get(0).unwrap().clone(), None, value.trailing.unwrap()))
                        } else if value.params.len() == 3 {
                            Ok(Self::RplGlobalUsers(value.params.get(0).unwrap().clone(), Some((value.params.get(1).unwrap().parse::<u32>().unwrap(), value.params.get(2).unwrap().parse::<u32>().unwrap())), value.trailing.unwrap()))
                        } else {
                            Err(Error::Invalid)
                        }
                    },
                    375 => Ok(Self::RplMotdStart(value.params.get(0).unwrap().clone(), value.trailing.unwrap())),
                    372 => Ok(Self::RplMotd(value.params.get(0).unwrap().clone(), value.trailing.unwrap())),
                    376 => Ok(Self::RplEndOfMotd(value.params.get(0).unwrap().clone(), value.trailing.unwrap())),
                    396 => Ok(Self::RplHostHidden(value.params.get(0).unwrap().clone(), value.params.get(1).unwrap().clone(), value.trailing.unwrap())),
                    _ => {
                        #[cfg(debug_assertions)]
                        {
                            eprintln!("Unknown command: {:?}", value.command);
                        }

                        Ok(Self::Generic(value))
                    },
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
                trailing: None,
            },
            IrcCommand::Nick(nickname) => GenericIrcCommand {
                command: GenericIrcCommandType::Text("NICK".to_string()),
                params: vec![nickname],
                trailing: None,
            },
            IrcCommand::User(username, realname) => GenericIrcCommand {
                command: GenericIrcCommandType::Text("USER".to_string()),
                params: vec![username, "0".to_string(), "*".to_string(), realname],
                trailing: None,
            },
            IrcCommand::Ping(message) => GenericIrcCommand {
                command: GenericIrcCommandType::Text("PING".to_string()),
                params: vec![],
                trailing: Some(message),
            },
            IrcCommand::Pong(message) => GenericIrcCommand {
                command: GenericIrcCommandType::Text("PONG".to_string()),
                params: vec![],
                trailing: Some(message),
            },
            IrcCommand::Notice(target, message) => GenericIrcCommand {
                command: GenericIrcCommandType::Text("NOTICE".to_string()),
                params: vec![target],
                trailing: Some(message),
            },
            IrcCommand::ErrorMsg(message) => GenericIrcCommand {
                command: GenericIrcCommandType::Text("ERROR".to_string()),
                params: vec![],
                trailing: Some(message),
            },

            IrcCommand::RplWelcome(client, message) => GenericIrcCommand {
                command: GenericIrcCommandType::Number(001),
                params: vec![client],
                trailing: Some(message),
            },
            IrcCommand::RplYourHost(client, message) => GenericIrcCommand {
                command: GenericIrcCommandType::Number(002),
                params: vec![client],
                trailing: Some(message),
            },
            IrcCommand::RplCreated(client, message) => GenericIrcCommand {
                command: GenericIrcCommandType::Number(003),
                params: vec![client],
                trailing: Some(message),
            },
            IrcCommand::RplMyInfo {
                client,
                server_name: servername,
                server_version: version,
                umodes,
                cmodes,
                cmodes_params
            } => GenericIrcCommand {
                command: GenericIrcCommandType::Number(004),
                params: if let Some(cmodes_params) = cmodes_params {
                    vec![client, servername, version, umodes, cmodes, cmodes_params]
                } else {
                    vec![client, servername, version, umodes, cmodes]
                },
                trailing: None,
            },
            IrcCommand::RplISupport(client, caps, message) => {
                let mut params = vec![client];
                params.extend(caps);

                GenericIrcCommand {
                    command: GenericIrcCommandType::Number(005),
                    params,
                    trailing: Some(message),
                }
            },

            IrcCommand::RplLUserClient(client, message) => {
                GenericIrcCommand {
                    command: GenericIrcCommandType::Number(251),
                    params: vec![client],
                    trailing: Some(message),
                }
            }
            IrcCommand::RplLUserOp(client, ops, message) => {
                GenericIrcCommand {
                    command: GenericIrcCommandType::Number(252),
                    params: vec![client, ops.to_string()],
                    trailing: Some(message),
                }
            }
            IrcCommand::RplLUserUnknown(client, connections, message) => {
                GenericIrcCommand {
                    command: GenericIrcCommandType::Number(253),
                    params: vec![client, connections.to_string()],
                    trailing: Some(message),
                }
            },
            IrcCommand::RplLUserChannels(client, channels, message) => {
                GenericIrcCommand {
                    command: GenericIrcCommandType::Number(254),
                    params: vec![client, channels.to_string()],
                    trailing: Some(message),
                }
            },
            IrcCommand::RplLUserMe(client, message) => {
                GenericIrcCommand {
                    command: GenericIrcCommandType::Number(255),
                    params: vec![client],
                    trailing: Some(message),
                }
            },

            IrcCommand::RplLocalUsers(client, users, message) => {
                GenericIrcCommand {
                    command: GenericIrcCommandType::Number(265),
                    params: match users {
                        None => vec![client],
                        Some((current, max)) => vec![client, current.to_string(), max.to_string()],
                    },
                    trailing: Some(message),
                }
            },
            IrcCommand::RplGlobalUsers(client, users, message) => {
                GenericIrcCommand {
                    command: GenericIrcCommandType::Number(266),
                    params: match users {
                        None => vec![client],
                        Some((current, max)) => vec![client, current.to_string(), max.to_string()],
                    },
                    trailing: Some(message),
                }
            },

            IrcCommand::RplMotdStart(client, message) => {
                GenericIrcCommand {
                    command: GenericIrcCommandType::Number(375),
                    params: vec![client],
                    trailing: Some(message),
                }
            },
            IrcCommand::RplMotd(client, message) => {
                GenericIrcCommand {
                    command: GenericIrcCommandType::Number(372),
                    params: vec![client],
                    trailing: Some(message),
                }
            },
            IrcCommand::RplEndOfMotd(client, message) => {
                GenericIrcCommand {
                    command: GenericIrcCommandType::Number(376),
                    params: vec![client],
                    trailing: Some(message),
                }
            },

            IrcCommand::RplHostHidden(client, host, message) => {
                GenericIrcCommand {
                    command: GenericIrcCommandType::Number(396),
                    params: vec![client, host],
                    trailing: Some(message),
                }
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
    // TODO: Store trailing seperately
    pub params: Vec<String>,
    pub trailing: Option<String>,
}

impl TryFrom<&str> for GenericIrcCommand {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let re = Regex::new("^([A-Z]+|[0-9]{3})((?: (?:[^:\\n\\r\\x00 ][^\\n\\r\\x00 ]*))*)?(?: :([^\\n\\r]+))?$").unwrap();

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
                let trimmed = params.trim_start();

                if trimmed.is_empty() {
                    vec![]
                } else {
                    trimmed.split(' ').into_iter().collect::<Vec<_>>()
                }
            }
        }.into_iter().map(|m| m.to_string()).collect();

        let trailing = caps.get(3).map(|m| m.as_str().to_string());
        
        Ok(GenericIrcCommand {
            command,
            params,
            trailing,
        })
    }
}

impl TryFrom<GenericIrcCommand> for String {
    type Error = Error;

    fn try_from(value: GenericIrcCommand) -> Result<Self, Error> {
        let mut buffer = String::new();

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

        if let Some(trailing) = value.trailing {
            buffer.push_str(format!(" :{}", trailing).as_str());
        }

        Ok(buffer)
    }
}



// TODO: May be overkill, but consider adding a test for every message type
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
                trailing: None,
            }),
        }));

        assert_eq!(":server PRIVMSG #meme :11/10 cock\r\n".try_into(), Ok(IrcMessage {
            tags: vec![],
            prefix: Some("server".to_string()),
            command: IrcCommand::Generic(GenericIrcCommand {
                command: GenericIrcCommandType::Text("PRIVMSG".to_string()),
                params: vec!["#meme".to_string()],
                trailing: Some("11/10 cock".to_string()),
            }),
        }));

        assert_eq!(":server 404 :shit\r\n".try_into(), Ok(IrcMessage {
            tags: vec![],
            prefix: Some("server".to_string()),
            command: IrcCommand::Generic(GenericIrcCommand {
                command: GenericIrcCommandType::Number(404),
                params: vec![],
                trailing: Some("shit".to_string()),
            }),
        }));

        assert_eq!("@foo;bar;test_tag=plumbus :127.0.0.1 MSG #rust :rustaceans rise!\r\n".try_into(), Ok(IrcMessage {
            tags: vec![("foo".to_string(), None), ("bar".to_string(), None), ("test_tag".to_string(), Some("plumbus".to_string()))],
            prefix: Some("127.0.0.1".to_string()),
            command: IrcCommand::Generic(GenericIrcCommand {
                command: GenericIrcCommandType::Text("MSG".to_string()),
                params: vec!["#rust".to_string()],
                trailing: Some("rustaceans rise!".to_string()),
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
                trailing: None,
            }),
        }).unwrap());

        assert_eq!(":server MSG #meme :11/10 cock\r\n".to_string(), String::try_from(IrcMessage {
            tags: vec![],
            prefix: Some("server".to_string()),
            command: IrcCommand::Generic(GenericIrcCommand {
                command: GenericIrcCommandType::Text("MSG".to_string()),
                params: vec!["#meme".to_string()],
                trailing: Some("11/10 cock".to_string()),
            }),
        }).unwrap());

        assert_eq!(":server 404 :shit\r\n".to_string(), String::try_from(IrcMessage {
            tags: vec![],
            prefix: Some("server".to_string()),
            command: IrcCommand::Generic(GenericIrcCommand {
                command: GenericIrcCommandType::Number(404),
                params: vec![],
                trailing: Some("shit".to_string()),
            }),
        }).unwrap());

        assert_eq!("@foo;bar;test_tag=plumbus :127.0.0.1 MSG #rust :rustaceans rise!\r\n".to_string(), String::try_from(IrcMessage {
            tags: vec![("foo".to_string(), None), ("bar".to_string(), None), ("test_tag".to_string(), Some("plumbus".to_string()))],
            prefix: Some("127.0.0.1".to_string()),
            command: IrcCommand::Generic(GenericIrcCommand {
                command: GenericIrcCommandType::Text("MSG".to_string()),
                params: vec!["#rust".to_string()],
                trailing: Some("rustaceans rise!".to_string()),
            }),
        }).unwrap());
    }

    #[test]
    fn message_variants() {
        assert_eq!(IrcCommand::Pass("password123".to_string()), GenericIrcCommand {
            command: GenericIrcCommandType::Text("PASS".to_string()),
            params: vec!["password123".to_string()],
            trailing: None,
        }.try_into().unwrap());

        assert_eq!(IrcCommand::Nick("Jimmy".to_string()), GenericIrcCommand {
            command: GenericIrcCommandType::Text("NICK".to_string()),
            params: vec!["Jimmy".to_string()],
            trailing: None,
        }.try_into().unwrap());

        assert_eq!(IrcCommand::User("Jim1982".to_string(), "James Bond".to_string()), GenericIrcCommand {
            command: GenericIrcCommandType::Text("USER".to_string()),
            params: vec!["Jim1982".to_string(), "James Bond".to_string()],
            trailing: None,
        }.try_into().unwrap());

        assert_eq!(String::try_from(IrcCommand::Pass("password123".to_string())).unwrap(), "PASS password123".to_string());

        assert_eq!(String::try_from(IrcCommand::Nick("Jimmy".to_string())).unwrap(), "NICK Jimmy".to_string());

        assert_eq!(String::try_from(IrcCommand::User("Jim1982".to_string(), "James Bond".to_string())).unwrap(), "USER Jim1982 0 * :James Bond".to_string());
    }
}
