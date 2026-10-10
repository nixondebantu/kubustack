# KUBUSTACK — Implementation Plan

Work through the milestones **in order, one at a time**. Each milestone ends with
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` passing,
ticked checkboxes below, a commit named `M<n>: <title>`, and a summary for the user.
See `CLAUDE.md` for rules and `SPEC.md` for details (section numbers in brackets).

---

## M0 — Project scaffold

- [x] `cargo init --name kubustack`, edition 2021
- [x] Add dependencies: `fltk`, `serde` (feature `derive`), `serde_json`
- [x] Release profile as in [§4]
- [x] Create the folder layout from [§6] with empty modules that compile
- [x] `LICENSE` (MIT, copyright holder: nixondebantu), `.gitignore` (`/target`)
- [x] `.github/workflows/ci.yml` [§10.1]: fmt, clippy, test, release build on `ubuntu-22.04`
- [x] Minimal `README.md` stub (name, one-line description, "work in progress")
- [x] `git init`, first commit. If the `gh` CLI is installed and authenticated, ask the user
      before creating the GitHub repo `nixondebantu/kubustack` and pushing; otherwise print
      the manual steps.

**Done when:** `cargo build` succeeds, and CI is green after the first push.

---

## M1 — Toolkit spike and measurements

- [x] Main window per [§8.1]: title, size, min size, X class, `Tabs` with empty
      "Containers" and "Images" groups, status bar text "Not connected"
- [x] Embedded SVG window icon (placeholder SVG is fine until M9)
- [x] Try the `Gtk` and `Gleam` schemes, pick one
- [x] Evaluate fltk features using the rule in [§4]:
  - [x] Default build: run on the current session, record `ldd` output
  - [x] `use-wayland` (if available): does it run on Wayland and on X11? Extra runtime libs?
  - [x] `no-pango`: size and `ldd` difference; check UTF-8 text still renders
- [x] Measure: stripped release binary size, idle RSS (`ps -o rss= -p <pid>`), idle CPU
- [x] Record results and chosen features in the [§14] Decision Log "Spike results" row

**Done when:** the empty window runs from a release build, numbers are recorded, and the
user has approved the feature choice.

---

## M2 — Docker client core

- [x] `DockerError` with user-facing `Display` messages [§7.3]
- [x] Socket discovery with `DOCKER_HOST` parsing [§7.1], written so paths and env values can be
      injected in tests
- [x] `http.rs`: `request`, response parser, chunked decoder, timeouts [§7.2]. `stream` moved
      to M6 and the URL-encode helper to M5, where each is first used (no dead code)
- [x] `models.rs`: `VersionInfo` (others can come with their milestones)
- [x] `api.rs`: `version()` (`ping()` moved to M3, where the GUI first uses it)
- [x] CLI: `--version`, `--help`, `--check` [§9] (`--check` prints the first two lines for now)
- [x] Unit tests: response parsing (200 with body, 204 empty, error JSON), chunked decoding,
      `DOCKER_HOST` parsing, discovery order (URL-encoding tests move to M5 with the helper)
- [x] Save raw fixtures captured from the local daemon into `tests/fixtures/`

**Done when:** `cargo run -- --check` prints socket and Docker version, and prints the correct
friendly error when Docker is stopped (`sudo systemctl stop docker`, ask the user first) or
when the socket path is wrong (`DOCKER_HOST=unix:///nope.sock`).

---

## M3 — Containers list (read-only)

- [x] `ContainerSummary` and `Port` models with fixture tests [§7.5]
- [x] `api::list_containers()` (`/version` doubles as the health check, §14 #25)
- [x] `format.rs`: relative time, ports formatting with IPv4/IPv6 merge, name stripping,
      truncation with `…`, with unit tests
- [x] `msg.rs`, `worker.rs`, and the main loop draining `app::channel` [§5]
- [x] Containers tab list with header, columns, sorting, green running status [§8.2, §8.3]
- [x] Refresh button and F5; refresh coalescing [§5]
- [x] Status bar connected text with counts [§8.6]
- [x] Error panel with Retry when Docker is unreachable [§8.6]
- [x] Selection kept by ID across refreshes; empty state text
- [x] `--check` now also prints container and image counts

**Done when:** the list matches `docker ps -a`, refreshing never freezes the window, and
stopping/starting Docker shows the error panel and Retry recovers.

---

## M4 — Container actions

- [x] `api::start`, `stop`, `restart`, `remove(force)` with 304 treated as success [§7.4]
- [x] Buttons enabled/disabled by selected container state [§8.3]
- [x] Confirmation dialogs, including "running → stop and remove" wording [§8.3]
- [x] Status bar progress and result text; `dialog::alert` on errors
- [x] Delete key → remove, with confirmation [§8.7]
- [x] List refreshes after each action (events come in M6, so refresh explicitly for now)

**Done when:** all actions work on a throwaway `kubustack-test-web` container, and errors
(e.g. removing an already-removed container) show a readable message.

---

## M5 — Images tab

- [x] `ImageSummary` model with fixture tests, including null `RepoTags` [§7.5]
- [x] `api::list_images`, `remove_image`, `prune_images(dangling_only)`; URL-encode helper for
      the `filters` JSON, with tests [§7.2]
- [x] Size formatting (base 1000, like the Docker CLI) with tests
- [x] Images list: one row per tag, short ID, sorting, `<none>` last [§8.4]
- [x] Remove with confirmation; 409 shows Docker's message
- [x] Prune three-button dialog and result summary in the status bar [§8.4]

**Done when:** the list matches `docker images`, removing `kubustack-test` tagged images works,
and removing an in-use image shows the conflict message. Test prune only after asking the user.

---

## M6 — Live updates via events

- [x] `http::stream` [§7.2]; `events.rs` listener thread using the streaming JSON deserializer [§7.7]
- [x] Debounced refresh on the main thread with `app::add_timeout3`
- [x] Reconnect with backoff; "live updates paused" status suffix; full refresh on reconnect
- [x] Remove the explicit post-action refreshes from M4 if events now cover them (keep them if
      it makes the UI feel snappier, but never refresh twice for one action)

**Done when:** running `docker run -d --name kubustack-test-ev nginx:alpine` and
`docker rm -f kubustack-test-ev` in a terminal updates the window within ~1 s, idle CPU stays
at ~0%, and restarting the Docker service recovers live updates.

---

## M7 — Logs window

- [x] `api::inspect_tty`, `api::logs(follow)` returning a stream [§7.4]
- [x] Stateful demuxer and UTF-8 carry-over in `logs.rs`, with split-at-every-byte tests [§7.6]
- [x] `format::strip_ansi` with tests
- [x] Logs window: monospace text, auto-scroll, Follow checkbox, Close [§8.5]
- [x] Stop following via socket `shutdown` on toggle-off and on window close
- [x] 6000→5000 line trimming
- [x] Focus an existing window instead of opening a duplicate
- [x] Open via Logs button, double-click, and Enter [§8.7]

**Done when:** logs match `docker logs --tail 500` for both a TTY container
(`docker run -dt ...`) and a non-TTY one, follow shows new lines live, and closing the window
ends its thread (check with `top -H -p <pid>`).

---

## M8 — Polish and performance check

- [x] Ctrl+Q quits; all shortcuts from [§8.7] work
- [x] Column widths adapt sensibly when the window is resized
- [x] Review every user-facing message for plain, helpful wording
- [x] Re-measure binary size, idle RSS with ~50 containers, idle CPU, startup time against
      [§3]; record in the Decision Log. Fix anything over budget.
- [x] `ldd` check: only standard desktop libraries
- [x] Remove dead code; clippy clean

**Done when:** all [§3] budgets are met and recorded.

---

## M9 — Packaging, release, and docs

- [x] Final `assets/kubustack.svg` icon [§10.5] and reference `assets/kubustack.desktop` [§10.4]
- [x] `scripts/install.sh` [§10.2] and `scripts/uninstall.sh` [§10.3]; check both with
      `shellcheck` if it is installed (not installed locally: CI runs it; tested under busybox
      `sh` in Alpine against a fake release)
- [x] `.github/workflows/release.yml` [§10.1]
- [x] `README.md`: description, screenshot placeholder, install one-liner, manual install,
      uninstall, usage, Podman/rootless notes, docker group fix, build from source,
      supported systems, license
- [ ] Run the manual test checklist below and fix issues _(owner, on Linux)_
- [x] With user approval: bump to `0.1.0` if needed, tag `v0.1.0`, push, confirm the release
      contains the tarball and `SHA256SUMS`

**Done when:** the one-liner installs the app on a second machine, it appears in the app menu
with its icon, and the uninstall script removes it cleanly.

### Manual test checklist

Use only `kubustack-test-*` containers and images.

- [ ] Launch from the app menu and from a terminal
- [ ] Containers list matches `docker ps -a`; images list matches `docker images`
- [ ] Start, stop, restart, remove (stopped and running) a test container
- [ ] Remove a test image; remove an in-use image (expect a conflict message)
- [ ] Prune dangling images (with user approval)
- [ ] Logs: TTY and non-TTY container, follow on/off, close while following
- [ ] Changes made in a terminal appear automatically
- [ ] Stop Docker while the app is open → error panel; start Docker → Retry recovers
- [ ] Run as a user outside the `docker` group → permission message with fix
- [ ] `DOCKER_HOST=tcp://...` → unsupported message
- [ ] Window resize, minimum size, keyboard shortcuts
- [ ] Idle for 5 minutes: CPU ~0%, memory stable
- [ ] `kubustack --version`, `--help`, `--check`
