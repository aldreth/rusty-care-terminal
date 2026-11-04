use crate::config::Config;
use crate::model::{BOT_SLOTS, BotMessage};

pub const DEFAULT_MESSAGE: &str = "Hi! You're doing great!!!";

const FALLBACK: &[&str] = &[
    DEFAULT_MESSAGE,
    "Drink some water. Your future self says thanks.",
    "Stretch those shoulders — tiny pauses count.",
    "One commit is still a win. Celebrate softly.",
    "Be gentle with yourself today.",
];

/// Offline kindness for bot slot `index`, labelled with the configured bot (if any).
pub fn fallback_message(cfg: &Config, index: usize) -> BotMessage {
    BotMessage {
        source_label: cfg.bots.get(index).map(|s| s.label()).unwrap_or_default(),
        title: None,
        text: FALLBACK[index % FALLBACK.len()].to_string(),
    }
}

pub fn mock_messages(cfg: &Config) -> Vec<BotMessage> {
    (0..BOT_SLOTS).map(|i| fallback_message(cfg, i)).collect()
}
