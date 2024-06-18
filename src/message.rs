use regex::Regex;

use crate::error::Error;

#[derive(Debug, PartialEq, Clone)]
pub struct GenericIrcMessage {
    pub prefix: Option<String>,
    // TODO: change type for error codes, prob use an enum
    pub command: String,
    pub params: Vec<String>,
}

impl TryFrom<&str> for GenericIrcMessage {
    type Error = Error;

    fn try_from(value: &str) -> Result<GenericIrcMessage, Error> {
        // TODO: remake regex to allow all possible params, with symbols and such
        let re = Regex::new("^(?::([A-Za-z0-9]+) )?([A-Z]+|[0-9]{3})(?: ([A-Za-z0-9: ]+))?$").unwrap();

        let Some(caps) = re.captures(value) else {
            return Err(Error::NoMatch(value.to_string()));
        };

        let prefix = caps.get(0).map(|m| m.as_str().to_string());

        let Some(command) = caps.get(1).map(|m| m.as_str().to_string()) else {
            return Err(Error::NoCommand(value.to_string()));
        };

        let params = match caps.get(2).map(|m| m.as_str()) {
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
            prefix,
            command,
            params,
        })
    }
}

impl From<GenericIrcMessage> for String {
    fn from(value: GenericIrcMessage) -> Self {
        let mut buffer = String::new();

        if let Some(prefix) = value.prefix {
            buffer.push_str(format!(":{} ", prefix).as_str());
        };

        buffer.push_str(value.command.as_str());

        // TODO: need to add support for trailing params
        for param in value.params {
            buffer.push_str(" ");
            buffer.push_str(param.as_str());
        };

        buffer
    }
}
