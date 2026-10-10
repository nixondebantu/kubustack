//! Stateful demultiplexer for Docker log streams, and the log reader thread.

use super::{api, DockerError};
use crate::format;
use crate::msg::Msg;
use fltk::app::Sender;
use std::io::Read;
use std::path::PathBuf;

/// Turns raw log stream bytes into text. Without a TTY, Docker wraps output in frames:
/// an 8-byte header `[stream, 0, 0, 0, size: u32 BE]` then `size` bytes. Frames and UTF-8
/// sequences may be split across reads, so partial bytes are kept for the next chunk.
pub struct Demuxer {
    tty: bool,
    /// Unparsed frame bytes (non-TTY only).
    frame: Vec<u8>,
    /// Trailing bytes of an incomplete UTF-8 sequence.
    utf8: Vec<u8>,
}

impl Demuxer {
    pub fn new(tty: bool) -> Self {
        Demuxer {
            tty,
            frame: Vec::new(),
            utf8: Vec::new(),
        }
    }

    /// Feeds a chunk and returns the text it completes (possibly empty).
    pub fn push(&mut self, chunk: &[u8]) -> String {
        let mut payload = std::mem::take(&mut self.utf8);
        if self.tty {
            payload.extend_from_slice(chunk);
        } else {
            self.frame.extend_from_slice(chunk);
            let mut start = 0;
            while self.frame.len() - start >= 8 {
                let h = &self.frame[start..start + 8];
                let size = u32::from_be_bytes([h[4], h[5], h[6], h[7]]) as usize;
                if self.frame.len() - start - 8 < size {
                    break;
                }
                payload.extend_from_slice(&self.frame[start + 8..start + 8 + size]);
                start += 8 + size;
            }
            self.frame.drain(..start);
        }
        let cut = complete_utf8_len(&payload);
        self.utf8 = payload.split_off(cut);
        String::from_utf8_lossy(&payload).into_owned()
    }
}

/// Length of `b` without a trailing incomplete (but so far valid) UTF-8 sequence.
fn complete_utf8_len(b: &[u8]) -> usize {
    // A sequence is at most 4 bytes, so only the last 3 can be an unfinished one.
    for back in 1..=b.len().min(3) {
        let i = b.len() - back;
        let lead = b[i];
        if lead & 0xC0 == 0x80 {
            continue; // continuation byte, keep looking for the lead byte
        }
        let need = match lead {
            0xC0..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF7 => 4,
            _ => 1,
        };
        return if back < need { i } else { b.len() };
    }
    b.len()
}

/// Reads a container's logs on a new thread and sends `LogText` chunks, then `LogEnded`.
/// `follow` streams new lines until stopped; otherwise it loads the last 500 lines.
pub fn spawn_reader(
    socket: PathBuf,
    container: String,
    follow: bool,
    window: u32,
    stream: u32,
    s: Sender<Msg>,
) {
    let run = move || {
        let result = (|| -> Result<(), DockerError> {
            let tty = api::inspect_tty(&socket, &container)?;
            let (mut body, handle) = api::logs(&socket, &container, follow)?;
            if follow {
                s.send(Msg::LogFollowing {
                    window,
                    stream,
                    handle,
                });
            }
            let mut demux = Demuxer::new(tty);
            let mut buf = [0u8; 8192];
            loop {
                let n = match body.read(&mut buf) {
                    Ok(0) => return Ok(()),
                    Ok(n) => n,
                    Err(e) => return Err(DockerError::Io(e.to_string())),
                };
                // ponytail: an ANSI escape split across two reads may leave stray characters.
                let text = format::strip_ansi(&demux.push(&buf[..n]));
                if !text.is_empty() {
                    s.send(Msg::LogText { window, text });
                }
            }
        })();
        s.send(Msg::LogEnded {
            window,
            stream,
            error: result.err(),
        });
    };
    if let Err(e) = std::thread::Builder::new().name("logs".into()).spawn(run) {
        eprintln!("kubustack: couldn't start a log reader: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(stream: u8, payload: &[u8]) -> Vec<u8> {
        let mut f = vec![stream, 0, 0, 0];
        f.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        f.extend_from_slice(payload);
        f
    }

    #[test]
    fn single_and_multiple_frames() {
        let mut d = Demuxer::new(false);
        assert_eq!(d.push(&frame(1, b"hello\n")), "hello\n");
        let mut two = frame(1, b"out\n");
        two.extend(frame(2, b"err\n"));
        assert_eq!(d.push(&two), "out\nerr\n");
    }

    #[test]
    fn frames_split_at_every_byte() {
        let mut raw = frame(1, "héllo ✓\n".as_bytes());
        raw.extend(frame(2, "wörld 🎉\n".as_bytes()));
        raw.extend(frame(1, b""));
        for cut in 0..=raw.len() {
            let mut d = Demuxer::new(false);
            let text = d.push(&raw[..cut]) + &d.push(&raw[cut..]);
            assert_eq!(text, "héllo ✓\nwörld 🎉\n", "cut at {cut}");
        }
        // One byte at a time too.
        let mut d = Demuxer::new(false);
        let text: String = raw.iter().map(|b| d.push(&[*b])).collect();
        assert_eq!(text, "héllo ✓\nwörld 🎉\n");
    }

    #[test]
    fn tty_raw_stream_with_split_utf8() {
        let raw = "tty ✓ 🎉\n".as_bytes();
        for cut in 0..=raw.len() {
            let mut d = Demuxer::new(true);
            let text = d.push(&raw[..cut]) + &d.push(&raw[cut..]);
            assert_eq!(text, "tty ✓ 🎉\n", "cut at {cut}");
        }
    }

    #[test]
    fn invalid_utf8_is_replaced_not_held() {
        let mut d = Demuxer::new(true);
        assert_eq!(d.push(b"a\xFFb"), "a\u{FFFD}b");
    }
}
