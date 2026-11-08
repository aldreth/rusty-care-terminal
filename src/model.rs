use chrono::{DateTime, Local};

/// Number of bot slots on screen: the animal box plus two side panels.
pub const BOT_SLOTS: usize = 3;

/// Animal in lower right panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Animal {
    Parrot,
    Cat,
    Bunny,
    Llama,
}

impl Animal {
    pub const ALL: [Animal; 4] = [Animal::Parrot, Animal::Cat, Animal::Bunny, Animal::Llama];

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "parrot" => Some(Animal::Parrot),
            "cat" => Some(Animal::Cat),
            "bunny" => Some(Animal::Bunny),
            "llama" => Some(Animal::Llama),
            _ => None,
        }
    }

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|a| *a == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    /// Stable pick for `RANDOM`, so the creature only changes with the message.
    pub fn for_message(message: &str) -> Self {
        let n: usize = message.bytes().map(usize::from).sum();
        Self::ALL[n % Self::ALL.len()]
    }
}

/// How a bot is addressed (`@user@instance` or an explicit feed URL).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotSource {
    /// `@user@instance` or `user@instance` → resolve to `https://{host}/@{user}.rss`
    Acct(String),
    /// Explicit RSS/Atom URL
    FeedUrl(String),
}

impl BotSource {
    pub fn parse(raw: &str) -> Self {
        let trimmed = raw.trim();
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            BotSource::FeedUrl(trimmed.to_string())
        } else {
            let acct = trimmed.trim_start_matches('@').to_string();
            BotSource::Acct(acct)
        }
    }

    /// RSS endpoint for this source; `None` when an account is not a valid `user@host`.
    pub fn rss_url(&self) -> Option<String> {
        match self {
            BotSource::FeedUrl(url) => Some(url.clone()),
            BotSource::Acct(acct) => {
                let (user, host) = split_acct(acct)?;
                Some(format!("https://{host}/@{user}.rss"))
            }
        }
    }

    pub fn label(&self) -> String {
        match self {
            BotSource::FeedUrl(url) => url.clone(),
            BotSource::Acct(acct) => format!("@{acct}"),
        }
    }
}

/// Splits `user@host`, rejecting anything that could escape the URL path or a
/// terminal hyperlink (only Mastodon username and hostname characters pass).
pub fn split_acct(acct: &str) -> Option<(&str, &str)> {
    let (user, host) = acct.split_once('@')?;
    let user_ok = !user.is_empty()
        && user
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'));
    let host_ok = !host.is_empty()
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':'));
    (user_ok && host_ok).then_some((user, host))
}

/// Drops control characters (escape sequences included) and collapses whitespace,
/// so remote text can never drive the terminal.
pub fn clean_text(raw: &str) -> String {
    raw.split(|c: char| c.is_whitespace() || c.is_control())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Remote URLs end up inside terminal escape sequences, so only plain absolute
/// http(s) URLs without whitespace or control characters are kept.
pub fn safe_link(raw: &str) -> Option<String> {
    let url = raw.trim();
    let web = url.starts_with("https://") || url.starts_with("http://");
    let plain = url.len() <= 2048 && !url.chars().any(|c| c.is_whitespace() || c.is_control());
    (web && plain).then(|| url.to_string())
}

/// `@user@host` / `user@host` → `https://{host}/@{user}`.
pub fn profile_url_from_acct(acct_or_label: &str) -> Option<String> {
    let acct = acct_or_label.trim().trim_start_matches('@');
    let (user, host) = split_acct(acct)?;
    Some(format!("https://{host}/@{user}"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotMessage {
    /// `@user@host`, or empty when the slot has no configured account.
    pub source_label: String,
    /// Post title, for feeds that have them (blogs, news); Mastodon posts don't.
    pub title: Option<String>,
    /// Web page for the post; the title links to it.
    pub link: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct CommitEntry {
    pub repo: String,
    pub hash: String,
    pub subject: String,
    pub when: DateTime<Local>,
}

#[derive(Debug, Clone, Default)]
pub struct CommitStats {
    pub today: Vec<CommitEntry>,
    pub week: Vec<CommitEntry>,
}

#[derive(Debug, Clone)]
pub struct WeatherSnapshot {
    pub place: String,
    pub summary: String,
    pub temperature: String,
    pub high_low: String,
    pub wind: String,
}

impl CommitStats {
    pub fn today_count(&self) -> u64 {
        self.today.len() as u64
    }

    pub fn week_count(&self) -> u64 {
        self.week.len() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_link_rejects_escapes_and_non_web_urls() {
        assert_eq!(
            safe_link(" https://example.com/post "),
            Some("https://example.com/post".into())
        );
        assert_eq!(safe_link("https://example.com/\u{1b}]8;;x"), None);
        assert_eq!(safe_link("https://example.com/a\u{7}b"), None);
        assert_eq!(safe_link("https://example.com/a b"), None);
        assert_eq!(safe_link("javascript:alert(1)"), None);
        assert_eq!(safe_link("/relative/post"), None);
    }

    #[test]
    fn animals_cycle_through_all_and_wrap() {
        let mut a = Animal::Parrot;
        let mut seen = vec![a];
        for _ in 0..3 {
            a = a.next();
            seen.push(a);
        }
        assert_eq!(seen, Animal::ALL);
        assert_eq!(a.next(), Animal::Parrot);
        assert_eq!(Animal::parse(" Llama "), Some(Animal::Llama));
        assert_eq!(Animal::parse("/tmp/x.png"), None);
    }

    #[test]
    fn acct_resolves_to_mastodon_rss() {
        let src = BotSource::parse("@selfcare@hachyderm.io");
        assert_eq!(
            src.rss_url().as_deref(),
            Some("https://hachyderm.io/@selfcare.rss")
        );
        assert_eq!(
            profile_url_from_acct("@selfcare@hachyderm.io").as_deref(),
            Some("https://hachyderm.io/@selfcare")
        );
    }

    #[test]
    fn rejects_accts_that_could_inject() {
        for bad in [
            "tinycarebot",
            "user@host/evil",
            "user@host?x=1",
            "us er@host",
            "user@\u{1b}]8;;x",
            "@host",
        ] {
            assert_eq!(BotSource::parse(bad).rss_url(), None, "{bad}");
            assert_eq!(profile_url_from_acct(bad), None, "{bad}");
        }
    }

    #[test]
    fn clean_text_strips_escapes_and_collapses_space() {
        assert_eq!(
            clean_text("hi\u{1b}[2J  there\n\tfriend\u{7}"),
            "hi [2J there friend"
        );
    }

    #[test]
    fn feed_url_passthrough() {
        let src = BotSource::parse("https://example.com/feed.rss");
        assert_eq!(
            src.rss_url().as_deref(),
            Some("https://example.com/feed.rss")
        );
    }
}
