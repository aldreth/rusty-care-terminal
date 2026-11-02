/// Tiny ASCII say-box wrapper (party-parrot spirit without full ANSI art yet).
pub fn say_box(character: &str, message: &str) -> String {
    let art = match character.to_ascii_lowercase().as_str() {
        "bunny" => BUNNY,
        "llama" => LLAMA,
        "cat" => CAT,
        _ => PARROT, // parrot + RANDOM default
    };

    let bubble = speech_bubble(message);
    format!("{bubble}\n{art}")
}

fn speech_bubble(message: &str) -> String {
    let text = if message.chars().count() > 48 {
        let truncated: String = message.chars().take(45).collect();
        format!("{truncated}...")
    } else {
        message.to_string()
    };
    let width = text.chars().count().max(8);
    let top = format!(" {}", "-".repeat(width + 2));
    let mid = format!("< {text:<width$} >");
    let bot = format!(" {}", "-".repeat(width + 2));
    format!("{top}\n{mid}\n{bot}")
}

const PARROT: &str = r#"
       \ 
        \  .--.
          /   o\
         |  ___/
          \_/ /
          _/ /
         / _/
        // /
       /_ /
"#;

const BUNNY: &str = r#"
       \ 
        \   (\(\
             (-.-)
            o_(")(")
"#;

const LLAMA: &str = r#"
       \ 
        \    ∿
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
