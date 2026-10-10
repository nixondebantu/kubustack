//! Confirm and alert dialog wrappers.

use crate::format;
use fltk::dialog;

/// Asks `question` with Cancel and `action` buttons. Escape cancels.
pub fn confirm(question: &str, action: &str) -> bool {
    dialog::choice2_default(&format::label(question), "Cancel", action, "") == Some(1)
}

/// Prune choice: `Some(true)` dangling only, `Some(false)` all unused, `None` cancelled.
pub fn prune_choice() -> Option<bool> {
    let question = "Remove unused images?\n\n\
        Dangling images are untagged leftovers from builds and pulls.\n\
        All unused images also removes tagged images that no container uses.";
    match dialog::choice2_default(
        question,
        "Cancel",
        "Remove dangling images",
        "Remove all unused images",
    ) {
        Some(1) => Some(true),
        Some(2) => Some(false),
        _ => None,
    }
}

pub fn alert(message: &str) {
    dialog::alert_default(&format::label(message));
}
