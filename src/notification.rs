use crate::local_executor::Notifier;

extern "C" {
    pub fn lwt_unix_send_notification(id: isize);
}

#[derive(Debug, Copy, Clone)]
pub struct Notification(pub isize);

impl Notification {
    pub fn send(self) {
        unsafe { lwt_unix_send_notification(self.0) }
    }
}

impl Notifier for Notification {
    fn notify(&self) {
        self.send()
    }
}
