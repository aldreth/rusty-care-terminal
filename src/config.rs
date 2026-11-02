use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{CareError, Result};
use crate::model::CareSource;

const DEFAULT_BOTS: &[&str] = &[
    "selfcare@hachyderm.io",
    "selfcaregentle@hachyderm.io",
    "3goodthings@fedigroups.social",
];

#[derive(Debug, Clone)]
pub struct Config {
    pub bots: Vec<CareSource>,
    pub bot_feeds: Vec<String>,
    pub say_box: String,
    pub repos: Vec<PathBuf>,
    pub repos_depth: usize,
    pub weather: Option<String>,
    pub celsius: bool,
    pub update_interval_mins: u64,
    pub terminal_title: String,
    pub pomodoro: u64,
    pub break_minutes: u64,
    pub mock: bool,
    pub config_path: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    bots: Option<Vec<String>>,
    bot_feeds: Option<Vec<String>>,
    say_box: Option<String>,
    repos: Option<Vec<String>>,
    repos_depth: Option<usize>,
    weather: Option<String>,
    celsius: Option<bool>,
    update_interval: Option<u64>,
    terminal_title: Option<String>,
    pomodoro: Option<u64>,
    break_minutes: Option<u64>,
    mock: Option<bool>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bots: DEFAULT_BOTS.iter().map(|b| CareSource::parse(b)).collect(),
            bot_feeds: Vec::new(),
            say_box: "parrot".into(),
            repos: Vec::new(),
            repos_depth: 1,
            weather: None,
            celsius: true,
            update_interval_mins: 20,
            terminal_title: "tiny care".into(),
            pomodoro: 25,
            break_minutes: 5,
            mock: false, // live Mastodon RSS by default; --mock / RCT_MOCK for offline
            config_path: None,
        }
    }
}

impl Config {
    pub fn load(cli_config: Option<PathBuf>, force_mock: bool) -> Result<Self> {
        let mut cfg = Config::default();

        let path = cli_config
            .or_else(|| env::var_os("RCT_CONFIG").map(PathBuf::from))
            .or_else(|| env::var_os("TTC_CONFIG").map(PathBuf::from))
            .or_else(discover_config_path);

        if let Some(ref p) = path {
            if p.is_file() {
                let file = load_file(p)?;
                merge_file(&mut cfg, file);
                cfg.config_path = Some(p.clone());
            }
        }

        merge_env(&mut cfg)?;

        if force_mock {
            cfg.mock = true;
        }

        // Explicit feed URLs win / pad sources when provided.
        if !cfg.bot_feeds.is_empty() {
            let mut sources: Vec<CareSource> = cfg
                .bot_feeds
                .iter()
                .map(|u| CareSource::FeedUrl(u.clone()))
                .collect();
            // Keep acct bots beyond feed list length for labeling/defaults.
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

fn discover_config_path() -> Option<PathBuf> {
    let local = PathBuf::from("config.toml");
    if local.is_file() {
        return Some(local);
    }
    let sample = PathBuf::from("config/config.example.toml");
    if sample.is_file() {
        // Prefer example only as discovery hint — do not auto-load sample as live config.
    }
    dirs::config_dir().map(|d| d.join("rusty-care-terminal").join("config.toml"))
}

fn load_file(path: &Path) -> Result<FileConfig> {
    let text = fs::read_to_string(path).map_err(|e| {
        CareError::Config(format!("failed to read {}: {e}", path.display()))
    })?;
    toml::from_str(&text).map_err(|e| {
        CareError::Config(format!("invalid TOML in {}: {e}", path.display()))
    })
}

fn merge_file(cfg: &mut Config, file: FileConfig) {
    if let Some(bots) = file.bots {
        cfg.bots = bots.iter().map(|b| CareSource::parse(b)).collect();
    }
    if let Some(feeds) = file.bot_feeds {
        cfg.bot_feeds = feeds;
    }
    if let Some(v) = file.say_box {
        cfg.say_box = v;
    }
    if let Some(repos) = file.repos {
        cfg.repos = expand_repos(&repos);
    }
    if let Some(v) = file.repos_depth {
        cfg.repos_depth = v;
    }
    if let Some(v) = file.weather {
        cfg.weather = Some(v);
    }
    if let Some(v) = file.celsius {
        cfg.celsius = v;
    }
    if let Some(v) = file.update_interval {
        cfg.update_interval_mins = v.max(1);
    }
    if let Some(v) = file.terminal_title {
        cfg.terminal_title = v;
    }
    if let Some(v) = file.pomodoro {
        cfg.pomodoro = v;
    }
    if let Some(v) = file.break_minutes {
        cfg.break_minutes = v;
    }
    if let Some(v) = file.mock {
        cfg.mock = v;
    }
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
