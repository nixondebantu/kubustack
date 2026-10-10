//! Messages sent to the UI thread, from job threads and from widget callbacks.

use crate::docker::models::{ContainerSummary, ImageSummary, VersionInfo};
use crate::docker::DockerError;
use std::os::unix::net::UnixStream;
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
    // from log reader threads
    LogText {
        window: u32,
        text: String,
    },
    /// A follow stream started; `handle` stops it.
    LogFollowing {
        window: u32,
        stream: u32,
        handle: UnixStream,
    },
    LogEnded {
        window: u32,
        stream: u32,
        error: Option<DockerError>,
    },
    // from the events listener
    DockerEvent(Scope),
    EventsConnected,
    EventsDisconnected,
    /// The debounce timer after events fired.
    EventsSettled,
    // from widgets and keys
    Refresh,
    Retry,
    ContainerSelected,
    ContainerAction(Action),
    ImageSelected,
    RemoveImage,
    PruneImages,
    DeleteKey,
    EnterKey,
    OpenLogs,
    LogFollow {
        window: u32,
        on: bool,
    },
    LogClose {
        window: u32,
    },
}

#[derive(Clone, Copy)]
pub enum Scope {
    Container,
    Image,
}

#[derive(Clone, Copy)]
pub enum Action {
    Start,
    Stop,
    Restart,
    Remove,
}
