# KUBUSTACK

A super-lightweight native Linux desktop app for viewing and managing your Docker containers
and images. One small window, one ~1.3 MB binary, no runtime to install, and zero CPU while
idle: the lists update live from Docker's event stream instead of polling.

<!-- Screenshot: docs/screenshot.png (to be added) -->

## Features

- **Containers:** see all containers (running first), start, stop, restart and remove them.
- **Logs:** last 500 lines, with optional live Follow.
- **Images:** one row per tag with size and age; remove images and prune unused ones.
- **Live updates:** changes made in a terminal show up within about a second.
- **Safe:** every destructive action asks first.
- **Friendly errors:** if Docker is stopped or you lack permission, the window says how to fix
  it, and recovers by itself when Docker comes back.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/nixondebantu/kubustack/main/scripts/install.sh | sh
```

This installs into your home directory without `sudo`:

- `~/.local/bin/kubustack`
- `~/.local/share/applications/kubustack.desktop` (app menu entry)
- `~/.local/share/icons/hicolor/scalable/apps/kubustack.svg`

Run the same command again to update. To install a specific version:
`curl -fsSL .../install.sh | KUBUSTACK_VERSION=v0.1.0 sh`

### Manual install

Download `kubustack-x86_64-linux.tar.gz` and `SHA256SUMS` from the
[latest release](https://github.com/nixondebantu/kubustack/releases/latest), then:

```bash
sha256sum -c SHA256SUMS
tar -xzf kubustack-x86_64-linux.tar.gz
install -m 755 kubustack ~/.local/bin/kubustack
```

### Uninstall

```bash
curl -fsSL https://raw.githubusercontent.com/nixondebantu/kubustack/main/scripts/uninstall.sh | sh
```

## Usage

Launch **KUBUSTACK** from your app menu, or run `kubustack`.

| Key | Action |
|---|---|
| F5 | Refresh both lists |
| Delete | Remove the selected container or image (asks first) |
| Enter / double-click | Open logs for the selected container |
| Ctrl+Q | Quit |

Command line:

```
kubustack            Launch the app
kubustack --check    Check the Docker connection without opening a window
kubustack --version  Print the version
kubustack --help     Print help
```

## Docker access

KUBUSTACK talks to the Docker Engine API over its Unix socket. It looks for, in order:
`DOCKER_HOST` (only `unix://` sockets), `/var/run/docker.sock`, then
`$XDG_RUNTIME_DIR/docker.sock` (rootless Docker).

**Permission denied?** Add yourself to the `docker` group, then log out and back in:

```bash
sudo usermod -aG docker $USER
```

Note that membership in the `docker` group is equivalent to root access on that machine.

**Podman:** enable its Docker-compatible socket and point KUBUSTACK at it (best effort):

```bash
systemctl --user enable --now podman.socket
DOCKER_HOST=unix://$XDG_RUNTIME_DIR/podman/podman.sock kubustack
```

Remote hosts (`tcp://`, `ssh://`) aren't supported.

## Supported systems

- x86_64 Linux with glibc 2.35 or newer: Ubuntu 22.04+, Debian 12+, Fedora 36+, Linux Mint 21+,
  current Arch.
- X11 desktops, and Wayland desktops through XWayland.
- Docker 20.10+ (Engine API 1.41+).

## Build from source

Requires Rust (stable) and, on Ubuntu/Debian:

```bash
sudo apt-get install -y build-essential cmake git pkg-config \
  libx11-dev libxext-dev libxft-dev libxinerama-dev libxcursor-dev \
  libxrender-dev libxfixes-dev libpango1.0-dev libgl1-mesa-dev libglu1-mesa-dev
git clone https://github.com/nixondebantu/kubustack.git
cd kubustack
cargo build --release
./target/release/kubustack
```

## License

[MIT](LICENSE)
