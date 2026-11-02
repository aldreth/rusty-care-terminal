/// Tiny ASCII say-box wrapper (party-parrot spirit without full ANSI art yet).
pub fn say_box(character: &str, message: &str, max_width: usize) -> String {
    let art = match character.to_ascii_lowercase().as_str() {
        "bunny" => BUNNY,
        "llama" => LLAMA,
        "cat" => CAT,
        _ => PARROT, // parrot + RANDOM default
    };

    let bubble = speech_bubble(message, max_width);
    format!("{bubble}\n{art}")
}

fn speech_bubble(message: &str, max_width: usize) -> String {
    // Inner text width: leave room for "< " + " >" and a little margin.
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

// Leading spaces preserved; each figure sits under the speech-bubble tail.
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
}
