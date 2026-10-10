//! Messages sent to the UI thread, from job threads and from widget callbacks.

use crate::docker::models::{ContainerSummary, ImageSummary, VersionInfo};
use crate::docker::DockerError;
use std::path::PathBuf;

pub enum Msg {
    // results from job threads
    Connected(Result<(PathBuf, VersionInfo), DockerError>),
    ContainersLoaded(Result<Vec<ContainerSummary>, DockerError>),
    ImagesLoaded(Result<Vec<ImageSummary>, DockerError>),
    /// `result` holds the success text for the status bar; `failed` prefixes the error.
    ActionDone {
        failed: String,
        result: Result<String, DockerError>,
    },
    // from widgets and keys
    Refresh,
    Retry,
    ContainerSelected,
    ContainerAction(Action),
    ImageSelected,
    RemoveImage,
    PruneImages,
    DeleteKey,
}

#[derive(Clone, Copy)]
pub enum Action {
    Start,
    Stop,
    Restart,
    Remove,
}
