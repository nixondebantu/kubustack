# KUBUSTACK — Specification (v1)

## 1. Overview

KUBUSTACK is a native Linux desktop application for quickly seeing and managing the Docker
containers and images on the local machine. It is inspired by OrbStack's convenience but
deliberately minimal: one small window, two tabs, plain widgets, near-zero idle resource use,
and installation by copying a single binary.

## 2. Goals and non-goals

### Goals (v1)

- List all containers (running and stopped) and all images.
- Start, stop, restart, and remove containers.
- View container logs, with optional live follow.
- Remove images and prune unused images.
- Update lists automatically when Docker state changes, without polling.
- Run as one self-contained binary with no runtime dependencies beyond libraries every Linux
  desktop already has.
- Install and update with one command, without `sudo`.

### Non-goals (v1)

- Live CPU/memory stats, volumes, networks, pulling images, Docker Compose grouping,
  exec/terminal into containers (all planned for v2, see §13).
- Remote Docker hosts (TCP/SSH).
- Theming, dark mode, tray icon, settings file.
- Kubernetes support of any kind.
- ARM builds.

## 3. Target platform and performance budget

| Item | Target |
|---|---|
| Architecture | x86_64 Linux |
| C library | glibc ≥ 2.35 (built on Ubuntu 22.04: runs on Ubuntu 22.04+, Debian 12+, Fedora 36+, current Arch, Mint 21+) |
| Display | X11; Wayland via XWayland (native Wayland rejected in the M1 spike, see §14 #23) |
| Docker | Engine API ≥ 1.41 (Docker 20.10+) |
| Podman | Best effort, through its Docker-compatible socket via `DOCKER_HOST` |
| Stripped release binary | ≤ 8 MB (expected 3–6 MB) |
| Idle memory (RSS, 50 containers listed) | ≤ 30 MB |
| Idle CPU | ~0%: no timers or polling while nothing happens |
| Cold start to first populated list | < 500 ms |
| Threads while idle | Main thread + events listener only |

The Milestone 1 spike measures real numbers and records them in the Decision Log (§14).

## 4. Tech stack and dependency policy

- **Language:** Rust stable, edition 2021.
- **GUI:** `fltk` (fltk-rs), latest 1.x at implementation time, FLTK statically linked.
  - Optional features (`use-wayland`, `no-pango`) were decided in Milestone 1 (§14 #23: neither
    is enabled) using this rule:
    enable a feature only if the binary still starts on both an X11 and a Wayland session
    and `ldd` shows no runtime libraries beyond those present on a stock desktop install.
- **JSON:** `serde` (with `derive`) and `serde_json`.
- **Everything else:** standard library (`std::os::unix::net::UnixStream`, `std::thread`,
  `std::time::SystemTime`, `std::env::args`).

Allowed crates are exactly `fltk`, `serde`, `serde_json`. Adding any other crate requires user
approval and a Decision Log entry.

`Cargo.toml` release profile:

```toml
[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

## 5. Architecture

```
┌──────────────────────────── main thread ────────────────────────────┐
│  FLTK event loop                                                     │
│  ├─ owns all widgets and app state (lists, selection, connection)    │
│  └─ receives Msg values from app::channel and updates the UI         │
└───────────────▲─────────────────────────────────────▲───────────────┘
                │ Msg                                  │ Msg
   ┌────────────┴────────────┐            ┌────────────┴─────────────┐
   │ job threads (short-lived)│            │ events listener (1, long)│
   │ list / start / stop /    │            │ GET /events stream       │
   │ remove / prune / logs    │            │ → Msg::DockerEvent       │
   └────────────┬────────────┘            └────────────┬─────────────┘
                │ HTTP/1.0 over Unix socket            │
                └──────────────► /var/run/docker.sock ◄┘
```

### Threading model

- **Main thread:** runs `app.wait()` in a loop. After each wakeup it drains the channel
  receiver and applies each `Msg` to the UI. It never performs Docker I/O.
- **Job threads:** each user action or refresh spawns one short-lived `std::thread` that
  performs the request and sends a result `Msg`. Actions are infrequent, so a thread per job is
  simpler than a pool.
  - Refresh coalescing: at most one in-flight refresh per list. If a refresh is requested while
    one is running, set a "pending" flag and run once more when it finishes.
- **Events listener:** one long-lived thread holding the `GET /events` stream (§7.6).
- **Log follow threads:** one per logs window while "Follow" is on (§7.5).

### Messages

A single `Msg` enum in `src/msg.rs`. Indicative shape (adjust as needed, keep it flat):

```rust
pub enum Msg {
    // results from job threads
    Connected(Result<VersionInfo, DockerError>),
    ContainersLoaded(Result<Vec<ContainerSummary>, DockerError>),
    ImagesLoaded(Result<Vec<ImageSummary>, DockerError>),
    ActionDone { label: String, result: Result<String, DockerError> },
    // from the events listener
    DockerEvent(EventScope),          // Container | Image
    EventsDisconnected,
    // from log threads
    LogText { window_id: u32, text: String },
    LogEnded { window_id: u32, error: Option<DockerError> },
}
```

UI widget callbacks send `Msg` values through the same channel (e.g. `Msg::Refresh`), so all
state lives in one struct handled by one `match`, with no `Rc<RefCell>` (§14 #27).

## 6. Project structure

```
kubustack/
├── Cargo.toml
├── Cargo.lock
├── CLAUDE.md  SPEC.md  PLAN.md  README.md  LICENSE
├── src/
│   ├── main.rs              # arg parsing, app init, main loop, Msg dispatch
│   ├── msg.rs               # Msg enum
│   ├── format.rs            # sizes, relative times, ports, names, ANSI stripping
│   ├── worker.rs            # helpers to spawn job threads that send Msg results
│   ├── docker/
│   │   ├── mod.rs           # DockerError, socket discovery
│   │   ├── http.rs          # raw HTTP/1.0 over UnixStream, response parsing
│   │   ├── api.rs           # typed functions per endpoint
│   │   ├── models.rs        # serde structs
│   │   ├── logs.rs          # log stream demultiplexer
│   │   └── events.rs        # events listener thread
│   └── ui/
│       ├── mod.rs
│       ├── main_window.rs   # window, tabs, status bar, error panel
│       ├── containers_tab.rs
│       ├── images_tab.rs
│       ├── logs_window.rs
│       └── dialogs.rs       # confirm / alert wrappers
├── tests/fixtures/          # saved Docker API responses (JSON and raw HTTP)
├── assets/
│   ├── kubustack.svg        # app icon
│   └── kubustack.desktop    # reference desktop entry
├── scripts/
│   ├── install.sh
│   └── uninstall.sh
└── .github/workflows/
    ├── ci.yml
    └── release.yml
```

## 7. Docker integration

### 7.1 Socket discovery

Resolve the socket path once at startup, in this order:

1. `DOCKER_HOST` if set:
   - `unix:///path/to.sock` → use that path.
   - Any other scheme (`tcp://`, `ssh://`) → show "Remote Docker hosts aren't supported yet.
     Unset DOCKER_HOST or point it to a unix:// socket."
2. `/var/run/docker.sock` if it exists.
3. `$XDG_RUNTIME_DIR/docker.sock` if it exists (rootless Docker).
4. Otherwise fall back to `/var/run/docker.sock` and let the connection error explain.

The README documents Podman use:
`DOCKER_HOST=unix://$XDG_RUNTIME_DIR/podman/podman.sock kubustack`.

### 7.2 HTTP client (`docker/http.rs`)

Each request opens a new `UnixStream`, writes the request, and reads the response.
HTTP/1.0 is used to keep parsing simple. Docker itself then sends plain bodies, but OrbStack's
socket proxy answers with chunked bodies anyway (§14 #26), so whole responses are de-chunked and
streams (logs, events) need a de-chunking reader.

Request format:

```
{METHOD} {PATH} HTTP/1.0\r\n
Host: docker\r\n
User-Agent: kubustack/{CARGO_PKG_VERSION}\r\n
Connection: close\r\n
Content-Length: 0\r\n          (POST and DELETE only)
\r\n
```

- Paths are unversioned (e.g. `/containers/json`), so the daemon uses its own API version.
- Two entry points:
  - `request(socket, method, path, timeout) -> Result<Vec<u8>, DockerError>`: returns the whole
    body. Statuses ≥ 400 become `DockerError::Api`; 2xx and 304 are success, so callers never
    need the status code.
  - `stream(method, path) -> Result<(u16, BufReader<UnixStream>), DockerError>`: reads the
    status line and headers, then hands back the reader positioned at the body.
- Parsing: status line `HTTP/1.x NNN reason`, headers until the empty line. The body is exactly
  `Content-Length` bytes when that header is present, otherwise everything until the server
  closes the connection.
- Defensive: if a `Transfer-Encoding: chunked` header is present, de-chunk the body. This is a
  small, unit-tested function.
- Timeouts via `set_read_timeout`: 15 s for list/inspect calls, 60 s for stop/restart (Docker
  waits up to 10 s before killing), 120 s for prune. Streams have no read timeout.
- Non-2xx responses: Docker returns `{"message": "..."}`. Convert to
  `DockerError::Api { status, message }`.
- Minimal URL-encoding helper for query values (needed for the `filters` JSON).

### 7.3 Errors (`docker/mod.rs`)

```rust
pub enum DockerError {
    SocketMissing(PathBuf),         // ENOENT / ECONNREFUSED
    PermissionDenied(PathBuf),      // EACCES
    Io(String),
    Api { status: u16, message: String },
    Parse(String),
    Unsupported(String),            // e.g. tcp:// DOCKER_HOST
}
```

Each variant has a user-facing message (`impl Display`):

- **SocketMissing:** "Can't reach Docker at {path}. Is the Docker service running? Try:
  `sudo systemctl start docker`"
- **PermissionDenied:** "Permission denied for {path}. Add your user to the docker group:
  `sudo usermod -aG docker $USER`, then log out and back in."
- **Api:** the Docker message, prefixed with a short action label (e.g. "Couldn't remove
  image: ...").

### 7.4 Endpoints used

| Purpose | Method and path | Success | Notes |
|---|---|---|---|
| Version / health check | `GET /version` | 200 | On startup and on Retry; version shown in status bar (§14 #25) |
| List containers | `GET /containers/json?all=1` | 200 | |
| Inspect container | `GET /containers/{id}/json` | 200 | Only to read `Config.Tty` for logs |
| Start | `POST /containers/{id}/start` | 204, 304 | 304 = already running, treat as success |
| Stop | `POST /containers/{id}/stop` | 204, 304 | |
| Restart | `POST /containers/{id}/restart` | 204 | |
| Remove container | `DELETE /containers/{id}?force={0,1}` | 204 | `force=1` only after the user confirms removal of a running container |
| Logs | `GET /containers/{id}/logs?stdout=1&stderr=1&tail=500` | 200 stream | Add `follow=1&tail=0` for follow mode |
| List images | `GET /images/json` | 200 | |
| Remove image | `DELETE /images/{name-or-id}` | 200 | 409 when in use: show Docker's message |
| Prune images | `POST /images/prune?filters=...` | 200 | See §8.4 |
| Events | `GET /events?filters={"type":["container","image"]}` | 200 stream | URL-encode the filters JSON |

### 7.5 Models (`docker/models.rs`)

Use `#[serde(rename_all = "PascalCase")]`, `#[serde(default)]` for optional collections, and
ignore unknown fields (serde's default).

```rust
pub struct ContainerSummary {
    pub id: String,
    pub names: Vec<String>,       // e.g. ["/web"]: strip the leading '/'
    pub image: String,
    pub state: String,            // running | exited | paused | created | restarting | dead
    pub status: String,           // human text, e.g. "Up 3 hours"
    pub created: i64,             // unix seconds
    pub ports: Vec<Port>,
}

pub struct Port {
    #[serde(rename = "IP")] pub ip: Option<String>,
    pub private_port: u16,
    pub public_port: Option<u16>,
    #[serde(rename = "Type")] pub kind: String,   // tcp | udp | sctp
}

pub struct ImageSummary {
    pub id: String,                       // "sha256:..."
    pub repo_tags: Option<Vec<String>>,   // may be null or ["<none>:<none>"]
    pub size: i64,
    pub created: i64,
}

pub struct VersionInfo { pub version: String, pub api_version: String }
```

Save real responses from Docker into `tests/fixtures/` and unit test deserialization against
them, including edge cases (null `RepoTags`, no ports, IPv4+IPv6 duplicate ports).

### 7.6 Logs (`docker/logs.rs`)

- Read `Config.Tty` from inspect first.
- **TTY = true:** the body is the raw stream.
- **TTY = false:** the body is multiplexed into frames: an 8-byte header
  `[stream_type: u8, 0, 0, 0, size: u32 big-endian]` followed by `size` bytes of payload
  (stream type 1 = stdout, 2 = stderr).
- Implement a **stateful demuxer** that accepts arbitrary byte chunks (frames can be split
  across reads) and yields text. Keep incomplete trailing UTF-8 bytes until the next chunk,
  then use `String::from_utf8_lossy`.
- Strip ANSI escape sequences (`format::strip_ansi`, a small state machine) before display.
- Stderr text is shown the same as stdout in v1.
- **Stopping follow:** the logs window keeps a `try_clone()` of the `UnixStream`. When follow
  is switched off or the window closes, call `shutdown(Shutdown::Both)` on the clone. The
  blocked read returns and the thread exits.

### 7.7 Events (`docker/events.rs`)

- One thread opens the events stream and parses objects with
  `serde_json::Deserializer::from_reader(reader).into_iter::<serde_json::Value>()`, which
  works whether or not objects are newline-separated.
- Map `Type == "container"` → `Msg::DockerEvent(Container)`, `Type == "image"` →
  `Msg::DockerEvent(Image)`. Ignore everything else.
- **Debounce on the main thread:** on a `DockerEvent`, mark that list dirty. If no timer is
  pending, schedule one with `app::add_timeout3(0.3, ...)` that refreshes the dirty lists.
  This turns bursts (e.g. a prune emitting dozens of events) into one refresh.
- **Reconnect:** if the stream ends or fails, send `Msg::EventsDisconnected` and retry with
  backoff (2 s, 4 s, 8 s, … capped at 30 s). On reconnect, refresh both lists once.

## 8. UI specification

### 8.1 Main window

```
┌ KUBUSTACK ────────────────────────────────────────────────────────────┐
│ ┌ Containers ┐┌ Images ┐                                                │
│ │ [Start] [Stop] [Restart] [Remove] [Logs]                  [Refresh] │ │
│ │ NAME           IMAGE             STATUS          PORTS      CREATED │ │
│ │ web            nginx:alpine      Up 3 hours      8080→80    3h ago  │ │
│ │ db             postgres:16       Exited (0) 2d…             2d ago  │ │
│ │ …                                                                   │ │
│ └─────────────────────────────────────────────────────────────────────┘ │
│ Connected · Docker 27.3.1 · 5 containers (2 running) · 12 images        │
└───────────────────────────────────────────────────────────────────────┘
```

- Title `KUBUSTACK`. Default size 900×560, resizable, minimum 640×400.
- Window X class `kubustack` (matches `StartupWMClass` in the desktop entry).
- Window icon from the embedded `assets/kubustack.svg` via `fltk::image::SvgImage`
  (`include_str!`), so no extra files are needed at runtime.
- Plain FLTK scheme (`Gtk` or `Gleam`, pick whichever looks cleaner in the spike), system
  default fonts, no images in lists.
- Layout: `Tabs` filling the window above a one-line status bar. Each tab has a button row
  on top and a list filling the rest. Use `fltk::group::Flex` for layout.

### 8.2 Lists

- Use `fltk::browser::HoldBrowser` (single selection) with `set_column_widths` and a tab
  column separator. Render the header as a separate non-selectable line above the list. If
  `HoldBrowser` proves inadequate, `fltk::table::Table` is the fallback; record the choice in
  the Decision Log.
- Track the selected item by **ID**, and restore the selection after each refresh if the item
  still exists.
- Long cell text is truncated with `…` to fit its column.
- Empty state: a single grey line "No containers" / "No images".

### 8.3 Containers tab

| Column | Source |
|---|---|
| Name | `Names[0]` without the leading `/` |
| Image | `Image` |
| Status | `Status` (green text when `State == running` via browser color codes) |
| Ports | Published as `8080→80/tcp`, unpublished as `80/tcp`; IPv4/IPv6 duplicates merged; comma-separated |
| Created | Relative time, e.g. `3h ago`, `2d ago` |

- Sort: running containers first, then by name.
- **Buttons** (enabled by the selected container's state):
  - Start: enabled when not running.
  - Stop and Restart: enabled when running.
  - Remove: always enabled. Confirm "Remove container {name}?". If it is running, the dialog
    says "{name} is running. Stop and remove it?" and uses `force=1`.
  - Logs: always enabled. Double-clicking a row also opens logs.
  - Refresh: always enabled.
- While an action runs, the status bar shows e.g. "Stopping web…". On success it shows
  "Stopped web". On failure, show `dialog::alert` with the error message.

### 8.4 Images tab

| Column | Source |
|---|---|
| Repository:Tag | One row **per tag**. Images without tags show `<none>:<none>` |
| Image ID | First 12 hex characters after `sha256:` |
| Size | Decimal units like the Docker CLI (`kB`, `MB`, `GB`, base 1000, one decimal) |
| Created | Relative time |

- Sort: by repository:tag, `<none>` rows last.
- **Buttons:**
  - Remove: confirm "Remove image {repo:tag}?". Delete by `repo:tag` (untags; deletes the
    image when it was the last tag) or by ID for `<none>` rows. On 409, show Docker's message
    (e.g. image in use by a container).
  - Prune: a three-button dialog: "Remove dangling images" (default),
    "Remove all unused images", "Cancel". Dangling uses `filters={"dangling":["true"]}`, all
    unused uses `filters={"dangling":["false"]}`. Afterwards the status bar shows
    "Removed N images, reclaimed X MB" from `ImagesDeleted` and `SpaceReclaimed`.
  - Refresh.

### 8.5 Logs window

- Separate window titled `Logs — {container name}`, 800×500, resizable.
- `TextDisplay` with a `TextBuffer`, monospace font (`Font::Courier`, size 13), auto-scroll to
  the end on new text.
- Controls: a "Follow" checkbox (off by default) and a "Close" button.
- On open: load the last 500 lines (non-follow request).
- Follow on: start a `follow=1&tail=0` stream and append text as it arrives.
  Follow off or window closed: stop the stream (§7.6).
- Memory cap: when the buffer exceeds 6000 lines, trim from the top back to 5000.
- Opening logs for a container whose logs window is already open focuses that window.

### 8.6 Status bar and connection states

- **Connected:** `Connected · Docker {version} · {n} containers ({r} running) · {m} images`
- **Disconnected:** the tabs are replaced by an error panel showing the `DockerError` message
  in plain language and a **Retry** button. Retry runs `/_ping` and, on success, restores the
  tabs and refreshes both lists.
- **Events stream down:** status bar appends ` · live updates paused` until it reconnects.

### 8.7 Keyboard shortcuts

| Key | Action |
|---|---|
| F5 | Refresh both lists |
| Delete | Remove the selected item (with confirmation) |
| Enter | Open logs for the selected container (Containers tab) |
| Ctrl+Q | Quit |

## 9. Command-line interface

Parsed manually from `std::env::args` (no `clap`).

| Command | Behavior |
|---|---|
| `kubustack` | Launch the GUI |
| `kubustack --version` | Print `kubustack {version}` and exit |
| `kubustack --help` | Print usage and exit |
| `kubustack --check` | Headless connection check (no window), exit code 0 on success, 1 on failure |

`--check` output example:

```
Socket:     /var/run/docker.sock
Docker:     27.3.1 (API 1.47)
Containers: 5 (2 running)
Images:     12
```

On failure it prints the same plain-language error message the GUI shows.

## 10. Packaging and distribution

### 10.1 Release artifacts

Pushing a tag `vX.Y.Z` runs `.github/workflows/release.yml` on `ubuntu-22.04`:

1. Check that the tag version equals the `version` in `Cargo.toml`; fail otherwise.
2. Install the build dependencies (as in `CLAUDE.md`), then `cargo test` and
   `cargo build --release`.
3. Create `kubustack-x86_64-linux.tar.gz` containing `kubustack`, `kubustack.svg`,
   `kubustack.desktop`, `LICENSE` and `README.md`.
4. Create `SHA256SUMS` for the tarball.
5. Publish a GitHub Release with both files using the preinstalled `gh` CLI
   (`gh release create`), avoiding third-party actions.

`.github/workflows/ci.yml` runs on every push and pull request: `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release`.

### 10.2 Install script (`scripts/install.sh`)

One-line install (also used to update, since re-running replaces the files):

```bash
curl -fsSL https://raw.githubusercontent.com/nixondebantu/kubustack/main/scripts/install.sh | sh
```

Requirements: POSIX `sh`, no `sudo`, no `jq`. Behavior:

1. Exit with a clear message if `uname -m` is not `x86_64`.
2. Use `curl`, or `wget` if `curl` is missing.
3. Download from
   `https://github.com/nixondebantu/kubustack/releases/latest/download/kubustack-x86_64-linux.tar.gz`
   plus `SHA256SUMS`. If `KUBUSTACK_VERSION=vX.Y.Z` is set, use
   `.../releases/download/vX.Y.Z/...` instead.
4. Verify the checksum with `sha256sum` when available.
5. Extract to a temp directory (cleaned up with `trap`), then install:
   - `~/.local/bin/kubustack` (mode 755)
   - `~/.local/share/icons/hicolor/scalable/apps/kubustack.svg`
   - `~/.local/share/applications/kubustack.desktop`, generated with the absolute path of the
     binary in `Exec=` (launchers don't always have `~/.local/bin` on `PATH`)
6. Run `update-desktop-database ~/.local/share/applications` if available (ignore failure).
7. Warnings, not failures: `~/.local/bin` not on `PATH`; user not in the `docker` group
   (include the fix command).
8. Print the installed version and how to launch it.

### 10.3 Uninstall script (`scripts/uninstall.sh`)

Removes the three installed files and refreshes the desktop database. Also runnable as a
one-liner from the raw GitHub URL.

### 10.4 Desktop entry

```ini
[Desktop Entry]
Type=Application
Name=KUBUSTACK
GenericName=Docker Manager
Comment=Lightweight viewer and manager for Docker containers and images
Exec=/home/USER/.local/bin/kubustack
Icon=kubustack
Terminal=false
Categories=Development;Utility;
Keywords=docker;container;image;podman;
StartupWMClass=kubustack
```

### 10.5 Icon

`assets/kubustack.svg`: a simple flat original design (three stacked rounded rectangles
suggesting a "stack" of containers, one accent color `#2F6FEB` on transparent), readable at
16 px. Keep it under 2 KB and use only shapes nanosvg can render (no filters or text).

## 11. Testing strategy

**Unit tests** (no Docker needed, run in CI):

- HTTP response parsing: status line, headers, body, empty 204 body, chunked decoding.
- Deserialization of every model from `tests/fixtures/`.
- Log demuxer: single frame, multiple frames, frames split at every byte position, TTY raw
  mode, split UTF-8 sequences.
- `strip_ansi`, size formatting, relative time, ports formatting and dedupe, name stripping.
- `DOCKER_HOST` parsing and socket discovery order (with paths injected for testability).
- URL-encoding of the filters JSON.

**Manual tests:** the checklist in `PLAN.md` (Milestone 9), run on a real desktop with
throwaway `kubustack-test-*` containers.

## 12. Acceptance criteria for v1.0

- [ ] All §2 goals work against a real Docker daemon.
- [ ] Performance budget in §3 met and measured numbers recorded in the Decision Log.
- [ ] `ldd target/release/kubustack` lists only standard desktop libraries.
- [ ] App shows the correct error panel when Docker is stopped, and recovers via Retry.
- [ ] App shows the permission error and fix when the user is not in the `docker` group.
- [ ] Lists update within ~1 s of `docker run` / `docker rm` in a terminal, with no polling.
- [ ] Closing a following logs window ends its thread (verify with `ps -T` or `top -H`).
- [ ] CI is green; tagging `v0.1.0` produces a release with tarball and checksums.
- [ ] The one-line installer works on a second machine and the app appears in the app menu.

## 13. Future (v2 candidates)

Dark mode, live stats for the selected container, volumes and networks tabs, Compose grouping
(by the `com.docker.compose.project` label), pull image, open a shell in a container via the
system terminal, column sorting, filter/search box, aarch64 builds, `.deb`/`.rpm` packages,
remote hosts, settings file.

## 14. Decision log

| # | Decision | Choice | Reason |
|---|---|---|---|
| 1 | v1 features | Containers + Images tabs, basic actions, logs | Small, finishable scope |
| 2 | Deferred | Stats, volumes, networks, pull, Compose, exec | Complexity and resource use |
| 3 | Logs | Last 500 lines + optional Follow | Useful without constant streaming |
| 4 | Language/toolkit | Rust + fltk-rs (static) | Smallest footprint with zero install deps |
| 5 | Docker access | Raw HTTP/1.0 over Unix socket, serde_json | No SDK or async runtime |
| 6 | Refresh | Events stream + debounce + manual Refresh | Zero idle CPU |
| 7 | Dependencies | Only fltk, serde, serde_json | Size and build speed |
| 8 | Display server | X11, Wayland via XWayland (see #23) | Decided in the Milestone 1 spike |
| 9 | Look | Plain FLTK scheme, no theming | Minimalism |
| 10 | Safety | Confirm all destructive actions | Protect user data |
| 11 | Errors | In-window panel with fix + Retry | No crashes, clear guidance |
| 12 | Tray/settings | None in v1 | Fewer deps and less code |
| 13 | Release | GitHub Releases binary, built on ubuntu-22.04 | Wide glibc compatibility |
| 14 | Install location | `~/.local`, no sudo, plus uninstall script | Simple and safe |
| 15 | Architecture | x86_64 only | Matches owner's machines |
| 16 | Repository | github.com/nixondebantu/kubustack | Owner's account |
| 17 | License | MIT | Simple, permissive |
| 18 | Versioning | Semver from 0.1.0, tag triggers release | Automated releases |
| 19 | Testing | Unit tests with fixtures + manual checklist | GUI is hard to automate |
| 20 | Name | KUBUSTACK | No exact-match project found (Oct 2026); a similar name, Kubestack (a Kubernetes Terraform framework), exists. Accepted by owner unless changed. |
| 21 | Dev machine | macOS arm64 (OrbStack Docker); Linux checks run by owner later | Owner's machine. FLTK runs on macOS, so UI work is testable; Linux-only checks (`ldd`, Wayland, size/RSS budget) need a Linux desktop |
| 22 | FLTK scheme | `Gtk` | Flatter and cleaner than `Gleam` (whose gradient borders look dated) in the M1 spike screenshots |
| 23 | fltk features | Default features only: no `use-wayland`, no `no-pango` | `no-pango` rendered `→` and CJK as boxes (the ports column needs `→`) and was not smaller (1.20 MB vs 1.18 MB). `use-wayland` needs extra build packages and could not be verified on a Wayland session; XWayland covers Wayland desktops |
| 24 | HTTP framing | Send `Connection: close` and honor `Content-Length` | OrbStack's socket proxy keeps HTTP/1.0 connections open, so reading to EOF hung until the timeout (found in M2) |
| 25 | Health check | `GET /version` instead of `/_ping` + `/version` | One request proves the connection and gives the version for the status bar |
| 26 | Chunked replies | De-chunk whole bodies; streams need a de-chunking reader (M6/M7) | OrbStack sends `Transfer-Encoding: chunked` even to HTTP/1.0 requests (found in M3) |
| 27 | UI events | Widget callbacks send `Msg` through the channel | One state struct and one dispatch `match`, no shared `Rc<RefCell>` |
| 28 | Port model | No `IP` field | Never displayed; IPv4/IPv6 duplicates are merged by port and type |
| 29 | Untagged images | Listed as `<none>:<none>` rows (matches `docker images -a`) | Docker 29's `docker images` hides untagged images by default, but they are what prune removes, so the list shows them. The status bar counts image objects, not rows |
| — | Spike results | **Linux** (Mint, X11, x86_64, fltk 1.5.23): default build 1.18 MB stripped; `ldd` lists 46 stock desktop libs (X11/Xft/fontconfig/freetype, pango/cairo/harfbuzz/glib), no extras. `no-pango` build: 1.20 MB, 25 libs, RSS 8.8 MB, CPU 0.1% (lifetime average incl. startup). Default-build RSS not captured, re-measured in M8. **macOS** (arm64): 1.06 MB, idle CPU 0.0% | Measured during M1 |
