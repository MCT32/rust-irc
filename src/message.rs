use regex::Regex;

use crate::error::Error;

#[derive(Debug, PartialEq, Clone)]
pub enum IrcMessage {
    PASS(String),
    NICK(String),
    USER{
        username: String,
        realname: String,
    },
    Generic(GenericIrcMessage),
}

impl TryFrom<GenericIrcMessage> for IrcMessage {
    type Error = Error;

    fn try_from(value: GenericIrcMessage) -> Result<Self, Error> {
        match &value.command {
            GenericIrcCommand::Text(command) => {
                match command.as_str() {
                    "PASS" => Ok(Self::PASS(value.params.get(0).unwrap().clone())),
                    "NICK" => Ok(Self::NICK(value.params.get(0).unwrap().clone())),
                    "USER" => Ok(Self::USER{ username: value.params.get(0).unwrap().clone(),
                        realname: value.params.get(1).unwrap().clone() }),
                    _ => Ok(Self::Generic(value)),
                }
            },
            GenericIrcCommand::Number(_) => {
                Ok(Self::Generic(value))
            },
        }
    }
}

impl TryFrom<&str> for IrcMessage {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        GenericIrcMessage::try_from(value)?.try_into()
    }
}

impl From<IrcMessage> for GenericIrcMessage {
    fn from(value: IrcMessage) -> Self {
        match value {
            IrcMessage::PASS(password) => GenericIrcMessage {
                // TODO: allow IrcMessage to carry tags and prefixes
                tags: vec![],
                prefix: None,
                command: GenericIrcCommand::Text("PASS".to_string()),
                params: vec![password],
            },
            IrcMessage::NICK(nickname) => GenericIrcMessage {
                tags: vec![],
                prefix: None,
                command: GenericIrcCommand::Text("NICK".to_string()),
                params: vec![nickname],
            },
            IrcMessage::USER{ username, realname } => GenericIrcMessage {
                tags: vec![],
                prefix: None,
                command: GenericIrcCommand::Text("USER".to_string()),
                params: vec![username, "0".to_string(), "*".to_string(), realname],
            },
            IrcMessage::Generic(generic) => generic,
        }
    }
}

impl TryFrom<IrcMessage> for String {
    type Error = Error;

    fn try_from(value: IrcMessage) -> Result<Self, Error> {
        GenericIrcMessage::from(value).try_into()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum GenericIrcCommand {
    Text(String),
    Number(u16),
}

impl TryFrom<&str> for GenericIrcCommand {
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

impl From<GenericIrcCommand> for String {
    fn from(value: GenericIrcCommand) -> Self {
        match value {
            GenericIrcCommand::Number(number) => number.to_string(),
            GenericIrcCommand::Text(text) => text,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct GenericIrcMessage {
    pub tags: Vec<(String, Option<String>)>,
    pub prefix: Option<String>,
    pub command: GenericIrcCommand,
    pub params: Vec<String>,
}

impl TryFrom<&str> for GenericIrcMessage {
    type Error = Error;

    fn try_from(value: &str) -> Result<GenericIrcMessage, Error> {
        let re = Regex::new("^(?:@([^\\n\\r\\x00 ]+) )?(?::([^\\r\\n\\x00 ]+) )?([A-Z]+|[0-9]{3})(?: ([^\\n\\r\\x00]+))?\\r\\n$").unwrap();

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

        let Ok(command) = GenericIrcCommand::try_from(command) else {
            return Err(Error::Invalid);
        };

        let params = match caps.get(4).map(|m| m.as_str()) {
            None => vec![],
            Some(params) => {
                match params.split_once(" :") {
                    Some((params, trailing)) => {
                        // TODO: Surely better way to do this
                        let mut params = params.split(' ').into_iter().collect::<Vec<_>>();
                        params.append(&mut vec![trailing]);
                        params
                    },
                    None => {
                        params.split(' ').into_iter().collect::<Vec<_>>()
                    }
                }
            },
        }.into_iter().map(|m| m.to_string()).collect();

        Ok(GenericIrcMessage {
            tags,
            prefix,
            command,
            params,
        })
    }
}

impl TryFrom<GenericIrcMessage> for String {
    type Error = Error;

    fn try_from(value: GenericIrcMessage) -> Result<Self, Error> {
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

        buffer.push_str("\r\n");

        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_string() {
        assert_eq!("LEAVE".try_into(), Ok(GenericIrcMessage {
            tags: vec![],
            prefix: None,
            command: GenericIrcCommand::Text("LEAVE".to_string()),
            params: vec![],
        }));

        assert_eq!(":server MSG #meme :11/10 cock".try_into(), Ok(GenericIrcMessage {
            tags: vec![],
            prefix: Some("server".to_string()),
            command: GenericIrcCommand::Text("MSG".to_string()),
            params: vec!["#meme", "11/10 cock"].into_iter().map(|m| m.to_string()).collect(),
        }));

        assert_eq!(":server 404 shit".try_into(), Ok(GenericIrcMessage {
            tags: vec![],
            prefix: Some("server".to_string()),
            command: GenericIrcCommand::Number(404),
            params: vec!["shit".to_string()],
        }));

        assert_eq!("@foo;bar;test_tag=plumbus :127.0.0.1 MSG #rust :rustaceans rise!".try_into(), Ok(GenericIrcMessage {
            tags: vec![("foo".to_string(), None), ("bar".to_string(), None), ("test_tag".to_string(), Some("plumbus".to_string()))],
            prefix: Some("127.0.0.1".to_string()),
            command: GenericIrcCommand::Text("MSG".to_string()),
            params: vec!["#rust", "rustaceans rise!"].into_iter().map(|m| m.to_string()).collect(),
        }));
    }

    #[test]
    fn to_string() {
        assert_eq!("LEAVE".to_string(), String::try_from(GenericIrcMessage {
            tags: vec![],
            prefix: None,
            command: GenericIrcCommand::Text("LEAVE".to_string()),
            params: vec![],
        }).unwrap());

        assert_eq!(":server MSG #meme :11/10 cock".to_string(), String::try_from(GenericIrcMessage {
            tags: vec![],
            prefix: Some("server".to_string()),
            command: GenericIrcCommand::Text("MSG".to_string()),
            params: vec!["#meme", "11/10 cock"].into_iter().map(|m| m.to_string()).collect(),
        }).unwrap());

        assert_eq!(":server 404 shit".to_string(), String::try_from(GenericIrcMessage {
            tags: vec![],
            prefix: Some("server".to_string()),
            command: GenericIrcCommand::Number(404),
            params: vec!["shit".to_string()],
        }).unwrap());

        assert_eq!("@foo;bar;test_tag=plumbus :127.0.0.1 MSG #rust :rustaceans rise!".to_string(), String::try_from(GenericIrcMessage {
            tags: vec![("foo".to_string(), None), ("bar".to_string(), None), ("test_tag".to_string(), Some("plumbus".to_string()))],
            prefix: Some("127.0.0.1".to_string()),
            command: GenericIrcCommand::Text("MSG".to_string()),
            params: vec!["#rust", "rustaceans rise!"].into_iter().map(|m| m.to_string()).collect(),
        }).unwrap());
    }

    #[test]
    fn message_variants() {
        assert_eq!(IrcMessage::PASS("password123".to_string()), GenericIrcMessage {
            tags: vec![],
            prefix: None,
            command: GenericIrcCommand::Text("PASS".to_string()),
            params: vec!["password123".to_string()],
        }.try_into().unwrap());

        assert_eq!(IrcMessage::NICK("Jimmy".to_string()), GenericIrcMessage {
            tags: vec![],
            prefix: None,
            command: GenericIrcCommand::Text("NICK".to_string()),
            params: vec!["Jimmy".to_string()],
        }.try_into().unwrap());

        assert_eq!(IrcMessage::USER{ username: "Jim1982".to_string(), realname: "James Bond".to_string() }, GenericIrcMessage {
            tags: vec![],
            prefix: None,
            command: GenericIrcCommand::Text("USER".to_string()),
            params: vec!["Jim1982".to_string(), "James Bond".to_string()],
        }.try_into().unwrap());

        assert_eq!(String::try_from(IrcMessage::PASS("password123".to_string())).unwrap(), "PASS password123".to_string());

        assert_eq!(String::try_from(IrcMessage::NICK("Jimmy".to_string())).unwrap(), "NICK Jimmy".to_string());

        assert_eq!(String::try_from(IrcMessage::USER{ username: "Jim1982".to_string(), realname: "James Bond".to_string() }).unwrap(), "USER Jim1982 0 * :James Bond".to_string());
    }
}
