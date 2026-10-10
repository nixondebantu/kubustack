//! KUBUSTACK entry point: argument parsing, app init, main loop.

mod docker;
mod format;
mod msg;
mod ui;
mod worker;

use fltk::app;
use std::process::ExitCode;

const USAGE: &str = "\
Usage: kubustack [OPTION]

Lightweight viewer and manager for Docker containers and images.

Options:
  --check     Check the Docker connection without opening a window
  --version   Print the version and exit
  --help      Print this help and exit

Environment:
  DOCKER_HOST  unix:///path/to/docker.sock to use a different socket";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None => run_gui(),
        Some("--version") => {
            println!("kubustack {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("--help" | "-h") => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some("--check") => check(),
        Some(other) => {
            eprintln!("kubustack: unknown option '{other}'\n\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn run_gui() -> ExitCode {
    let app = app::App::default().with_scheme(app::Scheme::Gtk);
    ui::main_window::show();
    // ponytail: plain app.run() until M3 adds the channel-draining loop.
    if let Err(e) = app.run() {
        eprintln!("kubustack: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// Headless connection check (SPEC §9).
fn check() -> ExitCode {
    let result = docker::socket_path().and_then(|socket| {
        println!("Socket:     {}", socket.display());
        docker::api::version(&socket)
    });
    match result {
        Ok(v) => {
            println!("Docker:     {} (API {})", v.version, v.api_version);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
