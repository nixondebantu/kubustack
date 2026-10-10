# CLAUDE.md — KUBUSTACK

## What this project is

KUBUSTACK is a super-lightweight native Linux desktop app for viewing and managing Docker
containers and images. It is a single small binary written in Rust with the FLTK toolkit,
and it talks to the Docker Engine API directly over the Unix socket.

- **Full specification:** `SPEC.md` (what to build and how)
- **Roadmap:** `PLAN.md` (ordered milestones with checklists)

Read both files completely before starting any work.

## Priorities (in this order)

1. Low resource use: RAM, CPU, binary size.
2. Safety and correctness with the user's Docker data.
3. Simplicity of code and UI.
4. Features.

When in doubt, choose the smaller, simpler option.

## Tech stack

- Rust stable, edition 2021
- `fltk` (fltk-rs) for the GUI, FLTK linked statically
- `serde` + `serde_json` for JSON
- Standard library for everything else (Unix socket, threads, time, arg parsing)

## Commands

```bash
cargo build                                   # debug build
cargo run                                     # launch the GUI
cargo run -- --check                          # headless Docker connection check
cargo test                                    # unit tests
cargo fmt --check                             # formatting
cargo clippy --all-targets -- -D warnings     # lints, must be clean
cargo build --release                         # optimized, stripped binary
```

Build dependencies on Ubuntu/Debian (verify against the current fltk-rs README):

```bash
sudo apt-get install -y build-essential cmake git pkg-config \
  libx11-dev libxext-dev libxft-dev libxinerama-dev libxcursor-dev \
  libxrender-dev libxfixes-dev libpango1.0-dev libgl1-mesa-dev libglu1-mesa-dev
```

## Hard rules

- **No new crates.** If one seems necessary, stop, explain why to the user, and wait for
  approval. If approved, record it in the Decision Log in `SPEC.md`.
- No async runtime, no Docker SDK, no HTTP client crate, no `chrono`, no `clap`, no logging
  framework.
- FLTK widgets are touched **only on the main thread**. Background threads talk to the UI
  through `fltk::app::channel` messages.
- Never block the main thread on Docker I/O.
- No `unwrap()` / `expect()` outside tests and early startup. The release profile uses
  `panic = "abort"`, so any panic kills the whole app.
- Every destructive action (remove container, remove image, prune) needs a confirmation dialog.
- No config files, tray icon, or persistence in v1.
- Keep idle CPU at zero: no polling timers while idle.

## Safety while developing on this machine

- **Never stop, remove, or prune the user's existing containers or images.**
- For testing, create throwaway resources prefixed `kubustack-test-`, for example
  `docker run -d --name kubustack-test-web nginx:alpine`, and remove them afterwards.
- Do not run `docker image prune -a` or `docker system prune`. To test prune, ask the user first.

## Workflow

- Work on **one milestone from `PLAN.md` at a time**, in order.
- At the end of each milestone:
  1. `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` all pass.
  2. Tick the completed checkboxes in `PLAN.md`.
  3. Commit with the message `M<n>: <milestone title>`.
  4. Summarize what changed and what the user should check manually, then stop and wait.
- If the spec is ambiguous or wrong, propose a fix and update `SPEC.md` (Decision Log)
  rather than silently deviating.

## Code style

- Follow the module layout in `SPEC.md` §6. One short doc comment at the top of each module.
- Prefer plain functions and structs over traits and generics.
- Error messages shown to the user are plain language and suggest a fix.
- Pure logic (parsing, formatting, demuxing) lives outside UI code so it can be unit tested.

## Repository

- GitHub: `github.com/nixondebantu/kubustack`
- License: MIT. Versioning: semver, starting at `0.1.0`.
- Pushing a tag `vX.Y.Z` triggers the release workflow.
