use chrono::{DateTime, Local};

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

#[derive(Debug, Clone)]
pub struct CareMessage {
    pub source_label: String,
    pub text: String,
    #[allow(dead_code)]
    pub fetched_at: DateTime<Local>,
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
    fn acct_resolves_to_mastodon_rss() {
        let src = CareSource::parse("@selfcare@hachyderm.io");
        assert_eq!(
            src.rss_url().as_deref(),
            Some("https://hachyderm.io/@selfcare.rss")
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
