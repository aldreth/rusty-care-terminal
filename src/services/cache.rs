use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::{BotMessage, clean_text};

/// Last good message per bot plus the shared rate-limit backoff.
/// Every operation is best-effort: a broken cache only costs the offline fallback.
pub struct Cache {
    root: PathBuf,
}

impl Cache {
    /// `$XDG_CACHE_HOME/rusty-care-terminal`, else `~/.cache/rusty-care-terminal`.
    pub fn default_location() -> Option<Self> {
        let base = env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| env::home_dir().map(|h| h.join(".cache")))?;
        Some(Self::at(base.join("rusty-care-terminal")))
    }

    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn message_path(&self, key: &str) -> PathBuf {
        let safe: String = key
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        self.root.join("bots").join(format!("{safe}.txt"))
    }

    fn backoff_path(&self) -> PathBuf {
        self.root.join("backoff_until.txt")
    }

    pub fn save_message(&self, key: &str, msg: &BotMessage) {
        write_atomic(
            &self.message_path(key),
            &format!("{}\n{}", msg.source_label, msg.text),
        );
    }

    pub fn load_message(&self, key: &str) -> Option<BotMessage> {
        let text = fs::read_to_string(self.message_path(key)).ok()?;
        let (label, body) = text.split_once('\n')?;
        let body = clean_text(body);
        if body.is_empty() {
            return None;
        }
        // Labels never contain spaces; older builds appended " → url" / " · cached".
        let label = clean_text(label);
        Some(BotMessage {
            source_label: label.split(' ').next().unwrap_or_default().to_string(),
            text: body,
        })
    }

    pub fn set_backoff_secs(&self, secs: u64) {
        let until = now_secs().saturating_add(secs);
        write_atomic(&self.backoff_path(), &until.to_string());
    }

    pub fn backoff_active(&self) -> bool {
        let active = fs::read_to_string(self.backoff_path())
            .ok()
            .and_then(|raw| raw.trim().parse::<u64>().ok())
            .is_some_and(|until| now_secs() < until);
        if !active {
            let _ = fs::remove_file(self.backoff_path());
        }
        active
    }
}

fn write_atomic(path: &Path, body: &str) {
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    if fs::write(&tmp, body).is_ok() && fs::rename(&tmp, path).is_err() {
        let _ = fs::remove_file(&tmp);
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn temp_cache(name: &str) -> Cache {
        let root = env::temp_dir().join(format!("rct-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        Cache::at(root)
    }

    #[test]
    fn roundtrip_message_cache() {
        let cache = temp_cache("roundtrip");
        let msg = BotMessage {
            source_label: "@bot@example.com".into(),
            text: "Drink water".into(),
        };
        cache.save_message("@bot@example.com", &msg);
        assert_eq!(cache.load_message("@bot@example.com"), Some(msg));
        let _ = fs::remove_dir_all(&cache.root);
    }

    #[test]
    fn load_cleans_control_chars() {
        let cache = temp_cache("clean");
        let path = cache.message_path("k");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "@a@b.c\nhi\u{1b}]52;c;bad\u{7} there").unwrap();
        let loaded = cache.load_message("k").unwrap();
        assert!(!loaded.text.contains('\u{1b}'));
        let _ = fs::remove_dir_all(&cache.root);
    }

    #[test]
    fn backoff_expires() {
        let cache = temp_cache("backoff");
        assert!(!cache.backoff_active());
        cache.set_backoff_secs(60);
        assert!(cache.backoff_active());
        cache.set_backoff_secs(0);
        assert!(!cache.backoff_active());
        let _ = fs::remove_dir_all(&cache.root);
    }

    #[test]
    fn message_path_is_under_cache() {
        let p = Cache::at("/x").message_path("a@b.c");
        assert!(p.ends_with(Path::new("bots").join("a_b_c.txt")));
    }
}
