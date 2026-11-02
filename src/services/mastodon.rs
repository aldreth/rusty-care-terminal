use std::time::Duration;

use chrono::Local;
use feed_rs::parser;

use crate::config::Config;
use crate::error::{CareError, Result};
use crate::model::{CareMessage, CareSource};
use crate::services::mock;

const USER_AGENT: &str = concat!("rusty-care-terminal/", env!("CARGO_PKG_VERSION"));

/// Fetch latest care messages via Mastodon RSS (or mock / static fallback).
pub fn fetch_care_messages(cfg: &Config) -> Vec<CareMessage> {
    if cfg.mock {
        return mock::mock_care_messages(cfg);
    }

    let sources = cfg.care_sources();
    let mut out = Vec::with_capacity(3);

    for i in 0..3 {
        let source = sources.get(i).cloned().unwrap_or_else(|| {
            CareSource::Acct(format!("care-{}", i + 1))
        });
        match fetch_one(&source) {
            Ok(msg) => out.push(msg),
            Err(e) => {
                eprintln!("warning: care feed {}: {e}", source.label());
                // Keep a soft fallback so panels never go blank.
                let fallback = mock::mock_care_messages(cfg);
                let idx = i.min(fallback.len().saturating_sub(1));
                if let Some(msg) = fallback.get(idx) {
                    out.push(CareMessage {
                        source_label: format!("{} · offline", source.label()),
                        text: msg.text.clone(),
                        fetched_at: Local::now(),
                        from_mock: true,
                    });
                } else {
                    out.push(CareMessage {
                        source_label: format!("{} · offline", source.label()),
                        text: "Hi! You're doing great!!!".into(),
                        fetched_at: Local::now(),
                        from_mock: true,
                    });
                }
            }
        }
    }

    out
}

fn fetch_one(source: &CareSource) -> Result<CareMessage> {
    let url = source
        .rss_url()
        .ok_or_else(|| CareError::Feed(format!("cannot resolve RSS for {}", source.label())))?;

    let client = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| CareError::Feed(e.to_string()))?;

    let response = client
        .get(&url)
        .send()
        .map_err(|e| CareError::Feed(format!("{url}: {e}")))?;

    if !response.status().is_success() {
        return Err(CareError::Feed(format!(
            "{url}: HTTP {}",
            response.status()
        )));
    }

    let bytes = response
        .bytes()
        .map_err(|e| CareError::Feed(format!("{url}: {e}")))?;

    let text = latest_entry_text(&bytes)?;
    Ok(CareMessage {
        source_label: format!("{} → {url}", source.label()),
        text,
        fetched_at: Local::now(),
        from_mock: false,
    })
}

pub fn latest_entry_text(feed_bytes: &[u8]) -> Result<String> {
    let feed = parser::parse(feed_bytes)
        .map_err(|e| CareError::Feed(format!("RSS parse error: {e}")))?;

    let entry = feed
        .entries
        .first()
        .ok_or_else(|| CareError::Feed("RSS feed has no entries".into()))?;

    let raw = entry
        .summary
        .as_ref()
        .map(|s| s.content.as_str())
        .or_else(|| {
            entry
                .content
                .as_ref()
                .and_then(|c| c.body.as_deref())
        })
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
                    if let Ok(code) = u32::from_str_radix(&other[3..other.len() - 1], 16) {
                        if let Some(c) = char::from_u32(code) {
                            out.push(c);
                            rest = &rest[end + 1..];
                            continue;
                        }
                    }
                    other
                }
                other if other.starts_with("&#") => {
                    if let Ok(code) = other[2..other.len() - 1].parse::<u32>() {
                        if let Some(c) = char::from_u32(code) {
                            out.push(c);
                            rest = &rest[end + 1..];
                            continue;
                        }
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
