use std::time::Duration;

use chrono::Local;
use feed_rs::parser;
use serde_json::Value;

use crate::config::Config;
use crate::error::{CareError, Result};
use crate::model::{CareMessage, CareSource};
use crate::services::{cache, mock};

const USER_AGENT: &str = concat!("rusty-care-terminal/", env!("CARGO_PKG_VERSION"));

/// Fetch latest care messages via Mastodon RSS (REST fallback, disk cache, backoff).
/// Returns `(messages, warnings)` — never prints to stderr (keeps the TUI clean).
pub fn fetch_care_messages(cfg: &Config) -> (Vec<CareMessage>, Vec<String>) {
    if cfg.mock {
        return (mock::mock_care_messages(cfg), Vec::new());
    }

    cache::clear_backoff_if_expired();
    let backing_off = cache::backoff_active();

    let sources = cfg.care_sources();
    let mut out = Vec::with_capacity(3);
    let mut warnings = Vec::new();

    for i in 0..3 {
        let source = sources
            .get(i)
            .cloned()
            .unwrap_or_else(|| CareSource::Acct(format!("care-{}", i + 1)));
        let cache_key = source.label();

        if backing_off {
            if let Some(msg) = cache::load_care(&cache_key) {
                out.push(msg);
                continue;
            }
            warnings.push(format!(
                "care feed {}: rate-limited; using offline fallback",
                source.label()
            ));
            out.push(offline_fallback(cfg, i, &source));
            continue;
        }

        match fetch_one(&source) {
            Ok(msg) => {
                cache::save_care(&cache_key, &msg);
                out.push(msg);
            }
            Err(e) => {
                let err = e.to_string();
                if err.contains("429") {
                    cache::set_backoff_secs(300);
                    warnings.push(format!(
                        "care feed {}: rate limited — backing off 5m",
                        source.label()
                    ));
                } else {
                    warnings.push(format!("care feed {}: {e}", source.label()));
                }
                if let Some(msg) = cache::load_care(&cache_key) {
                    out.push(msg);
                } else {
                    out.push(offline_fallback(cfg, i, &source));
                }
            }
        }
    }

    (out, warnings)
}

fn offline_fallback(cfg: &Config, index: usize, source: &CareSource) -> CareMessage {
    let fallback = mock::mock_care_messages(cfg);
    let idx = index.min(fallback.len().saturating_sub(1));
    if let Some(msg) = fallback.get(idx) {
        CareMessage {
            source_label: format!("{} · offline", source.label()),
            text: msg.text.clone(),
            fetched_at: Local::now(),
            from_mock: true,
        }
    } else {
        CareMessage {
            source_label: format!("{} · offline", source.label()),
            text: "Hi! You're doing great!!!".into(),
            fetched_at: Local::now(),
            from_mock: true,
        }
    }
}

fn fetch_one(source: &CareSource) -> Result<CareMessage> {
    let url = source
        .rss_url()
        .ok_or_else(|| CareError::Feed(format!("cannot resolve RSS for {}", source.label())))?;

    match fetch_rss(&url) {
        Ok(text) => Ok(CareMessage {
            source_label: format!("{} → {url}", source.label()),
            text,
            fetched_at: Local::now(),
            from_mock: false,
        }),
        Err(rss_err) => {
            // Empty/broken RSS → optional public REST fallback for acct sources.
            if let CareSource::Acct(acct) = source {
                match fetch_rest(acct) {
                    Ok(text) => Ok(CareMessage {
                        source_label: format!("{} → rest", source.label()),
                        text,
                        fetched_at: Local::now(),
                        from_mock: false,
                    }),
                    Err(_) => Err(rss_err),
                }
            } else {
                Err(rss_err)
            }
        }
    }
}

fn http_client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| CareError::Feed(e.to_string()))
}

fn fetch_rss(url: &str) -> Result<String> {
    let client = http_client()?;
    let response = client
        .get(url)
        .send()
        .map_err(|e| CareError::Feed(format!("{url}: {e}")))?;

    let status = response.status();
    if status.as_u16() == 429 {
        let retry = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(300);
        cache::set_backoff_secs(retry);
        return Err(CareError::Feed(format!("{url}: HTTP 429")));
    }
    if !status.is_success() {
        return Err(CareError::Feed(format!("{url}: HTTP {status}")));
    }

    let bytes = response
        .bytes()
        .map_err(|e| CareError::Feed(format!("{url}: {e}")))?;
    latest_entry_text(&bytes)
}

fn fetch_rest(acct: &str) -> Result<String> {
    let (user, host) = acct
        .split_once('@')
        .ok_or_else(|| CareError::Feed(format!("invalid acct for REST fallback: {acct}")))?;
    let client = http_client()?;
    let lookup = format!("https://{host}/api/v1/accounts/lookup?acct={user}");
    let account: Value = client
        .get(&lookup)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())
        .map_err(|e| CareError::Feed(format!("lookup {acct}: {e}")))?;
    let id = account["id"]
        .as_str()
        .ok_or_else(|| CareError::Feed(format!("lookup {acct}: missing id")))?;
    let statuses_url = format!(
        "https://{host}/api/v1/accounts/{id}/statuses?limit=1&exclude_replies=true&exclude_reblogs=true"
    );
    let statuses: Value = client
        .get(&statuses_url)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())
        .map_err(|e| CareError::Feed(format!("statuses {acct}: {e}")))?;
    let content = statuses
        .as_array()
        .and_then(|a| a.first())
        .and_then(|s| s["content"].as_str())
        .ok_or_else(|| CareError::Feed(format!("statuses {acct}: empty")))?;
    let plain = strip_html(content);
    if plain.is_empty() {
        Err(CareError::Feed(format!("statuses {acct}: empty text")))
    } else {
        Ok(plain)
    }
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

    let plain = strip_html(raw);
    if plain.is_empty() {
        Err(CareError::Feed("latest entry had empty text".into()))
    } else {
        Ok(plain)
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
    decode_entities(&out)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn decode_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        if let Some(end) = rest.find(';') {
            let entity = &rest[..=end];
            out.push_str(match entity {
                "&amp;" => "&",
                "&lt;" => "<",
                "&gt;" => ">",
                "&quot;" => "\"",
                "&apos;" | "&#39;" | "&#x27;" => "'",
                "&nbsp;" => " ",
                other if other.starts_with("&#x") || other.starts_with("&#X") => {
                    if let Ok(code) = u32::from_str_radix(&other[3..other.len() - 1], 16)
                        && let Some(c) = char::from_u32(code)
                    {
                        out.push(c);
                        rest = &rest[end + 1..];
                        continue;
                    }
                    other
                }
                other if other.starts_with("&#") => {
                    if let Ok(code) = other[2..other.len() - 1].parse::<u32>()
                        && let Some(c) = char::from_u32(code)
                    {
                        out.push(c);
                        rest = &rest[end + 1..];
                        continue;
                    }
                    other
                }
                other => other,
            });
            rest = &rest[end + 1..];
        } else {
            out.push('&');
            rest = &rest[1..];
        }
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
            strip_html("<p>It&#39;s okay &amp; fine</p>"),
            "It's okay & fine"
        );
    }
}

#[cfg(test)]
mod live {
    use super::*;
    use crate::model::CareSource;

    #[test]
    #[ignore = "network"]
    fn fetches_selfcare_rss() {
        let msg = fetch_one(&CareSource::parse("selfcare@hachyderm.io")).expect("fetch");
        assert!(!msg.from_mock);
        assert!(!msg.text.is_empty());
        eprintln!("live: {}", msg.text);
    }
}
