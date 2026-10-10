//! Long-lived listener thread for the Docker events stream.

use super::http::{encode, stream};
use crate::msg::{Msg, Scope};
use fltk::app::Sender;
use serde::Deserialize;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Deserialize)]
struct Event {
    #[serde(rename = "Type", default)]
    kind: String,
    #[serde(rename = "Action", default)]
    action: String,
}

/// Which list an event affects. Exec events (e.g. health checks) don't change any list.
fn scope(ev: &Event) -> Option<Scope> {
    match ev.kind.as_str() {
        "container" if !ev.action.starts_with("exec_") => Some(Scope::Container),
        "image" => Some(Scope::Image),
        _ => None,
    }
}

/// Starts the listener thread. It reconnects forever with backoff (2 s, 4 s, … 30 s).
pub fn spawn_listener(socket: PathBuf, s: Sender<Msg>) {
    let run = move || {
        let filters = encode(r#"{"type":["container","image"]}"#);
        let path = format!("/events?filters={filters}");
        let mut backoff = 2;
        loop {
            let started = Instant::now();
            if let Ok((body, _)) = stream(&socket, "GET", &path) {
                s.send(Msg::EventsConnected);
                let events = serde_json::Deserializer::from_reader(body).into_iter::<Event>();
                for ev in events {
                    let Ok(ev) = ev else { break };
                    if let Some(scope) = scope(&ev) {
                        s.send(Msg::DockerEvent(scope));
                    }
                }
                // A stream that stayed up a while was healthy: start backing off from scratch.
                if started.elapsed() > Duration::from_secs(30) {
                    backoff = 2;
                }
            }
            s.send(Msg::EventsDisconnected);
            std::thread::sleep(Duration::from_secs(backoff));
            backoff = (backoff * 2).min(30);
        }
    };
    if let Err(e) = std::thread::Builder::new().name("events".into()).spawn(run) {
        eprintln!("kubustack: couldn't start the events listener: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(kind: &str, action: &str) -> Event {
        Event {
            kind: kind.into(),
            action: action.into(),
        }
    }

    #[test]
    fn scopes() {
        assert!(matches!(
            scope(&ev("container", "start")),
            Some(Scope::Container)
        ));
        assert!(matches!(scope(&ev("image", "delete")), Some(Scope::Image)));
        assert!(scope(&ev("container", "exec_start: sh -c true")).is_none());
        assert!(scope(&ev("network", "connect")).is_none());
    }

    #[test]
    fn parses_concatenated_events() {
        // Objects may arrive with or without newlines between them.
        let raw =
            br#"{"Type":"container","Action":"start"}{"Type":"image","Action":"pull","Actor":{}}
{"Type":"network","Action":"connect"}"#;
        let kinds: Vec<String> = serde_json::Deserializer::from_slice(raw)
            .into_iter::<Event>()
            .map(|e| e.unwrap().kind)
            .collect();
        assert_eq!(kinds, ["container", "image", "network"]);
    }
}
