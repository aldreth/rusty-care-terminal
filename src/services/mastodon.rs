use std::thread;

use feed_rs::parser;

use crate::config::Config;
use crate::error::{AppError, Result};
use crate::model::{BOT_SLOTS, BotMessage, BotSource, clean_text, safe_link, split_acct};
use crate::services::cache::Cache;
use crate::services::http::{self, HttpError, encode_query};
use crate::services::mock;

const DEFAULT_BACKOFF_SECS: u64 = 300;

/// Latest message per bot slot via Mastodon RSS (public REST fallback, disk cache,
/// rate-limit backoff). Slots are fetched in parallel. Returns `(messages, warnings)`.
pub fn fetch_messages(cfg: &Config) -> (Vec<BotMessage>, Vec<String>) {
    if cfg.mock {
        return (mock::mock_messages(cfg), Vec::new());
    }

    let cache = Cache::default_location();
    let backing_off = cache.as_ref().is_some_and(Cache::backoff_active);
    let sources = &cfg.bots;

    let results: Vec<(BotMessage, Option<String>)> = thread::scope(|s| {
        let handles: Vec<_> = (0..BOT_SLOTS)
            .map(|i| {
                let cache = cache.as_ref();
                s.spawn(move || message_for_slot(cfg, i, sources.get(i), cache, backing_off))
            })
            .collect();
        handles
            .into_iter()
            .enumerate()
            .map(|(i, h)| {
                h.join().unwrap_or_else(|_| {
                    (
                        mock::fallback_message(cfg, i),
                        Some("bot feed worker crashed".into()),
                    )
                })
            })
            .collect()
    });

    let mut messages = Vec::with_capacity(BOT_SLOTS);
    let mut warnings = Vec::new();
    for (msg, warning) in results {
        messages.push(msg);
        warnings.extend(warning);
    }
    (messages, warnings)
}

fn message_for_slot(
    cfg: &Config,
    index: usize,
    source: Option<&BotSource>,
    cache: Option<&Cache>,
    backing_off: bool,
) -> (BotMessage, Option<String>) {
    let Some(source) = source else {
        return (mock::fallback_message(cfg, index), None);
    };
    let label = source.label();
    let cached_or_fallback = || {
        cache
            .and_then(|c| c.load_message(&label))
            .unwrap_or_else(|| mock::fallback_message(cfg, index))
    };

    if backing_off {
        return (cached_or_fallback(), None);
    }

    match fetch_one(source) {
        Ok(msg) => {
            if let Some(cache) = cache {
                cache.save_message(&label, &msg);
            }
            (msg, None)
        }
        Err(e) => {
            let warning = if let AppError::Http(HttpError::RateLimited { retry_after_secs }) = &e {
                let secs = retry_after_secs
                    .unwrap_or(DEFAULT_BACKOFF_SECS)
                    .clamp(60, 3600);
                if let Some(cache) = cache {
                    cache.set_backoff_secs(secs);
                }
                format!("bot {label}: rate limited — pausing {}m", secs / 60)
            } else {
                format!("bot {label}: {e}")
            };
            (cached_or_fallback(), Some(warning))
        }
    }
}

fn fetch_one(source: &BotSource) -> Result<BotMessage> {
    let url = source.rss_url().ok_or_else(|| {
        AppError::Feed(format!(
            "{} is not a valid @user@host account",
            source.label()
        ))
    })?;

    let post = match http::get_bytes(&url)
        .map_err(AppError::from)
        .and_then(|bytes| latest_post(&bytes))
    {
        Ok(post) => post,
        Err(rss_err) if rss_err.is_rate_limited() => return Err(rss_err),
        Err(rss_err) => match source {
            // Empty or broken RSS: try the public REST API for account sources.
            BotSource::Acct(acct) => Post {
                text: fetch_rest(acct).map_err(|_| rss_err)?,
                ..Post::default()
            },
            BotSource::FeedUrl(_) => return Err(rss_err),
        },
    };
    Ok(BotMessage {
        source_label: source.label(),
        title: post.title,
        link: post.link,
        text: post.text,
    })
}

fn fetch_rest(acct: &str) -> Result<String> {
    let (user, host) = split_acct(acct)
        .ok_or_else(|| AppError::Feed(format!("invalid acct for REST fallback: {acct}")))?;
    let account = http::get_json(&format!(
        "https://{host}/api/v1/accounts/lookup?acct={}",
        encode_query(user)
    ))?;
    let id = account["id"]
        .as_str()
        .filter(|id| id.chars().all(|c| c.is_ascii_alphanumeric()))
        .ok_or_else(|| AppError::Feed(format!("lookup {acct}: missing id")))?;
    let statuses = http::get_json(&format!(
        "https://{host}/api/v1/accounts/{id}/statuses?limit=1&exclude_replies=true&exclude_reblogs=true"
    ))?;
    let content = statuses
        .as_array()
        .and_then(|a| a.first())
        .and_then(|s| s["content"].as_str())
        .ok_or_else(|| AppError::Feed(format!("statuses {acct}: empty")))?;
    non_empty(strip_html(content))
}

/// Blog posts can run to thousands of words; a panel shows a few lines.
const MAX_TEXT_CHARS: usize = 600;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Post {
    /// Only when the feed has real titles (blogs, news); Mastodon's don't.
    pub title: Option<String>,
    /// Only alongside a title, which links to it.
    pub link: Option<String>,
    pub text: String,
}

/// The newest post in an RSS or Atom feed.
pub fn latest_post(feed_bytes: &[u8]) -> Result<Post> {
    let feed =
        parser::parse(feed_bytes).map_err(|e| AppError::Feed(format!("RSS parse error: {e}")))?;

    let entry = feed
        .entries
        .first()
        .ok_or_else(|| AppError::Feed("RSS feed has no entries".into()))?;

    let title = entry
        .title
        .as_ref()
        .map(|t| strip_html(&t.content))
        .filter(|t| !t.is_empty());
    let body = entry
        .summary
        .as_ref()
        .map(|s| s.content.as_str())
        .or_else(|| entry.content.as_ref().and_then(|c| c.body.as_deref()))
        .map(strip_html)
        .unwrap_or_default();
    let body = truncate_chars(&body, MAX_TEXT_CHARS);

    let text_only = |text| Post {
        text,
        ..Post::default()
    };
    match title {
        Some(title) if body.is_empty() => Ok(text_only(title)),
        // Feeds without real titles often repeat the start of the post instead.
        Some(title) if body.starts_with(title.trim_end_matches(['…', '.'])) => {
            Ok(text_only(body))
        }
        None => Ok(text_only(non_empty(body)?)),
        Some(title) => Ok(Post {
            title: Some(title),
            link: entry_link(entry),
            text: body,
        }),
    }
}

/// The post's own page: its `alternate` link, else the first web link.
fn entry_link(entry: &feed_rs::model::Entry) -> Option<String> {
    let alternate = entry
        .links
        .iter()
        .find(|l| l.rel.as_deref().is_none_or(|rel| rel == "alternate"));
    alternate
        .into_iter()
        .chain(&entry.links)
        .find_map(|l| safe_link(&l.href))
}

fn truncate_chars(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((cut, _)) => format!("{}…", text[..cut].trim_end()),
        None => text.to_string(),
    }
}

fn non_empty(text: String) -> Result<String> {
    if text.is_empty() {
        Err(AppError::Feed("latest post had no text".into()))
    } else {
        Ok(text)
    }
}

/// Tags that separate blocks of text, so `<p>One</p><p>Two</p>` reads "One Two".
const BLOCK_TAGS: &[&str] = &[
    "p",
    "br",
    "div",
    "li",
    "ul",
    "ol",
    "blockquote",
    "pre",
    "hr",
    "tr",
    "td",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "figure",
    "figcaption",
];

fn strip_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut tag: Option<String> = None;
    for ch in input.chars() {
        match (&mut tag, ch) {
            (None, '<') => tag = Some(String::new()),
            (None, ch) => out.push(ch),
            (Some(name), '>') => {
                if is_block_tag(name) {
                    out.push(' ');
                }
                tag = None;
            }
            (Some(name), ch) if name.len() < 16 => name.push(ch),
            (Some(_), _) => {}
        }
    }
    clean_text(&decode_entities(&out))
}

fn is_block_tag(tag: &str) -> bool {
    let name: String = tag
        .trim_start_matches('/')
        .chars()
        .take_while(char::is_ascii_alphanumeric)
        .collect();
    BLOCK_TAGS
        .iter()
        .any(|block| name.eq_ignore_ascii_case(block))
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
        let bytes = include_bytes!("../../fixtures/bots/selfcare.rss");
        let Post { title, link, text } = latest_post(bytes).unwrap();
        assert_eq!(title, None, "Mastodon posts have no titles");
        assert_eq!(link, None);
        assert!(text.contains("timer"));
        assert!(text.contains("breaks"));
        assert!(!text.contains('<'), "expected tags stripped: {text}");
        assert!(!text.contains("&"), "expected entities decoded: {text}");
    }

    const BLOG: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Family Diary</title>
  <id>https://example.com/</id>
  <updated>2026-10-31T16:03:03Z</updated>
  <entry>
    <title>Toby&#39;s A levels</title>
    <id>https://example.com/a-levels</id>
    <link rel="alternate" href="https://example.com/2026/10/a-levels"/>
    <updated>2026-10-31T16:03:03Z</updated>
    <content type="html">&lt;p&gt;Hooray!!&lt;/p&gt;&lt;p&gt;Toby got his &lt;em&gt;A level&lt;/em&gt; results.&lt;/p&gt;</content>
  </entry>
</feed>"#;

    #[test]
    fn blog_posts_keep_their_title_and_paragraph_breaks() {
        let post = latest_post(BLOG.as_bytes()).unwrap();
        assert_eq!(post.title.as_deref(), Some("Toby's A levels"));
        assert_eq!(
            post.link.as_deref(),
            Some("https://example.com/2026/10/a-levels")
        );
        assert_eq!(post.text, "Hooray!! Toby got his A level results.");
    }

    #[test]
    fn non_web_post_links_are_dropped() {
        let hostile = BLOG.replace(
            "https://example.com/2026/10/a-levels",
            "javascript:alert(1)",
        );
        let post = latest_post(hostile.as_bytes()).unwrap();
        assert!(post.title.is_some());
        assert_eq!(post.link, None);
    }

    #[test]
    fn long_posts_are_trimmed() {
        let long = BLOG.replace("Hooray!!", &"word ".repeat(400));
        let text = latest_post(long.as_bytes()).unwrap().text;
        assert!(text.chars().count() <= MAX_TEXT_CHARS + 1);
        assert!(text.ends_with('…'));
    }

    #[test]
    fn strip_html_separates_blocks_but_not_inline_tags() {
        assert_eq!(
            strip_html("<p>Rest.</p><p><a href=\"/tags/pets\">#<span>pets</span></a></p>"),
            "Rest. #pets"
        );
        assert_eq!(strip_html("one<br/>two<BR>three"), "one two three");
        assert_eq!(
            strip_html("<span class=\"h-card\">@<span>bob</span></span>"),
            "@bob"
        );
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
        let (msg, warning) = message_for_slot(&cfg, 2, None, None, false);
        assert!(warning.is_none());
        assert!(msg.source_label.is_empty());
        assert!(!msg.text.is_empty());
    }

    #[test]
    fn invalid_accounts_warn_without_network() {
        let cfg = Config::default();
        let source = BotSource::parse("tinycarebot");
        let (_, warning) = message_for_slot(&cfg, 0, Some(&source), None, false);
        assert!(warning.unwrap().contains("not a valid"));
    }
}

#[cfg(test)]
mod live {
    use super::*;

    #[test]
    #[ignore = "network"]
    fn fetches_selfcare_rss() {
        let msg = fetch_one(&BotSource::parse("selfcare@hachyderm.io")).expect("fetch");
        assert!(!msg.text.is_empty());
        eprintln!("live: {}", msg.text);
    }
}
