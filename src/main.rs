//! KUBUSTACK entry point: argument parsing, app init, main loop, Msg dispatch.

mod docker;
mod format;
mod msg;
mod ui;
mod worker;

use docker::{api, DockerError};
use fltk::app::{self, Sender};
use msg::Msg;
use std::path::PathBuf;
use std::process::ExitCode;
use ui::main_window::MainWindow;
use worker::Refresh;

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
    let (s, r) = app::channel::<Msg>();
    let mut state = App {
        ui: MainWindow::new(s),
        s,
        socket: None,
        version: String::new(),
        image_count: None,
        containers_refresh: Refresh::default(),
        images_refresh: Refresh::default(),
    };
    state.connect();
    while app.wait() {
        while let Some(msg) = r.recv() {
            state.handle(msg);
        }
    }
    ExitCode::SUCCESS
}

/// UI-thread state. Only touched from the main loop.
struct App {
    ui: MainWindow,
    s: Sender<Msg>,
    /// `Some` while connected.
    socket: Option<PathBuf>,
    version: String,
    image_count: Option<usize>,
    containers_refresh: Refresh,
    images_refresh: Refresh,
}

impl App {
    fn handle(&mut self, msg: Msg) {
        match msg {
            Msg::Connected(Ok((socket, v))) => {
                self.socket = Some(socket);
                self.version = v.version;
                self.ui.show_tabs();
                self.refresh_all();
            }
            Msg::Connected(Err(e)) => self.disconnected(e),
            Msg::Refresh => self.refresh_all(),
            Msg::Retry => self.connect(),
            Msg::ContainersLoaded(result) => {
                let again = self.containers_refresh.finish();
                match result {
                    Ok(list) if self.socket.is_some() => self.ui.containers.set_containers(list),
                    Ok(_) => return,
                    Err(e) => return self.disconnected(e),
                }
                if again {
                    self.refresh_containers();
                }
                self.update_status();
            }
            Msg::ImagesCounted(result) => {
                let again = self.images_refresh.finish();
                match result {
                    Ok(n) if self.socket.is_some() => self.image_count = Some(n),
                    Ok(_) => return,
                    Err(e) => return self.disconnected(e),
                }
                if again {
                    self.refresh_images();
                }
                self.update_status();
            }
            Msg::ContainerSelected => self.ui.containers.on_select(),
        }
    }

    fn connect(&mut self) {
        self.ui.set_status("Connecting…");
        worker::spawn(self.s, || {
            Msg::Connected(docker::socket_path().and_then(|p| api::version(&p).map(|v| (p, v))))
        });
    }

    fn disconnected(&mut self, e: DockerError) {
        self.socket = None;
        self.image_count = None;
        self.ui.show_error(&e.to_string());
        self.ui.set_status("Not connected");
    }

    fn refresh_all(&mut self) {
        self.refresh_containers();
        self.refresh_images();
    }

    fn refresh_containers(&mut self) {
        let Some(socket) = self.socket.clone() else {
            return;
        };
        if self.containers_refresh.start() {
            worker::spawn(self.s, move || {
                Msg::ContainersLoaded(api::list_containers(&socket))
            });
        }
    }

    fn refresh_images(&mut self) {
        let Some(socket) = self.socket.clone() else {
            return;
        };
        if self.images_refresh.start() {
            worker::spawn(self.s, move || {
                Msg::ImagesCounted(api::count_images(&socket))
            });
        }
    }

    fn update_status(&mut self) {
        let (n, running) = self.ui.containers.counts();
        let mut text = format!(
            "Connected · Docker {} · {n} containers ({running} running)",
            self.version
        );
        if let Some(m) = self.image_count {
            text += &format!(" · {m} images");
        }
        self.ui.set_status(&text);
    }
}

/// Headless connection check (SPEC §9).
fn check() -> ExitCode {
    let result = docker::socket_path().and_then(|socket| {
        println!("Socket:     {}", socket.display());
        let v = api::version(&socket)?;
        println!("Docker:     {} (API {})", v.version, v.api_version);
        let containers = api::list_containers(&socket)?;
        let running = containers.iter().filter(|c| c.state == "running").count();
        println!("Containers: {} ({running} running)", containers.len());
        println!("Images:     {}", api::count_images(&socket)?);
        Ok(())
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
