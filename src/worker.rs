//! Helpers to spawn short-lived job threads that send Msg results.

use crate::msg::Msg;
use fltk::app::Sender;

/// Runs `job` on a new thread and sends its result to the UI thread.
pub fn spawn(s: Sender<Msg>, job: impl FnOnce() -> Msg + Send + 'static) {
    // ponytail: if the OS refuses a thread, the job is dropped and logged; nothing retries it.
    if let Err(e) = std::thread::Builder::new().spawn(move || s.send(job())) {
        eprintln!("kubustack: couldn't start a background job: {e}");
    }
}

/// At most one in-flight refresh per list; a request during one runs once more afterwards.
#[derive(Default)]
pub struct Refresh {
    busy: bool,
    again: bool,
}

impl Refresh {
    /// Returns true if the caller should start a refresh now.
    pub fn start(&mut self) -> bool {
        if self.busy {
            self.again = true;
            return false;
        }
        self.busy = true;
        true
    }

    /// Marks the refresh finished. Returns true if another one was requested meanwhile.
    pub fn finish(&mut self) -> bool {
        self.busy = false;
        std::mem::take(&mut self.again)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalesces_requests() {
        let mut r = Refresh::default();
        assert!(r.start());
        assert!(!r.start()); // in flight: queued
        assert!(!r.start()); // still one queued, not two
        assert!(r.finish()); // run once more
        assert!(r.start());
        assert!(!r.finish()); // nothing queued
        assert!(r.start());
    }
}
