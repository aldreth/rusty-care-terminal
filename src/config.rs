use std::env;
use std::path::PathBuf;

use crate::error::{CareError, Result};
use crate::model::CareSource;

const DEFAULT_BOTS: &[&str] = &[
    "selfcare@hachyderm.io",
    "selfcaregentle@hachyderm.io",
    // GoatsLive has a lively public RSS; 3goodthings group feed is often empty.
    "GoatsLive@mastodon.social",
];

#[derive(Debug, Clone)]
pub struct Config {
    pub bots: Vec<CareSource>,
    pub bot_feeds: Vec<String>,
    pub say_box: String,
    pub repos: Vec<PathBuf>,
    pub repos_depth: usize,
    /// Identity strings for sole/joint authorship filter (comma-separated).
    /// Default: each repo's `user.email` / `user.name`.
    pub git_author: Option<String>,
    pub weather: Option<String>,
    pub celsius: bool,
    pub update_interval_mins: u64,
    pub terminal_title: String,
    pub pomodoro: u64,
    pub break_minutes: u64,
    pub mock: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bots: DEFAULT_BOTS.iter().map(|b| CareSource::parse(b)).collect(),
            bot_feeds: Vec::new(),
            say_box: "parrot".into(),
            repos: Vec::new(),
            repos_depth: 1,
            git_author: None,
            weather: None,
            celsius: true,
            update_interval_mins: 20,
            terminal_title: "tiny care".into(),
            pomodoro: 25,
            break_minutes: 5,
            mock: false,
        }
    }
}

impl Config {
    /// Load from environment (`RCT_*`, with legacy `TTC_*` aliases) and CLI overrides.
    pub fn load(cli: CliOverrides) -> Result<Self> {
        let mut cfg = Config::default();
        merge_env(&mut cfg)?;

        if let Some(repos) = cli.repos {
            cfg.repos = expand_repos(&split_csv(&repos));
        }
        if let Some(depth) = cli.repos_depth {
            cfg.repos_depth = depth;
        }
        if let Some(bots) = cli.bots {
            cfg.bots = split_csv(&bots)
                .iter()
                .map(|b| CareSource::parse(b))
                .collect();
        }
        if let Some(feeds) = cli.bot_feeds {
            cfg.bot_feeds = split_csv(&feeds);
        }
        if let Some(say_box) = cli.say_box {
            cfg.say_box = say_box;
        }
        if let Some(author) = cli.git_author {
            cfg.git_author = Some(author);
        }
        if let Some(mins) = cli.update_interval {
            cfg.update_interval_mins = mins.max(1);
        }
        if let Some(weather) = cli.weather {
            cfg.weather = Some(weather);
        }
        if let Some(celsius) = cli.celsius {
            cfg.celsius = celsius;
        }
        if let Some(pomodoro) = cli.pomodoro {
            cfg.pomodoro = pomodoro.max(1);
        }
        if let Some(break_minutes) = cli.break_minutes {
            cfg.break_minutes = break_minutes.max(1);
        }
        if cli.mock {
            cfg.mock = true;
        }

        // Explicit feed URLs win / pad sources when provided.
        if !cfg.bot_feeds.is_empty() {
            let mut sources: Vec<CareSource> = cfg
                .bot_feeds
                .iter()
                .map(|u| CareSource::FeedUrl(u.clone()))
                .collect();
            if sources.len() < cfg.bots.len() {
                sources.extend(cfg.bots.iter().skip(sources.len()).cloned());
            }
            cfg.bots = sources;
        }

        Ok(cfg)
    }

    pub fn care_sources(&self) -> &[CareSource] {
        &self.bots
    }
}

/// Values from clap that override env / defaults.
#[derive(Debug, Default)]
pub struct CliOverrides {
    pub repos: Option<String>,
    pub repos_depth: Option<usize>,
    pub bots: Option<String>,
    pub bot_feeds: Option<String>,
    pub say_box: Option<String>,
    pub git_author: Option<String>,
    pub update_interval: Option<u64>,
    pub weather: Option<String>,
    pub celsius: Option<bool>,
    pub pomodoro: Option<u64>,
    pub break_minutes: Option<u64>,
    pub mock: bool,
}

fn merge_env(cfg: &mut Config) -> Result<()> {
    // Prefer RCT_*; accept legacy TTC_* as aliases.
    if let Some(v) = env_first(&["RCT_BOTS", "TTC_BOTS"]) {
        cfg.bots = split_csv(&v).iter().map(|b| CareSource::parse(b)).collect();
    }
    if let Some(v) = env_first(&["RCT_BOT_FEEDS"]) {
        cfg.bot_feeds = split_csv(&v);
    }
    if let Some(v) = env_first(&["RCT_SAY_BOX", "TTC_SAY_BOX"]) {
        cfg.say_box = v;
    }
    if let Some(v) = env_first(&["RCT_REPOS", "TTC_REPOS"]) {
        cfg.repos = expand_repos(&split_csv(&v));
    }
    if let Some(v) = env_first(&["RCT_REPOS_DEPTH", "TTC_REPOS_DEPTH"]) {
        cfg.repos_depth = v
            .parse()
            .map_err(|_| CareError::Config(format!("invalid repos_depth: {v}")))?;
    }
    if let Some(v) = env_first(&["RCT_GIT_AUTHOR", "TTC_GIT_AUTHOR"]) {
        cfg.git_author = Some(v);
    }
    if let Some(v) = env_first(&["RCT_WEATHER", "TTC_WEATHER"]) {
        cfg.weather = Some(v);
    }
    if let Some(v) = env_first(&["RCT_CELSIUS", "TTC_CELSIUS"]) {
        cfg.celsius = parse_bool(&v)?;
    }
    if let Some(v) = env_first(&["RCT_UPDATE_INTERVAL", "TTC_UPDATE_INTERVAL"]) {
        cfg.update_interval_mins = v
            .parse::<u64>()
            .map_err(|_| CareError::Config(format!("invalid update_interval: {v}")))?
            .max(1);
    }
    if let Some(v) = env_first(&["RCT_TERMINAL_TITLE", "TTC_TERMINAL_TITLE"]) {
        cfg.terminal_title = v;
    }
    if let Some(v) = env_first(&["RCT_POMODORO", "TTC_POMODORO"]) {
        cfg.pomodoro = v
            .parse()
            .map_err(|_| CareError::Config(format!("invalid pomodoro: {v}")))?;
    }
    if let Some(v) = env_first(&["RCT_BREAK", "TTC_BREAK"]) {
        cfg.break_minutes = v
            .parse()
            .map_err(|_| CareError::Config(format!("invalid break_minutes: {v}")))?;
    }
    if let Some(v) = env_first(&["RCT_MOCK"]) {
        cfg.mock = parse_bool(&v)?;
    }
    Ok(())
}

fn env_first(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|k| env::var(k).ok())
}

fn split_csv(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn expand_repos(raw: &[String]) -> Vec<PathBuf> {
    raw.iter()
        .map(|p| {
            let expanded = shellexpand::tilde(p).into_owned();
            PathBuf::from(expanded)
        })
        .collect()
}

fn parse_bool(v: &str) -> Result<bool> {
    match v.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        other => Err(CareError::Config(format!("invalid bool: {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bots_have_rss_urls() {
        let cfg = Config::default();
        assert_eq!(cfg.bots.len(), 3);
        assert!(cfg.bots[0].rss_url().unwrap().contains("selfcare.rss"));
    }
}
