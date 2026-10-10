//! Pure formatting helpers: sizes, relative times, ports, names, ANSI stripping.

use crate::docker::models::Port;
use std::time::{SystemTime, UNIX_EPOCH};

/// Current time in unix seconds.
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Container display name: `Names[0]` without the leading `/`, or the short ID.
pub fn container_name(names: &[String], id: &str) -> String {
    match names.first() {
        Some(n) => n.trim_start_matches('/').to_string(),
        None => id.chars().take(12).collect(),
    }
}

/// "just now", "5m ago", "3h ago", "2d ago", "4mo ago", "1y ago".
pub fn relative_time(created: i64, now: i64) -> String {
    let s = now - created;
    match s {
        _ if s < 60 => "just now".to_string(),
        _ if s < 3600 => format!("{}m ago", s / 60),
        _ if s < 86_400 => format!("{}h ago", s / 3600),
        _ if s < 30 * 86_400 => format!("{}d ago", s / 86_400),
        _ if s < 365 * 86_400 => format!("{}mo ago", s / (30 * 86_400)),
        _ => format!("{}y ago", s / (365 * 86_400)),
    }
}

/// `8080→80/tcp, 9000/tcp`: sorted, with IPv4/IPv6 duplicates merged.
pub fn ports(ports: &[Port]) -> String {
    let mut keys: Vec<(u16, Option<u16>, &str)> = ports
        .iter()
        .map(|p| (p.private_port, p.public_port, p.kind.as_str()))
        .collect();
    keys.sort();
    keys.dedup();
    keys.iter()
        .map(|(private, public, kind)| match public {
            Some(public) => format!("{public}→{private}/{kind}"),
            None => format!("{private}/{kind}"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Cuts `s` to at most `max` characters, ending with `…` when cut.
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Escapes `@` so FLTK labels show it literally instead of as a symbol.
pub fn label(s: &str) -> String {
    s.replace('@', "@@")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn port(private: u16, public: Option<u16>, kind: &str) -> Port {
        Port {
            private_port: private,
            public_port: public,
            kind: kind.to_string(),
        }
    }

    #[test]
    fn names() {
        assert_eq!(container_name(&["/web".into()], "abc"), "web");
        assert_eq!(container_name(&[], "0123456789abcdef"), "0123456789ab");
    }

    #[test]
    fn relative_times() {
        let now = 1_000_000_000;
        assert_eq!(relative_time(now + 5, now), "just now"); // clock skew
        assert_eq!(relative_time(now - 59, now), "just now");
        assert_eq!(relative_time(now - 60, now), "1m ago");
        assert_eq!(relative_time(now - 3 * 3600, now), "3h ago");
        assert_eq!(relative_time(now - 2 * 86_400, now), "2d ago");
        assert_eq!(relative_time(now - 65 * 86_400, now), "2mo ago");
        assert_eq!(relative_time(now - 800 * 86_400, now), "2y ago");
    }

    #[test]
    fn ports_merge_and_sort() {
        let list = [
            port(80, Some(8080), "tcp"),
            port(9000, None, "tcp"),
            port(80, Some(8080), "tcp"), // IPv6 duplicate
            port(53, Some(5353), "udp"),
        ];
        assert_eq!(ports(&list), "5353→53/udp, 8080→80/tcp, 9000/tcp");
        assert_eq!(ports(&[]), "");
    }

    #[test]
    fn truncation() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("exactly10!", 10), "exactly10!");
        assert_eq!(truncate("much too long", 5), "much…");
        assert_eq!(truncate("ünïcødé", 4), "ünï…");
    }

    #[test]
    fn label_escapes_at() {
        assert_eq!(label("nginx@sha256:x"), "nginx@@sha256:x");
    }
}
