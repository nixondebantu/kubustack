//! Typed functions, one per Docker Engine API endpoint.

use super::http::{encode, request};
use super::models::{ContainerSummary, ImageSummary, PruneReport, VersionInfo};
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

/// `GET /images/json`.
pub fn list_images(socket: &Path) -> Result<Vec<ImageSummary>, DockerError> {
    get_json(socket, "/images/json")
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

/// `DELETE /images/{name}`: untags `repo:tag`, deleting the image when it was the last tag.
/// Docker answers 409 when a container uses the image.
pub fn remove_image(socket: &Path, name: &str) -> Result<(), DockerError> {
    request(socket, "DELETE", &format!("/images/{name}"), ACTION_TIMEOUT).map(drop)
}

/// `POST /images/prune`: dangling images only, or all images no container uses.
pub fn prune_images(socket: &Path, dangling_only: bool) -> Result<PruneReport, DockerError> {
    let filters = format!(r#"{{"dangling":["{dangling_only}"]}}"#);
    let path = format!("/images/prune?filters={}", encode(&filters));
    let body = request(socket, "POST", &path, Duration::from_secs(120))?;
    serde_json::from_slice(&body).map_err(|e| DockerError::Parse(e.to_string()))
}
