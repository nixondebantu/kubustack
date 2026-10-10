//! Docker client: errors, socket discovery, and submodules.

pub mod api;
pub mod events;
pub mod http;
pub mod logs;
pub mod models;

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum DockerError {
    SocketMissing(PathBuf),
    PermissionDenied(PathBuf),
    Io(String),
    Api { status: u16, message: String },
    Parse(String),
    Unsupported(String),
}

impl fmt::Display for DockerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DockerError::SocketMissing(p) => write!(
                f,
                "Can't reach Docker at {}. Is the Docker service running? Try: `sudo systemctl start docker`",
                p.display()
            ),
            DockerError::PermissionDenied(p) => write!(
                f,
                "Permission denied for {}. Add your user to the docker group: `sudo usermod -aG docker $USER`, then log out and back in.",
                p.display()
            ),
            DockerError::Io(e) => write!(
                f,
                "Couldn't talk to Docker: {e} Check that the Docker service is running."
            ),
            DockerError::Api { status, message } if message.is_empty() => {
                write!(f, "Docker returned HTTP {status}")
            }
            DockerError::Api { message, .. } => f.write_str(message),
            DockerError::Parse(e) => write!(
                f,
                "Docker sent a response KUBUSTACK couldn't read ({e}). Please report this at https://github.com/nixondebantu/kubustack/issues"
            ),
            DockerError::Unsupported(e) => f.write_str(e),
        }
    }
}

impl DockerError {
    /// Maps an I/O error on the socket to the matching user-facing error.
    pub fn from_io(e: io::Error, socket: &Path) -> Self {
        match e.kind() {
            io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused => {
                DockerError::SocketMissing(socket.to_path_buf())
            }
            io::ErrorKind::PermissionDenied => DockerError::PermissionDenied(socket.to_path_buf()),
            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => {
                DockerError::Io("Docker took too long to respond, it may be busy.".into())
            }
            _ => DockerError::Io(e.to_string()),
        }
    }
}

const DEFAULT_SOCKET: &str = "/var/run/docker.sock";

/// Resolves the Docker socket from the environment (SPEC §7.1).
pub fn socket_path() -> Result<PathBuf, DockerError> {
    let host = std::env::var("DOCKER_HOST").ok();
    let xdg = std::env::var("XDG_RUNTIME_DIR").ok();
    resolve_socket(host.as_deref(), xdg.as_deref(), &|p| p.exists())
}

/// Socket discovery with the environment and filesystem injected for tests.
pub fn resolve_socket(
    docker_host: Option<&str>,
    xdg_runtime_dir: Option<&str>,
    exists: &dyn Fn(&Path) -> bool,
) -> Result<PathBuf, DockerError> {
    if let Some(host) = docker_host.filter(|h| !h.is_empty()) {
        return match host.strip_prefix("unix://") {
            Some(path) => Ok(PathBuf::from(path)),
            None => Err(DockerError::Unsupported(
                "Remote Docker hosts aren't supported yet. Unset DOCKER_HOST or point it to a unix:// socket.".into(),
            )),
        };
    }
    let default = PathBuf::from(DEFAULT_SOCKET);
    if exists(&default) {
        return Ok(default);
    }
    if let Some(dir) = xdg_runtime_dir.filter(|d| !d.is_empty()) {
        let rootless = Path::new(dir).join("docker.sock");
        if exists(&rootless) {
            return Ok(rootless);
        }
    }
    Ok(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &Path) -> bool {
        false
    }

    #[test]
    fn docker_host_unix() {
        let p = resolve_socket(
            Some("unix:///run/user/1000/podman/podman.sock"),
            None,
            &none,
        );
        assert_eq!(
            p.unwrap(),
            PathBuf::from("/run/user/1000/podman/podman.sock")
        );
    }

    #[test]
    fn docker_host_remote_is_unsupported() {
        for host in ["tcp://1.2.3.4:2375", "ssh://me@box"] {
            let err = resolve_socket(Some(host), None, &none).unwrap_err();
            assert!(matches!(err, DockerError::Unsupported(_)), "{host}");
        }
    }

    #[test]
    fn empty_docker_host_is_ignored() {
        let p = resolve_socket(Some(""), None, &|p| p == Path::new(DEFAULT_SOCKET));
        assert_eq!(p.unwrap(), PathBuf::from(DEFAULT_SOCKET));
    }

    #[test]
    fn discovery_order() {
        let xdg = Some("/run/user/1000");
        let rootless = PathBuf::from("/run/user/1000/docker.sock");
        // Both exist: the system socket wins.
        let p = resolve_socket(None, xdg, &|_| true).unwrap();
        assert_eq!(p, PathBuf::from(DEFAULT_SOCKET));
        // Only rootless exists.
        let p = resolve_socket(None, xdg, &|p| p == rootless.as_path()).unwrap();
        assert_eq!(p, rootless);
        // Nothing exists: fall back to the system socket.
        assert_eq!(
            resolve_socket(None, xdg, &none).unwrap(),
            PathBuf::from(DEFAULT_SOCKET)
        );
        assert_eq!(
            resolve_socket(None, None, &none).unwrap(),
            PathBuf::from(DEFAULT_SOCKET)
        );
    }

    #[test]
    fn io_errors_map_to_friendly_variants() {
        let s = Path::new("/x.sock");
        let e = |k| DockerError::from_io(io::Error::from(k), s);
        assert!(matches!(
            e(io::ErrorKind::NotFound),
            DockerError::SocketMissing(_)
        ));
        assert!(matches!(
            e(io::ErrorKind::ConnectionRefused),
            DockerError::SocketMissing(_)
        ));
        assert!(matches!(
            e(io::ErrorKind::PermissionDenied),
            DockerError::PermissionDenied(_)
        ));
    }
}
