use crate::event::Event;

pub trait EventHandler: Send + Sync {
    // fn on_status_change(&self, ctx: Context) {
    //     let _ = ctx;
    // }

    // fn on_raw_message(&self, message: IrcMessage) {
    //     let _ = message;
    // }

    // // Called on connect
    // fn on_welcome(&self, message: String) {
    //     let _ = message;
    // }
    // fn on_your_host(&self, message: String) {
    //     let _ = message;
    // }

    // fn on_error(&self, message: String) {
    //     let _ = message;
    // }

    // fn on_notice(&self, message: String) {
    //     let _ = message;
    // }

    fn on_event(&self, event: Event) {
        let _ = event;
    }
}
