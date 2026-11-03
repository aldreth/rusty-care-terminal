use std::io::Read;
use std::sync::OnceLock;
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::blocking::Client;
use reqwest::header::RETRY_AFTER;
use serde_json::Value;
use thiserror::Error;

const USER_AGENT: &str = concat!(
    "rusty-care-terminal/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/aldreth/rusty-care-terminal)"
);

/// Feeds and API replies are a few KB; anything this big is not what we asked for.
const MAX_BODY_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("rate limited (HTTP 429)")]
    RateLimited { retry_after_secs: Option<u64> },

    #[error("HTTP {0}")]
    Status(u16),

    #[error("{0}")]
    Transport(String),

    #[error("response larger than {MAX_BODY_BYTES} bytes")]
    TooLarge,

    #[error("invalid JSON: {0}")]
    Json(String),
}

fn client() -> Result<&'static Client, HttpError> {
    static CLIENT: OnceLock<Result<Client, String>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            Client::builder()
                .user_agent(USER_AGENT)
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(10))
                .build()
                .map_err(|e| e.to_string())
        })
        .as_ref()
        .map_err(|e| HttpError::Transport(e.clone()))
}

pub fn get_bytes(url: &str) -> Result<Vec<u8>, HttpError> {
    let response = client()?
        .get(url)
        .send()
        .map_err(|e| HttpError::Transport(e.without_url().to_string()))?;

    let status = response.status();
    if status == StatusCode::TOO_MANY_REQUESTS {
        let retry_after_secs = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.trim().parse().ok());
        return Err(HttpError::RateLimited { retry_after_secs });
    }
    if !status.is_success() {
        return Err(HttpError::Status(status.as_u16()));
    }

    let mut body = Vec::new();
    response
        .take(MAX_BODY_BYTES + 1)
        .read_to_end(&mut body)
        .map_err(|e| HttpError::Transport(e.to_string()))?;
    if body.len() as u64 > MAX_BODY_BYTES {
        return Err(HttpError::TooLarge);
    }
    Ok(body)
}

pub fn get_json(url: &str) -> Result<Value, HttpError> {
    let body = get_bytes(url)?;
    serde_json::from_slice(&body).map_err(|e| HttpError::Json(e.to_string()))
}

/// Minimal `application/x-www-form-urlencoded` encoding for query values.
pub fn encode_query(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_query_values() {
        assert_eq!(encode_query("São Paulo&x=1"), "S%C3%A3o+Paulo%26x%3D1");
    }
}
