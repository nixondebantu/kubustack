//! KUBUSTACK entry point: argument parsing, app init, main loop.

mod docker;
mod format;
mod msg;
mod ui;
mod worker;

fn main() {
    println!("kubustack {}: work in progress", env!("CARGO_PKG_VERSION"));
}
