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
