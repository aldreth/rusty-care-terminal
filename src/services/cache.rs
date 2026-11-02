use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Local;

use crate::model::CareMessage;

fn cache_root() -> PathBuf {
    let base = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(shellexpand::tilde("~/.cache").into_owned()));
    base.join("rusty-care-terminal")
}

fn care_path(key: &str) -> PathBuf {
    let safe: String = key
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    cache_root().join("care").join(format!("{safe}.txt"))
}

fn backoff_path() -> PathBuf {
    cache_root().join("backoff_until.txt")
}

pub fn save_care(key: &str, msg: &CareMessage) {
    let path = care_path(key);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let body = format!("{}\n{}", msg.source_label, msg.text);
    let _ = fs::write(path, body);
}

pub fn load_care(key: &str) -> Option<CareMessage> {
    let text = fs::read_to_string(care_path(key)).ok()?;
    let mut lines = text.lines();
    let label = lines.next()?.to_string();
    let body = lines.collect::<Vec<_>>().join("\n").trim().to_string();
    if body.is_empty() {
        return None;
    }
    Some(CareMessage {
        source_label: format!("{label} · cached"),
        text: body,
        fetched_at: Local::now(),
        from_mock: true,
    })
}

pub fn set_backoff_secs(secs: u64) {
    let until = now_secs().saturating_add(secs);
    let path = backoff_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, until.to_string());
}

pub fn backoff_active() -> bool {
    let Ok(raw) = fs::read_to_string(backoff_path()) else {
        return false;
    };
    let Ok(until) = raw.trim().parse::<u64>() else {
        return false;
    };
    now_secs() < until
}

pub fn clear_backoff_if_expired() {
    if !backoff_active() {
        let _ = fs::remove_file(backoff_path());
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_care_cache() {
        let key = "test_cache_key_rct";
        let msg = CareMessage {
            source_label: "@bot@example.com".into(),
            text: "Drink water".into(),
            fetched_at: Local::now(),
            from_mock: false,
        };
        save_care(key, &msg);
        let loaded = load_care(key).expect("cached");
        assert!(loaded.text.contains("Drink water"));
        assert!(loaded.source_label.contains("cached"));
        let _ = fs::remove_file(care_path(key));
    }

    #[test]
    fn care_path_is_under_cache() {
        let p = care_path("a@b.c");
        assert!(p.to_string_lossy().ends_with("care/a_b_c.txt"));
    }
}
