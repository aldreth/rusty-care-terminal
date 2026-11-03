//! Keyboard input plus terminal colour-scheme reports.
//!
//! On Unix the tty is read directly: crossterm 0.29 can't parse the
//! `CSI ? 997 ; Ps n` reports that Ghostty, kitty and others send for DEC mode
//! 2031, and its parser swallows every later key once one arrives. Windows
//! keeps crossterm's reader, so the byte parser is unused there.
#![cfg_attr(not(unix), allow(dead_code))]

use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::theme::ResolvedTheme;

/// Asks for the terminal's current dark/light scheme and for a report whenever it changes.
pub const SUBSCRIBE_COLOR_SCHEME: &str = "\x1b[?996n\x1b[?2031h";
pub const UNSUBSCRIBE_COLOR_SCHEME: &str = "\x1b[?2031l";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Key(KeyEvent),
    ColorScheme(ResolvedTheme),
}

#[cfg(unix)]
pub use unix::InputSource;

#[cfg(not(unix))]
pub use crossterm_reader::InputSource;

#[cfg(unix)]
mod unix {
    use std::io::{self, Read};
    use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
    use std::thread;

    use super::{Duration, Input, Parser};

    pub struct InputSource {
        rx: Receiver<Input>,
    }

    impl InputSource {
        pub fn start() -> anyhow::Result<Self> {
            let (tx, rx) = mpsc::channel();
            thread::Builder::new().name("input".into()).spawn(move || {
                let mut stdin = io::stdin().lock();
                let mut parser = Parser::default();
                let mut buf = [0u8; 1024];
                loop {
                    let n = match stdin.read(&mut buf) {
                        Ok(0) => return,
                        Ok(n) => n,
                        Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                        Err(_) => return,
                    };
                    for input in parser.feed(&buf[..n]) {
                        if tx.send(input).is_err() {
                            return;
                        }
                    }
                }
            })?;
            Ok(Self { rx })
        }

        /// Waits up to `timeout` for the next input.
        pub fn next(&self, timeout: Duration) -> anyhow::Result<Option<Input>> {
            match self.rx.recv_timeout(timeout) {
                Ok(input) => Ok(Some(input)),
                Err(RecvTimeoutError::Timeout) => Ok(None),
                Err(RecvTimeoutError::Disconnected) => anyhow::bail!("terminal input closed"),
            }
        }
    }
}

#[cfg(not(unix))]
mod crossterm_reader {
    use crossterm::event::{self, Event};

    use super::{Duration, Input};

    pub struct InputSource;

    impl InputSource {
        pub fn start() -> anyhow::Result<Self> {
            Ok(Self)
        }

        pub fn next(&self, timeout: Duration) -> anyhow::Result<Option<Input>> {
            if event::poll(timeout)?
                && let Event::Key(key) = event::read()?
            {
                return Ok(Some(Input::Key(key)));
            }
            Ok(None)
        }
    }
}

/// An unfinished escape sequence longer than this is junk, not a slow terminal.
const MAX_PENDING: usize = 64;

/// Turns raw tty bytes into keys and colour-scheme reports, carrying sequences
/// split across reads over to the next one.
#[derive(Default)]
struct Parser {
    pending: Vec<u8>,
}

enum Step {
    Done(usize, Option<Input>),
    Incomplete,
}

impl Parser {
    fn feed(&mut self, bytes: &[u8]) -> Vec<Input> {
        let mut buf = std::mem::take(&mut self.pending);
        buf.extend_from_slice(bytes);
        let mut out = Vec::new();
        let mut at = 0;
        while at < buf.len() {
            match parse_one(&buf[at..]) {
                Step::Done(used, input) => {
                    out.extend(input);
                    at += used;
                }
                Step::Incomplete if buf.len() - at <= MAX_PENDING => {
                    self.pending = buf[at..].to_vec();
                    break;
                }
                Step::Incomplete => break,
            }
        }
        out
    }
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> Option<Input> {
    Some(Input::Key(KeyEvent::new(code, modifiers)))
}

fn parse_one(b: &[u8]) -> Step {
    const ESC: u8 = 0x1b;
    match b[0] {
        // A lone ESC at the end of a read is the Esc key, not the start of a sequence.
        ESC if b.len() == 1 => Step::Done(1, key(KeyCode::Esc, KeyModifiers::NONE)),
        ESC => match b[1] {
            b'[' => parse_csi(b),
            b'O' if b.len() < 3 => Step::Incomplete,
            b'O' => Step::Done(3, arrow(b[2])),
            c @ 0x20..=0x7e => Step::Done(2, key(KeyCode::Char(c as char), KeyModifiers::ALT)),
            _ => Step::Done(1, key(KeyCode::Esc, KeyModifiers::NONE)),
        },
        b'\t' => Step::Done(1, key(KeyCode::Tab, KeyModifiers::NONE)),
        b'\r' | b'\n' => Step::Done(1, key(KeyCode::Enter, KeyModifiers::NONE)),
        0x7f => Step::Done(1, key(KeyCode::Backspace, KeyModifiers::NONE)),
        c @ 0x01..=0x1a => Step::Done(
            1,
            key(KeyCode::Char((b'a' + c - 1) as char), KeyModifiers::CONTROL),
        ),
        c if c < 0x20 => Step::Done(1, None),
        c => {
            let width = match c {
                0xf0..=0xf7 => 4,
                0xe0..=0xef => 3,
                0xc0..=0xdf => 2,
                _ => 1,
            };
            if b.len() < width {
                return Step::Incomplete;
            }
            let ch = std::str::from_utf8(&b[..width])
                .ok()
                .and_then(|s| s.chars().next());
            Step::Done(
                width,
                ch.and_then(|ch| key(KeyCode::Char(ch), KeyModifiers::NONE)),
            )
        }
    }
}

/// `ESC [ params final`, where the final byte is in `0x40..=0x7e`.
fn parse_csi(b: &[u8]) -> Step {
    let Some(end) = b[2..].iter().position(|c| (0x40..=0x7e).contains(c)) else {
        return Step::Incomplete;
    };
    let end = end + 2;
    let params = &b[2..end];
    let input = match (params, b[end]) {
        (b"?997;1", b'n') => Some(Input::ColorScheme(ResolvedTheme::Dark)),
        (b"?997;2", b'n') => Some(Input::ColorScheme(ResolvedTheme::Light)),
        (b"", b'Z') => key(KeyCode::BackTab, KeyModifiers::SHIFT),
        (p, c) if !p.starts_with(b"?") => arrow(c),
        _ => None,
    };
    Step::Done(end + 1, input)
}

fn arrow(c: u8) -> Option<Input> {
    let code = match c {
        b'A' => KeyCode::Up,
        b'B' => KeyCode::Down,
        b'C' => KeyCode::Right,
        b'D' => KeyCode::Left,
        _ => return None,
    };
    key(code, KeyModifiers::NONE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(inputs: &[Input]) -> Vec<KeyCode> {
        inputs
            .iter()
            .filter_map(|i| match i {
                Input::Key(k) => Some(k.code),
                Input::ColorScheme(_) => None,
            })
            .collect()
    }

    #[test]
    fn reads_plain_and_special_keys() {
        let got = Parser::default().feed(b"qa\t\x1b[A\x1bOB\x1b[Z");
        assert_eq!(
            keys(&got),
            [
                KeyCode::Char('q'),
                KeyCode::Char('a'),
                KeyCode::Tab,
                KeyCode::Up,
                KeyCode::Down,
                KeyCode::BackTab,
            ]
        );
    }

    #[test]
    fn ctrl_c_and_lone_escape() {
        let got = Parser::default().feed(b"\x03");
        assert_eq!(
            got,
            [Input::Key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL
            ))]
        );
        assert_eq!(keys(&Parser::default().feed(b"\x1b")), [KeyCode::Esc]);
    }

    #[test]
    fn colour_scheme_reports_do_not_swallow_keys() {
        let got = Parser::default().feed(b"j\x1b[?997;2nk\x1b[?997;1nq");
        assert_eq!(
            got,
            [
                key(KeyCode::Char('j'), KeyModifiers::NONE).unwrap(),
                Input::ColorScheme(ResolvedTheme::Light),
                key(KeyCode::Char('k'), KeyModifiers::NONE).unwrap(),
                Input::ColorScheme(ResolvedTheme::Dark),
                key(KeyCode::Char('q'), KeyModifiers::NONE).unwrap(),
            ]
        );
    }

    #[test]
    fn sequences_split_across_reads_are_joined() {
        let mut parser = Parser::default();
        assert!(parser.feed(b"\x1b[?99").is_empty());
        assert_eq!(
            parser.feed(b"7;1nr"),
            [
                Input::ColorScheme(ResolvedTheme::Dark),
                key(KeyCode::Char('r'), KeyModifiers::NONE).unwrap(),
            ]
        );
    }

    #[test]
    fn unknown_reports_are_dropped_and_junk_is_bounded() {
        assert_eq!(
            keys(&Parser::default().feed(b"\x1b[?2026;2$yq")),
            [KeyCode::Char('q')]
        );
        let mut parser = Parser::default();
        let mut junk = b"\x1b[".to_vec();
        junk.extend(std::iter::repeat_n(b'1', 100));
        assert!(parser.feed(&junk).is_empty());
        assert_eq!(keys(&parser.feed(b"q")), [KeyCode::Char('q')]);
    }
}
