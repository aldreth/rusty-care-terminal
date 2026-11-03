use std::fs;
use std::path::Path;

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::theme::Theme;
use crate::ui::sprites::{self, Sprite};

const MARGIN: usize = 1;
const TAIL_ROWS: usize = 2;
const MAX_BUBBLE_TEXT: usize = 56;

/// Speech bubble with a `\` tail landing on a pixel creature's head.
///
/// Built-ins: `parrot` (default), `cat`, `bunny`, `llama`, `RANDOM`.
/// Custom: path to a `.png`/`.jpg`/`.jpeg` (pixel-rendered) or a text/`.cow` file.
/// Lines are pre-laid-out for `max_width` × `max_height` and must not be re-wrapped.
pub fn say_box_lines(
    character: &str,
    message: &str,
    max_width: usize,
    max_height: usize,
    theme: Theme,
) -> Vec<Line<'static>> {
    let style = Style::default().fg(theme.fg);
    let text_width = max_width
        .saturating_sub(MARGIN + 4)
        .clamp(1, MAX_BUBBLE_TEXT);
    let wrapped = wrap_text(message, text_width);
    let bubble_rows = wrapped.len() + 2;

    let mut lines = Vec::new();
    let art_rows = max_height.saturating_sub(bubble_rows + TAIL_ROWS);
    let art_cols = max_width.saturating_sub(MARGIN);

    match resolve_art(character, message, art_cols, art_rows) {
        Art::Sprite(variants) => {
            if let Some(placed) = sprites::fit(&variants, art_cols, art_rows) {
                // Tail cells sit at `tail` and `tail + 1`; the head's top-left
                // outline pixel is one cell further right, so the diagonal meets it.
                let min_tail = MARGIN + 2;
                let mut left = (MARGIN + 1).max((min_tail + 2).saturating_sub(placed.anchor_col));
                if left + placed.cols > max_width {
                    left = max_width.saturating_sub(placed.cols);
                }
                let tail = (left + placed.anchor_col).saturating_sub(2);
                let min_inner = (tail + 3).saturating_sub(MARGIN + 4);
                push_bubble(&mut lines, &wrapped, min_inner, style);
                lines.push(Line::from(Span::styled(
                    format!("{}\\", " ".repeat(tail)),
                    style,
                )));
                lines.push(Line::from(Span::styled(
                    format!("{}\\", " ".repeat(tail + 1)),
                    style,
                )));
                let pad = " ".repeat(left);
                for line in placed.lines {
                    let mut spans = vec![Span::raw(pad.clone())];
                    spans.extend(line.spans);
                    lines.push(Line::from(spans));
                }
            } else {
                push_bubble(&mut lines, &wrapped, 0, style);
            }
        }
        Art::Text(rows) => {
            push_bubble(&mut lines, &wrapped, 0, style);
            let pad = " ".repeat(MARGIN);
            lines.extend(
                rows.into_iter()
                    .take(max_height.saturating_sub(bubble_rows))
                    .map(|row| {
                        let clipped: String = row.chars().take(art_cols).collect();
                        Line::from(Span::styled(format!("{pad}{clipped}"), style))
                    }),
            );
        }
    }

    lines.truncate(max_height.max(1));
    lines
}

enum Art {
    Sprite(Vec<Sprite>),
    Text(Vec<String>),
}

fn resolve_art(character: &str, message: &str, max_cols: usize, max_rows: usize) -> Art {
    let trimmed = character.trim();
    let path = Path::new(trimmed);
    if !trimmed.is_empty() && path.is_file() {
        return load_art_file(path, max_cols, max_rows);
    }

    let key = trimmed.to_ascii_lowercase();
    let name = if key == "random" {
        // Stable per message so the creature doesn't change on every redraw.
        let n: usize = message.bytes().map(usize::from).sum();
        sprites::CREATURES[n % sprites::CREATURES.len()]
    } else {
        key.as_str()
    };
    Art::Sprite(sprites::creature(name))
}

fn load_art_file(path: &Path, max_cols: usize, max_rows: usize) -> Art {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if matches!(ext.as_str(), "png" | "jpg" | "jpeg")
        && let Ok(img) = image::open(path)
    {
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let sprite = Sprite::from_rgba(rgba.as_raw(), w, h, (max_cols, max_rows * 2));
        return Art::Sprite(vec![sprite]);
    }

    match fs::read_to_string(path) {
        Ok(raw) => Art::Text(strip_ansi(&raw).lines().map(str::to_string).collect()),
        Err(_) => Art::Sprite(sprites::creature("parrot")),
    }
}

fn push_bubble(lines: &mut Vec<Line<'static>>, wrapped: &[String], min_inner: usize, style: Style) {
    let width = wrapped
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0)
        .max(min_inner);
    let pad = " ".repeat(MARGIN);
    let edge = format!("{pad}+{}+", "-".repeat(width + 2));
    lines.push(Line::from(Span::styled(edge.clone(), style)));
    for row in wrapped {
        lines.push(Line::from(Span::styled(
            format!("{pad}| {row:<width$} |"),
            style,
        )));
    }
    lines.push(Line::from(Span::styled(edge, style)));
}

fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(ch);
    }
    out
}

fn wrap_text(message: &str, width: usize) -> Vec<String> {
    let mut rows = Vec::new();
    let mut current = String::new();
    for word in message.split_whitespace() {
        if word.chars().count() > width {
            if !current.is_empty() {
                rows.push(std::mem::take(&mut current));
            }
            let mut chunk = String::new();
            for ch in word.chars() {
                if chunk.chars().count() >= width {
                    rows.push(std::mem::take(&mut chunk));
                }
                chunk.push(ch);
            }
            if !chunk.is_empty() {
                current = chunk;
            }
            continue;
        }
        let next_len = if current.is_empty() {
            word.chars().count()
        } else {
            current.chars().count() + 1 + word.chars().count()
        };
        if next_len > width && !current.is_empty() {
            rows.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        rows.push(current);
    }
    if rows.is_empty() {
        rows.push(String::new());
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn wraps_long_message_instead_of_ellipsis() {
        let msg = "Make yourself a healthy meal. It doesn't have to be fancy to count.";
        let out = say_box_lines("parrot", msg, 60, 20, Theme::dark());
        let joined: Vec<String> = out.iter().map(text_of).collect();
        assert!(!joined.join("\n").contains("..."));
        assert!(joined.join("\n").contains("healthy"));
    }

    #[test]
    fn random_resolves_to_known_art() {
        let out = say_box_lines("RANDOM", "hi", 40, 16, Theme::dark());
        assert!(out.len() > 4);
    }

    #[test]
    fn never_exceeds_the_area() {
        for (w, h) in [(20, 6), (30, 10), (44, 12), (80, 24), (120, 40), (12, 3)] {
            for name in ["parrot", "cat", "bunny", "llama"] {
                let out = say_box_lines(
                    name,
                    "please remember to breathe deeply",
                    w,
                    h,
                    Theme::dark(),
                );
                assert!(out.len() <= h.max(1), "{name} {w}x{h} rows {}", out.len());
                for line in &out {
                    let cols: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
                    assert!(cols <= w, "{name} {w}x{h} line {cols} cols");
                }
            }
        }
    }

    #[test]
    fn tail_lands_on_the_head() {
        for (name, w, h) in [
            ("parrot", 60, 24),
            ("parrot", 40, 14),
            ("cat", 60, 30),
            ("bunny", 40, 16),
            ("llama", 70, 30),
        ] {
            let out = say_box_lines(name, "breathe", w, h, Theme::dark());
            let texts: Vec<String> = out.iter().map(text_of).collect();
            let tail = texts.iter().position(|l| l.trim() == "\\").expect("tail");
            let tail2_col = texts[tail + 1].chars().position(|c| c == '\\').unwrap();
            assert_eq!(
                texts[tail].chars().position(|c| c == '\\').unwrap() + 1,
                tail2_col
            );
            // The diagonal ends at the bottom-right corner of its cell, so the head's
            // top-left pixel must fill the top half of the next cell down-right.
            let below = texts[tail + 2].chars().nth(tail2_col + 1).unwrap();
            assert!(matches!(below, '█' | '▀'), "{name} {w}x{h}: got {below:?}");
        }
    }

    #[test]
    fn tail_starts_inside_the_bubble() {
        for name in ["parrot", "cat", "bunny", "llama"] {
            let out = say_box_lines(name, "hi", 80, 30, Theme::dark());
            let texts: Vec<String> = out.iter().map(text_of).collect();
            let bottom = texts
                .iter()
                .rposition(|l| l.trim_start().starts_with('+'))
                .unwrap();
            let edge = &texts[bottom];
            let start = edge.find('+').unwrap();
            let end = edge.rfind('+').unwrap();
            let tail = texts[bottom + 1].find('\\').expect("tail");
            assert!(
                tail > start && tail < end,
                "{name}: tail {tail} outside {start}..{end}"
            );
        }
    }
}
