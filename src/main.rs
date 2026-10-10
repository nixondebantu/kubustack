//! KUBUSTACK entry point: argument parsing, app init, main loop, Msg dispatch.

mod docker;
mod format;
mod msg;
mod ui;
mod worker;

use docker::{api, events, DockerError};
use fltk::app::{self, Sender};
use msg::{Action, Msg, Scope};
use std::path::PathBuf;
use std::process::ExitCode;
use ui::{dialogs, main_window::MainWindow};
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
        note: None,
        listening: false,
        events_down: false,
        dirty: (false, false),
        settling: false,
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
    /// Last action progress or result, appended to the status bar until the next Refresh.
    note: Option<String>,
    /// The events listener thread has been started (it lives for the whole run).
    listening: bool,
    /// Live updates are paused: the events stream is down.
    events_down: bool,
    /// Lists (containers, images) changed by events since the last debounced refresh.
    dirty: (bool, bool),
    /// A debounce timer is pending.
    settling: bool,
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
                if !self.listening {
                    self.listening = true;
                    if let Some(socket) = self.socket.clone() {
                        events::spawn_listener(socket, self.s);
                    }
                }
            }
            Msg::Connected(Err(e)) => self.disconnected(e),
            Msg::Refresh => {
                self.note = None;
                self.refresh_all();
            }
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
            Msg::ImagesLoaded(result) => {
                let again = self.images_refresh.finish();
                match result {
                    Ok(list) if self.socket.is_some() => {
                        self.image_count = Some(list.len());
                        self.ui.images.set_images(list);
                    }
                    Ok(_) => return,
                    Err(e) => return self.disconnected(e),
                }
                if again {
                    self.refresh_images();
                }
                self.update_status();
            }
            Msg::ContainerSelected => self.ui.containers.on_select(),
            Msg::ContainerAction(a) => self.container_action(a),
            Msg::DockerEvent(scope) => {
                match scope {
                    Scope::Container => self.dirty.0 = true,
                    Scope::Image => self.dirty.1 = true,
                }
                // Debounce: one refresh for a burst of events (e.g. a prune).
                if !self.settling {
                    self.settling = true;
                    let s = self.s;
                    app::add_timeout3(0.3, move |_| s.send(Msg::EventsSettled));
                }
            }
            Msg::EventsSettled => {
                self.settling = false;
                let (containers, images) = std::mem::take(&mut self.dirty);
                if containers {
                    self.refresh_containers();
                }
                if images {
                    self.refresh_images();
                }
            }
            Msg::EventsConnected => {
                if self.events_down {
                    self.events_down = false;
                    // Docker may have come back while we showed the error panel.
                    if self.socket.is_some() {
                        self.refresh_all();
                    } else {
                        self.connect();
                    }
                }
            }
            Msg::EventsDisconnected => {
                if !self.events_down {
                    self.events_down = true;
                    // A refresh shows the error panel at once if Docker itself went away.
                    self.refresh_all();
                    self.update_status();
                }
            }
            Msg::ImageSelected => self.ui.images.on_select(),
            Msg::RemoveImage => self.remove_image(),
            Msg::PruneImages => self.prune_images(),
            Msg::DeleteKey => match self.ui.active_tab().as_deref() {
                Some("Containers") => self.container_action(Action::Remove),
                Some("Images") => self.remove_image(),
                _ => {}
            },
            Msg::ActionDone { failed, result } => {
                match result {
                    Ok(done) => self.note = Some(done),
                    Err(e) => {
                        self.note = Some(failed.clone());
                        dialogs::alert(&format!("{failed}: {e}"));
                    }
                }
                // Events refresh the lists; refresh by hand only while they're down.
                if self.events_down {
                    self.refresh_all();
                }
                self.update_status();
            }
        }
    }

    /// Runs an action on the selected container, confirming removal first.
    fn container_action(&mut self, action: Action) {
        let Some(socket) = self.socket.clone() else {
            return;
        };
        let Some(c) = self.ui.containers.selected() else {
            return;
        };
        let name = format::container_name(&c.names, &c.id);
        let id = c.id.clone();
        let running = c.state == "running";
        let (doing, done, verb) = match action {
            Action::Start => ("Starting", "Started", "start"),
            Action::Stop => ("Stopping", "Stopped", "stop"),
            Action::Restart => ("Restarting", "Restarted", "restart"),
            Action::Remove => {
                let question = if running {
                    format!("{name} is running. Stop and remove it?")
                } else {
                    format!("Remove container {name}?")
                };
                if !dialogs::confirm(&question, "Remove") {
                    return;
                }
                ("Removing", "Removed", "remove")
            }
        };
        self.note = Some(format!("{doing} {name}…"));
        self.update_status();
        worker::spawn(self.s, move || {
            let result = match action {
                Action::Start => api::start(&socket, &id),
                Action::Stop => api::stop(&socket, &id),
                Action::Restart => api::restart(&socket, &id),
                Action::Remove => api::remove(&socket, &id, running),
            };
            Msg::ActionDone {
                failed: format!("Couldn't {verb} {name}"),
                result: result.map(|()| format!("{done} {name}")),
            }
        });
    }

    /// Removes the selected image (untags it, or deletes an untagged image by ID) after confirming.
    fn remove_image(&mut self) {
        let Some(socket) = self.socket.clone() else {
            return;
        };
        let Some(row) = self.ui.images.selected() else {
            return;
        };
        let name = row.display_name();
        let reference = row.reference().to_string();
        if !dialogs::confirm(&format!("Remove image {name}?"), "Remove") {
            return;
        }
        self.note = Some(format!("Removing {name}…"));
        self.update_status();
        worker::spawn(self.s, move || Msg::ActionDone {
            failed: format!("Couldn't remove image {name}"),
            result: api::remove_image(&socket, &reference).map(|()| format!("Removed {name}")),
        });
    }

    fn prune_images(&mut self) {
        let Some(socket) = self.socket.clone() else {
            return;
        };
        let Some(dangling_only) = dialogs::prune_choice() else {
            return;
        };
        self.note = Some("Removing unused images…".into());
        self.update_status();
        worker::spawn(self.s, move || Msg::ActionDone {
            failed: "Couldn't remove unused images".into(),
            result: api::prune_images(&socket, dangling_only).map(|r| {
                let n = r
                    .images_deleted
                    .iter()
                    .filter(|i| i.deleted.is_some())
                    .count();
                format!(
                    "Removed {n} images, reclaimed {}",
                    format::size(r.space_reclaimed)
                )
            }),
        });
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
            worker::spawn(self.s, move || Msg::ImagesLoaded(api::list_images(&socket)));
        }
    }

    fn update_status(&mut self) {
        if self.socket.is_none() {
            return;
        }
        let (n, running) = self.ui.containers.counts();
        let mut text = format!(
            "Connected · Docker {} · {n} containers ({running} running)",
            self.version
        );
        if let Some(m) = self.image_count {
            text += &format!(" · {m} images");
        }
        if let Some(note) = &self.note {
            text += &format!(" · {note}");
        }
        if self.events_down {
            text += " · live updates paused";
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
        println!("Images:     {}", api::list_images(&socket)?.len());
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
