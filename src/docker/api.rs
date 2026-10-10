//! Typed functions, one per Docker Engine API endpoint.

use super::http::request;
use super::models::{ContainerSummary, VersionInfo};
use super::DockerError;
use serde::de::DeserializeOwned;
use std::path::Path;
use std::time::Duration;

const LIST_TIMEOUT: Duration = Duration::from_secs(15);

fn get_json<T: DeserializeOwned>(socket: &Path, path: &str) -> Result<T, DockerError> {
    let body = request(socket, "GET", path, LIST_TIMEOUT)?;
    serde_json::from_slice(&body).map_err(|e| DockerError::Parse(e.to_string()))
}

/// `GET /version`. Also serves as the health check on startup and Retry.
pub fn version(socket: &Path) -> Result<VersionInfo, DockerError> {
    get_json(socket, "/version")
}

/// `GET /containers/json?all=1`.
pub fn list_containers(socket: &Path) -> Result<Vec<ContainerSummary>, DockerError> {
    get_json(socket, "/containers/json?all=1")
}

/// Number of images from `GET /images/json`.
// ponytail: count only; M5 replaces this with list_images and real models.
pub fn count_images(socket: &Path) -> Result<usize, DockerError> {
    get_json::<Vec<serde::de::IgnoredAny>>(socket, "/images/json").map(|v| v.len())
}

/// Docker waits up to 10 s before killing a container on stop/restart.
const ACTION_TIMEOUT: Duration = Duration::from_secs(60);

fn post(socket: &Path, path: &str) -> Result<(), DockerError> {
    request(socket, "POST", path, ACTION_TIMEOUT).map(drop)
}

/// `POST /containers/{id}/start`. 304 (already running) counts as success.
pub fn start(socket: &Path, id: &str) -> Result<(), DockerError> {
    post(socket, &format!("/containers/{id}/start"))
}

/// `POST /containers/{id}/stop`. 304 (already stopped) counts as success.
pub fn stop(socket: &Path, id: &str) -> Result<(), DockerError> {
    post(socket, &format!("/containers/{id}/stop"))
}

/// `POST /containers/{id}/restart`.
pub fn restart(socket: &Path, id: &str) -> Result<(), DockerError> {
    post(socket, &format!("/containers/{id}/restart"))
}

/// `DELETE /containers/{id}`. `force` kills a running container first.
pub fn remove(socket: &Path, id: &str, force: bool) -> Result<(), DockerError> {
    let path = format!("/containers/{id}?force={}", u8::from(force));
    request(socket, "DELETE", &path, ACTION_TIMEOUT).map(drop)
}
