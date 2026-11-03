use chrono::{DateTime, Local};

/// Say-box creature.
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

/// How a care source is addressed (`@user@instance` or an explicit feed URL).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CareSource {
    /// `@user@instance` or `user@instance` → resolve to `https://{host}/@{user}.rss`
    Acct(String),
    /// Explicit RSS/Atom URL
    FeedUrl(String),
}

impl CareSource {
    pub fn parse(raw: &str) -> Self {
        let trimmed = raw.trim();
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            CareSource::FeedUrl(trimmed.to_string())
        } else {
            let acct = trimmed.trim_start_matches('@').to_string();
            CareSource::Acct(acct)
        }
    }

    /// Intended RSS endpoint for this source (Milestone 2 will fetch it).
    pub fn rss_url(&self) -> Option<String> {
        match self {
            CareSource::FeedUrl(url) => Some(url.clone()),
            CareSource::Acct(acct) => {
                let (user, host) = split_acct(acct)?;
                Some(format!("https://{host}/@{user}.rss"))
            }
        }
    }

    pub fn label(&self) -> String {
        match self {
            CareSource::FeedUrl(url) => url.clone(),
            CareSource::Acct(acct) => format!("@{acct}"),
        }
    }
}

fn split_acct(acct: &str) -> Option<(&str, &str)> {
    let (user, host) = acct.split_once('@')?;
    if user.is_empty() || host.is_empty() {
        None
    } else {
        Some((user, host))
    }
}

/// `@user@host` / `user@host` → `https://{host}/@{user}`.
pub fn profile_url_from_acct(acct_or_label: &str) -> Option<String> {
    let acct = acct_or_label.trim().trim_start_matches('@');
    let (user, host) = split_acct(acct)?;
    Some(format!("https://{host}/@{user}"))
}

#[derive(Debug, Clone)]
pub struct CareMessage {
    pub source_label: String,
    pub text: String,
    #[allow(dead_code)]
    pub fetched_at: DateTime<Local>,
    #[allow(dead_code)]
    pub from_mock: bool,
}

#[derive(Debug, Clone)]
pub struct CommitEntry {
    pub repo: String,
    pub hash: String,
    #[allow(dead_code)]
    pub author: String,
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
    #[allow(dead_code)]
    pub fetched_at: DateTime<Local>,
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
        let src = CareSource::parse("@selfcare@hachyderm.io");
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
    fn feed_url_passthrough() {
        let src = CareSource::parse("https://example.com/feed.rss");
        assert_eq!(
            src.rss_url().as_deref(),
            Some("https://example.com/feed.rss")
        );
    }
}
