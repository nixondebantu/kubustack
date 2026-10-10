//! Messages sent to the UI thread, from job threads and from widget callbacks.

use crate::docker::models::{ContainerSummary, VersionInfo};
use crate::docker::DockerError;
use std::path::PathBuf;

pub enum Msg {
    // results from job threads
    Connected(Result<(PathBuf, VersionInfo), DockerError>),
    ContainersLoaded(Result<Vec<ContainerSummary>, DockerError>),
    ImagesCounted(Result<usize, DockerError>),
    ActionDone {
        done: String,
        failed: String,
        result: Result<(), DockerError>,
    },
    // from widgets and keys
    Refresh,
    Retry,
    ContainerSelected,
    ContainerAction(Action),
    DeleteKey,
}

#[derive(Clone, Copy)]
pub enum Action {
    Start,
    Stop,
    Restart,
    Remove,
}
