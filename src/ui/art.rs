use std::fs;
use std::path::Path;

use std::time::{SystemTime, UNIX_EPOCH};

/// Tiny ASCII say-box wrapper (party-parrot spirit).
pub fn say_box(character: &str, message: &str, max_width: usize) -> String {
    let art = resolve_art(character);
    let bubble = speech_bubble(message, max_width);
    format!("{bubble}\n{art}")
}

fn resolve_art(character: &str) -> String {
    let trimmed = character.trim();
    if trimmed.is_empty() {
        return PARROT.to_string();
    }
    // External art file (.cow / .ansi / plain text).
    if Path::new(trimmed).is_file() {
        return fs::read_to_string(trimmed)
            .unwrap_or_else(|_| PARROT.to_string())
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
        _ => PARROT.to_string(),
    }
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

    let top = format!(" {}", "-".repeat(width + 2));
    let bot = format!(" {}", "-".repeat(width + 2));
    let mut lines = vec![top];
    for (i, row) in wrapped.iter().enumerate() {
        let edge = if wrapped.len() == 1 {
            ("<", ">")
        } else if i == 0 {
            ("/", "\\")
        } else if i + 1 == wrapped.len() {
            ("\\", "/")
        } else {
            ("|", "|")
        };
        lines.push(format!("{} {row:<width$} {}", edge.0, edge.1));
    }
    lines.push(bot);
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

const PARROT: &str = r#"
 \
  \   .--.
    \/ o)\
    /\__/ \
   //  \_\
  //    ||
  ||    ||
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
        let out = say_box("parrot", msg, 60);
        assert!(!out.contains("..."), "unexpected truncation: {out}");
        assert!(out.contains("healthy"));
        assert!(out.lines().count() > 4);
    }

    #[test]
    fn random_resolves_to_known_art() {
        let out = say_box("RANDOM", "hi", 40);
        assert!(out.contains('<') || out.contains('/'));
    }
}
