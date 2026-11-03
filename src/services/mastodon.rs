use std::thread;

use feed_rs::parser;

use crate::config::Config;
use crate::error::{CareError, Result};
use crate::model::{CARE_SLOTS, CareMessage, CareSource, clean_text, split_acct};
use crate::services::cache::Cache;
use crate::services::http::{self, HttpError, encode_query};
use crate::services::mock;

const DEFAULT_BACKOFF_SECS: u64 = 300;

/// Latest care tip per slot via Mastodon RSS (public REST fallback, disk cache,
/// rate-limit backoff). Slots are fetched in parallel. Returns `(messages, warnings)`.
pub fn fetch_care_messages(cfg: &Config) -> (Vec<CareMessage>, Vec<String>) {
    if cfg.mock {
        return (mock::mock_care_messages(cfg), Vec::new());
    }

    let cache = Cache::default_location();
    let backing_off = cache.as_ref().is_some_and(Cache::backoff_active);
    let sources = cfg.care_sources();

    let results: Vec<(CareMessage, Option<String>)> = thread::scope(|s| {
        let handles: Vec<_> = (0..CARE_SLOTS)
            .map(|i| {
                let cache = cache.as_ref();
                s.spawn(move || care_for_slot(cfg, i, sources.get(i), cache, backing_off))
            })
            .collect();
        handles
            .into_iter()
            .enumerate()
            .map(|(i, h)| {
                h.join().unwrap_or_else(|_| {
                    (
                        mock::fallback_message(cfg, i),
                        Some("care feed worker crashed".into()),
                    )
                })
            })
            .collect()
    });

    let mut messages = Vec::with_capacity(CARE_SLOTS);
    let mut warnings = Vec::new();
    for (msg, warning) in results {
        messages.push(msg);
        warnings.extend(warning);
    }
    (messages, warnings)
}

fn care_for_slot(
    cfg: &Config,
    index: usize,
    source: Option<&CareSource>,
    cache: Option<&Cache>,
    backing_off: bool,
) -> (CareMessage, Option<String>) {
    let Some(source) = source else {
        return (mock::fallback_message(cfg, index), None);
    };
    let label = source.label();
    let cached_or_fallback = || {
        cache
            .and_then(|c| c.load_care(&label))
            .unwrap_or_else(|| mock::fallback_message(cfg, index))
    };

    if backing_off {
        return (cached_or_fallback(), None);
    }

    match fetch_one(source) {
        Ok(msg) => {
            if let Some(cache) = cache {
                cache.save_care(&label, &msg);
            }
            (msg, None)
        }
        Err(e) => {
            let warning = if let CareError::Http(HttpError::RateLimited { retry_after_secs }) = &e {
                let secs = retry_after_secs
                    .unwrap_or(DEFAULT_BACKOFF_SECS)
                    .clamp(60, 3600);
                if let Some(cache) = cache {
                    cache.set_backoff_secs(secs);
                }
                format!("care feed {label}: rate limited — pausing {}m", secs / 60)
            } else {
                format!("care feed {label}: {e}")
            };
            (cached_or_fallback(), Some(warning))
        }
    }
}

fn fetch_one(source: &CareSource) -> Result<CareMessage> {
    let url = source.rss_url().ok_or_else(|| {
        CareError::Feed(format!(
            "{} is not a valid @user@host account",
            source.label()
        ))
    })?;

    let text = match http::get_bytes(&url)
        .map_err(CareError::from)
        .and_then(|bytes| latest_entry_text(&bytes))
    {
        Ok(text) => text,
        Err(rss_err) if rss_err.is_rate_limited() => return Err(rss_err),
        Err(rss_err) => match source {
            // Empty or broken RSS: try the public REST API for account sources.
            CareSource::Acct(acct) => fetch_rest(acct).map_err(|_| rss_err)?,
            CareSource::FeedUrl(_) => return Err(rss_err),
        },
    };
    Ok(CareMessage {
        source_label: source.label(),
        text,
    })
}

fn fetch_rest(acct: &str) -> Result<String> {
    let (user, host) = split_acct(acct)
        .ok_or_else(|| CareError::Feed(format!("invalid acct for REST fallback: {acct}")))?;
    let account = http::get_json(&format!(
        "https://{host}/api/v1/accounts/lookup?acct={}",
        encode_query(user)
    ))?;
    let id = account["id"]
        .as_str()
        .filter(|id| id.chars().all(|c| c.is_ascii_alphanumeric()))
        .ok_or_else(|| CareError::Feed(format!("lookup {acct}: missing id")))?;
    let statuses = http::get_json(&format!(
        "https://{host}/api/v1/accounts/{id}/statuses?limit=1&exclude_replies=true&exclude_reblogs=true"
    ))?;
    let content = statuses
        .as_array()
        .and_then(|a| a.first())
        .and_then(|s| s["content"].as_str())
        .ok_or_else(|| CareError::Feed(format!("statuses {acct}: empty")))?;
    non_empty(strip_html(content))
}

pub fn latest_entry_text(feed_bytes: &[u8]) -> Result<String> {
    let feed =
        parser::parse(feed_bytes).map_err(|e| CareError::Feed(format!("RSS parse error: {e}")))?;

    let entry = feed
        .entries
        .first()
        .ok_or_else(|| CareError::Feed("RSS feed has no entries".into()))?;

    let raw = entry
        .summary
        .as_ref()
        .map(|s| s.content.as_str())
        .or_else(|| entry.content.as_ref().and_then(|c| c.body.as_deref()))
        .or_else(|| entry.title.as_ref().map(|t| t.content.as_str()))
        .unwrap_or("");
    non_empty(strip_html(raw))
}

fn non_empty(text: String) -> Result<String> {
    if text.is_empty() {
        Err(CareError::Feed("latest post had no text".into()))
    } else {
        Ok(text)
    }
}

fn strip_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    clean_text(&decode_entities(&out))
}

fn decode_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let Some(end) = rest.find(';').filter(|&end| end <= 10) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[..=end];
        let decoded = match entity {
            "&amp;" => Some('&'),
            "&lt;" => Some('<'),
            "&gt;" => Some('>'),
            "&quot;" => Some('"'),
            "&apos;" => Some('\''),
            "&nbsp;" => Some(' '),
            _ => entity
                .strip_prefix("&#x")
                .or_else(|| entity.strip_prefix("&#X"))
                .and_then(|hex| u32::from_str_radix(&hex[..hex.len() - 1], 16).ok())
                .or_else(|| {
                    entity
                        .strip_prefix("&#")
                        .and_then(|dec| dec[..dec.len() - 1].parse().ok())
                })
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => out.push(c),
            None => out.push_str(entity),
        }
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fixture_rss() {
        let bytes = include_bytes!("../../fixtures/care/selfcare.rss");
        let text = latest_entry_text(bytes).unwrap();
        assert!(text.contains("timer"));
        assert!(text.contains("breaks"));
        assert!(!text.contains('<'), "expected tags stripped: {text}");
        assert!(!text.contains("&"), "expected entities decoded: {text}");
    }

    #[test]
    fn strip_html_decodes_entities() {
        assert_eq!(
            strip_html("<p>It&#39;s okay &amp; fine &#x1F49A;</p>"),
            "It's okay & fine 💚"
        );
        assert_eq!(strip_html("Tom & Jerry; friends"), "Tom & Jerry; friends");
    }

    #[test]
    fn strip_html_drops_encoded_escape_sequences() {
        let text = strip_html("hi &#27;]0;pwned&#7; &#x1b;[2J there");
        assert!(!text.chars().any(char::is_control), "{text:?}");
    }

    #[test]
    fn missing_slots_fall_back_quietly() {
        let mut cfg = Config::default();
        cfg.bots.truncate(1);
        let (msg, warning) = care_for_slot(&cfg, 2, None, None, false);
        assert!(warning.is_none());
        assert!(msg.source_label.is_empty());
        assert!(!msg.text.is_empty());
    }

    #[test]
    fn invalid_accounts_warn_without_network() {
        let cfg = Config::default();
        let source = CareSource::parse("tinycarebot");
        let (_, warning) = care_for_slot(&cfg, 0, Some(&source), None, false);
        assert!(warning.unwrap().contains("not a valid"));
    }
}

#[cfg(test)]
mod live {
    use super::*;

    #[test]
    #[ignore = "network"]
    fn fetches_selfcare_rss() {
        let msg = fetch_one(&CareSource::parse("selfcare@hachyderm.io")).expect("fetch");
        assert!(!msg.text.is_empty());
        eprintln!("live: {}", msg.text);
    }
}
