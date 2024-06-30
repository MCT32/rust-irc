use crate::message::IrcCommand;

pub trait EventHandler: Send + Sync {
    fn on_raw_message(&self, message: IrcCommand) {
        let _ = message;
    }

    // Called on connect
    fn on_welcome(&self, message: String) {
        let _ = message;
    }
    fn on_your_host(&self, message: String) {
        let _ = message;
    }

    fn on_notice(&self, message: String) {
        let _ = message;
    }
}
