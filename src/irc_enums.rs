pub enum IrcEvent<'a> {
    ReceiveMessage(&'a str),
}

pub enum IrcCommand<'a> {
    SendMessage(&'a str),
}
