//! This module provides an interface to Lwt-specific mechanism to notify event
//! loop about some events happening from other threads. This is done by calling
//! `lwt_unix_send_notification` function. This has to be specific for
//! concurrency library used on OCaml side, as we need to send event from
//! potentially other thread which should wake up the event loop on OCaml side.

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
