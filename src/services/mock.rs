use crate::config::Config;
use crate::model::{CARE_SLOTS, CareMessage};

pub const DEFAULT_MESSAGE: &str = "Hi! You're doing great!!!";

const FALLBACK: &[&str] = &[
    DEFAULT_MESSAGE,
    "Drink some water. Your future self says thanks.",
    "Stretch those shoulders — tiny pauses count.",
    "One commit is still a win. Celebrate softly.",
    "Be gentle with yourself today.",
];

/// Offline kindness for care slot `index`, labelled with the configured account (if any).
pub fn fallback_message(cfg: &Config, index: usize) -> CareMessage {
    CareMessage {
        source_label: cfg
            .care_sources()
            .get(index)
            .map(|s| s.label())
            .unwrap_or_default(),
        text: FALLBACK[index % FALLBACK.len()].to_string(),
    }
}

pub fn mock_care_messages(cfg: &Config) -> Vec<CareMessage> {
    (0..CARE_SLOTS).map(|i| fallback_message(cfg, i)).collect()
}
