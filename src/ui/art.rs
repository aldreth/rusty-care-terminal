use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::theme::Theme;
use crate::ui::sprites;

/// Speech bubble + chunky half-block pixel character (or a custom image/text file).
///
/// Built-ins are hand-pixelled sprites (`parrot`, `bunny`, `llama`, `cat`, `RANDOM`)
/// scaled only by integer factors so they stay blocky. Custom: path to a
/// `.png`/`.jpg`/`.jpeg` (half-block rendered) or any text/`.ansi`/`.cow` file.
pub fn say_box_lines(
    character: &str,
    message: &str,
    max_width: usize,
    max_height: usize,
    theme: Theme,
) -> Vec<Line<'static>> {
    let bubble_width = max_width.saturating_sub(2).max(16);
    let bubble = speech_bubble(message, bubble_width);
    let bubble_lines: Vec<Line<'static>> = bubble
        .lines()
        .map(|row| Line::from(Span::styled(row.to_string(), Style::default().fg(theme.fg))))
        .collect();

    let bubble_h = bubble_lines.len();
    let art_rows = max_height.saturating_sub(bubble_h).max(5);
    let art_cols = max_width.saturating_sub(1).max(12);

    let mut lines = bubble_lines;
    lines.extend(resolve_art_lines(character, art_cols, art_rows, theme));
    lines
}

fn resolve_art_lines(
    character: &str,
    max_cols: usize,
    max_rows: usize,
    theme: Theme,
) -> Vec<Line<'static>> {
    let trimmed = character.trim();
    if trimmed.is_empty() {
        return sprites::sprite_lines("parrot", max_cols, max_rows);
    }

    let path = Path::new(trimmed);
    if path.is_file() {
        return load_art_file(path, max_cols, max_rows, theme);
    }

    let key = trimmed.to_ascii_lowercase();
    let choice = if key == "random" {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        ["parrot", "bunny", "llama", "cat"][(n as usize) % 4]
    } else {
        key.as_str()
    };

    match choice {
        "bunny" | "llama" | "cat" | "parrot" => sprites::sprite_lines(choice, max_cols, max_rows),
        _ => sprites::sprite_lines("parrot", max_cols, max_rows),
    }
}

fn load_art_file(
    path: &Path,
    max_cols: usize,
    max_rows: usize,
    theme: Theme,
) -> Vec<Line<'static>> {
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
        return sprites::rgba_to_halfblocks(rgba.as_raw(), w, h, max_cols, max_rows);
    }

    // Text / cowsay / ansi-ish: show as monochrome lines (ANSI escapes stripped lightly).
    match fs::read_to_string(path) {
        Ok(raw) => {
            let cleaned = strip_ansi(&raw);
            cleaned
                .lines()
                .take(max_rows)
                .map(|row| {
                    let clipped: String = row.chars().take(max_cols).collect();
                    Line::from(Span::styled(clipped, Style::default().fg(theme.bar)))
                })
                .collect()
        }
        Err(_) => sprites::sprite_lines("parrot", max_cols, max_rows),
    }
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

fn speech_bubble(message: &str, max_width: usize) -> String {
    let inner = max_width.saturating_sub(4).max(12);
    let wrapped = wrap_text(message, inner);
    let width = wrapped
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(8)
        .max(8)
        .min(inner);

    let top = format!("+{}+", "-".repeat(width + 2));
    let bot = format!("+{}+", "-".repeat(width + 2));
    let mut lines = vec![top];
    for row in &wrapped {
        lines.push(format!("| {row:<width$} |"));
    }
    lines.push(bot);
    lines.push("        \\".into());
    lines.push("         \\".into());
    lines.join("\n")
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

    #[test]
    fn wraps_long_message_instead_of_ellipsis() {
        let msg = "Make yourself a healthy meal. It doesn't have to be fancy to count.";
        let theme = Theme::dark();
        let out = say_box_lines("parrot", msg, 60, 20, theme);
        let joined: String = out
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!joined.contains("..."), "unexpected truncation: {joined}");
        assert!(joined.contains("healthy"));
        assert!(out.len() > 4);
    }

    #[test]
    fn random_resolves_to_known_art() {
        let theme = Theme::dark();
        let out = say_box_lines("RANDOM", "hi", 40, 16, theme);
        assert!(!out.is_empty());
    }
}
