use chrono::Local;

use crate::config::Config;
use crate::model::CareMessage;

const FALLBACK: &[&str] = &[
    "Hi! You're doing great!!!",
    "Drink some water. Your future self says thanks.",
    "Stretch those shoulders — tiny pauses count.",
    "One commit is still a win. Celebrate softly.",
    "Be gentle with yourself today.",
];

/// Build mock care messages for the three panels (RSS fetch lands in Milestone 2).
pub fn mock_care_messages(cfg: &Config) -> Vec<CareMessage> {
    let sources = cfg.care_sources();
    let now = Local::now();

    (0..3)
        .map(|i| {
            let label = sources
                .get(i)
                .map(|s| {
                    // Surface the intended RSS endpoint even while content is mocked.
                    match s.rss_url() {
                        Some(url) => format!("{} → {url}", s.label()),
                        None => s.label(),
                    }
                })
                .unwrap_or_else(|| format!("care-{}", i + 1));
            let text = FALLBACK[i % FALLBACK.len()].to_string();
            CareMessage {
                source_label: label,
                text,
                fetched_at: now,
                from_mock: true,
            }
        })
        .collect()
}
