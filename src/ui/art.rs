use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::theme::Theme;

/// Speech bubble + character as styled lines, sized for the say-box.
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
    let art_budget = max_height.saturating_sub(bubble_h).max(4);
    let art = resolve_art(character, art_budget);
    let mut lines = bubble_lines;
    lines.extend(colorize_art(&art, theme));
    lines
}

fn resolve_art(character: &str, height_budget: usize) -> String {
    let trimmed = character.trim();
    if trimmed.is_empty() {
        return parrot_for_height(height_budget).to_string();
    }
    if Path::new(trimmed).is_file() {
        return fs::read_to_string(trimmed)
            .unwrap_or_else(|_| parrot_for_height(height_budget).to_string())
            .trim_end()
            .to_string();
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
        "bunny" => BUNNY.to_string(),
        "llama" => LLAMA.to_string(),
        "cat" => CAT.to_string(),
        _ => parrot_for_height(height_budget).to_string(),
    }
}

fn parrot_for_height(height_budget: usize) -> &'static str {
    if height_budget >= 12 {
        PARROT_LARGE
    } else if height_budget >= 9 {
        PARROT_MEDIUM
    } else {
        PARROT_SMALL
    }
}

/// Color party-parrot ASCII: cyan body, magenta beak-ish glyphs, reset outline.
fn colorize_art(art: &str, theme: Theme) -> Vec<Line<'static>> {
    art.lines()
        .skip_while(|l| l.is_empty())
        .map(|row| {
            let spans: Vec<Span<'static>> = row
                .chars()
                .map(|ch| {
                    let color = match ch {
                        'o' | 'O' | '@' => theme.time_fresh,
                        '>' | '<' | '=' | '~' | '"' | '\'' => theme.hash,
                        '.' | '-' | '_' | '/' | '\\' | '|' | '(' | ')' => theme.border,
                        ' ' => theme.fg,
                        _ => theme.bar, // body fill
                    };
                    Span::styled(ch.to_string(), Style::default().fg(color))
                })
                .collect();
            Line::from(spans)
        })
        .collect()
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

const PARROT_SMALL: &str = r#"
  \
   \   .--.
     \/ o)\
     /\__/ \
    //  \_\
   //    ||
   ||    ||
"#;

const PARROT_MEDIUM: &str = r#"
   \
    \     .---.
     \   / o o \
      \  |  >  |
       \/ \___/
       /\  \_/ \
      // \/   \ \
     ||  |     | |
     ||  |     | |
      \\/       \/
"#;

const PARROT_LARGE: &str = r#"
    \
     \        .------.
      \      /  .--.  \
       \    /  /    \  \
        \   |  | oo |  |
         \  |  |  > |  |
          \ |  '----'  /
           \/   ||    /
           /\   ||   \
          // \  ||  / \
         //   \ || /   \
        ||     \||/     |
        ||      \/      |
         \\            /
          \\          /
           \/\/\/\/\/
"#;

const BUNNY: &str = r#"
 \
  \  (\(\
      (-.-)
     o_(")(")
"#;

const LLAMA: &str = r#"
 \
  \   ~~
     (oo)
    /||||\
   / |||| \
"#;

const CAT: &str = r#"
 \
  \  /\_/\
    ( o.o )
     > ^ <
"#;

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
