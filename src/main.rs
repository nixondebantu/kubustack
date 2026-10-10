//! KUBUSTACK entry point: argument parsing, app init, main loop.

mod docker;
mod format;
mod msg;
mod ui;
mod worker;

use fltk::app;

fn main() {
    let app = app::App::default().with_scheme(app::Scheme::Gtk);
    ui::main_window::show();
    // ponytail: plain app.run() until M3 adds the channel-draining loop.
    if let Err(e) = app.run() {
        eprintln!("kubustack: {e}");
    }
}
