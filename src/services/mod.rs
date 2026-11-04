pub mod cache;
pub mod git;
pub mod http;
pub mod mastodon;
pub mod mock;
pub mod weather;

use std::thread;

use crate::config::Config;
use crate::model::{BotMessage, CommitStats, WeatherSnapshot};

/// Everything one refresh produces.
pub struct Snapshot {
    pub commits: CommitStats,
    pub messages: Vec<BotMessage>,
    /// `None` when no location is configured.
    pub weather: Option<Result<WeatherSnapshot, String>>,
    pub warnings: Vec<String>,
}

/// Git, bot feeds and weather in parallel; blocking, so call it off the UI thread.
pub fn fetch_all(config: &Config) -> Snapshot {
    thread::scope(|s| {
        let git = s.spawn(|| {
            git::collect_commits(&config.repos, config.repos_depth, &config.git_author_emails)
        });
        let weather = s.spawn(|| {
            config
                .weather
                .as_deref()
                .map(|loc| weather::fetch_weather(loc, config.celsius).map_err(|e| e.to_string()))
        });
        let (messages, bot_warnings) = mastodon::fetch_messages(config);

        let mut warnings = Vec::new();
        let commits = match git.join() {
            Ok((commits, git_warnings)) => {
                warnings.extend(git_warnings);
                commits
            }
            Err(_) => {
                warnings.push("git scan crashed".into());
                CommitStats::default()
            }
        };
        warnings.extend(bot_warnings);
        let weather = weather
            .join()
            .unwrap_or_else(|_| Some(Err("weather lookup crashed".into())));
        if let Some(Err(e)) = &weather {
            warnings.push(e.clone());
        }

        Snapshot {
            commits,
            messages,
            weather,
            warnings,
        }
    })
}
