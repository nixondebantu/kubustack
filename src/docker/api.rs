//! Typed functions, one per Docker Engine API endpoint.

use super::http::request;
use super::models::VersionInfo;
use super::DockerError;
use std::path::Path;
use std::time::Duration;

const LIST_TIMEOUT: Duration = Duration::from_secs(15);

/// `GET /version`.
pub fn version(socket: &Path) -> Result<VersionInfo, DockerError> {
    let body = request(socket, "GET", "/version", LIST_TIMEOUT)?;
    serde_json::from_slice(&body).map_err(|e| DockerError::Parse(e.to_string()))
}
