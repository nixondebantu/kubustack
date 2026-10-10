//! Confirm and alert dialog wrappers.

use crate::format;
use fltk::dialog;

/// Asks `question` with Cancel and `action` buttons. Escape cancels.
pub fn confirm(question: &str, action: &str) -> bool {
    dialog::choice2_default(&format::label(question), "Cancel", action, "") == Some(1)
}

pub fn alert(message: &str) {
    dialog::alert_default(&format::label(message));
}
