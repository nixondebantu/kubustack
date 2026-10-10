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

/// Size in decimal units like the Docker CLI: `512 B`, `7.8 MB`, `1.2 GB`.
pub fn size(bytes: i64) -> String {
    const UNITS: [&str; 5] = ["B", "kB", "MB", "GB", "TB"];
    let mut v = bytes.max(0) as f64;
    let mut unit = 0;
    // 999.95 so values that round to "1000.0" move up a unit.
    while v >= 999.95 && unit < UNITS.len() - 1 {
        v /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{v} B")
    } else {
        format!("{v:.1} {}", UNITS[unit])
    }
}

/// Removes ANSI escape sequences (colors, cursor moves, titles), carriage returns and other
/// control characters except newline and tab.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\x1b' => match chars.next() {
                // CSI: ESC [ params... final byte in @..~
                Some('[') => {
                    for c in chars.by_ref() {
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                // OSC: ESC ] ... ended by BEL or ESC \
                Some(']') => {
                    while let Some(c) = chars.next() {
                        if c == '\x07' || (c == '\x1b' && chars.next_if_eq(&'\\').is_some()) {
                            break;
                        }
                    }
                }
                // nF escapes like ESC ( B: intermediate bytes, then one final byte
                Some(' '..='/') => {
                    while chars.next_if(|c| (' '..='/').contains(c)).is_some() {}
                    chars.next();
                }
                _ => {} // two-character escape
            },
            '\n' | '\t' => out.push(c),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
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
    fn sizes() {
        assert_eq!(size(0), "0 B");
        assert_eq!(size(999), "999 B");
        assert_eq!(size(1000), "1.0 kB");
        assert_eq!(size(999_960), "1.0 MB");
        assert_eq!(size(14_555_608), "14.6 MB");
        assert_eq!(size(1_234_000_000), "1.2 GB");
        assert_eq!(size(5_000_000_000_000_000), "5000.0 TB");
        assert_eq!(size(-5), "0 B");
    }

    #[test]
    fn strips_ansi() {
        assert_eq!(strip_ansi("\x1b[1;31mred\x1b[0m plain"), "red plain");
        assert_eq!(strip_ansi("\x1b]0;title\x07after"), "after");
        assert_eq!(strip_ansi("\x1b]8;;http://x\x1b\\link"), "link");
        assert_eq!(strip_ansi("line\r\nnext\ttab\x08"), "line\nnext\ttab");
        assert_eq!(strip_ansi("\x1b(Bok ünï ✓"), "ok ünï ✓");
        assert_eq!(strip_ansi("cut\x1b[3"), "cut");
    }

    #[test]
    fn label_escapes_at() {
        assert_eq!(label("nginx@sha256:x"), "nginx@@sha256:x");
    }
}
