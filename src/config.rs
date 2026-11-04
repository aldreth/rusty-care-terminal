use std::path::PathBuf;

use crate::error::{AppError, Result};
use crate::model::{Animal, BotSource};
use crate::theme::ThemeMode;

const DEFAULT_BOTS: &[&str] = &[
    "selfcare@hachyderm.io",
    "selfcaregentle@hachyderm.io",
    // GoatsLive has a lively public RSS; 3goodthings group feed is often empty.
    "GoatsLive@mastodon.social",
];

pub const MAX_POMODORO_MINS: u64 = 120;
pub const MAX_BREAK_MINS: u64 = 60;

#[derive(Debug, Clone)]
pub struct Config {
    /// Mastodon accounts or feed URLs; the first feeds the say-box.
    pub bots: Vec<BotSource>,
    /// Starting say-box animal; `None` = RANDOM (picked per message).
    pub say_box: Option<Animal>,
    pub repos: Vec<PathBuf>,
    pub repos_depth: usize,
    /// Your commit emails, lowercased; empty means each repo's `user.email`.
    pub git_author_emails: Vec<String>,
    pub weather: Option<String>,
    pub celsius: bool,
    pub update_interval_mins: u64,
    /// `None` leaves the terminal's title alone.
    pub terminal_title: Option<String>,
    pub pomodoro: u64,
    pub break_minutes: u64,
    pub mock: bool,
    /// UI palette: auto (default: terminal then OS), dark, or light.
    pub theme: ThemeMode,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bots: DEFAULT_BOTS.iter().map(|b| BotSource::parse(b)).collect(),
            say_box: Some(Animal::Parrot),
            repos: Vec::new(),
            repos_depth: 1,
            git_author_emails: Vec::new(),
            weather: None,
            celsius: true,
            update_interval_mins: 20,
            terminal_title: Some("Rusty care terminal".into()),
            pomodoro: 25,
            break_minutes: 5,
            mock: false,
            theme: ThemeMode::Auto,
        }
    }
}

/// Values from clap: CLI flags, or their `RCT_*` env vars (clap reads those).
#[derive(Debug, Default)]
pub struct CliOverrides {
    pub repos: Option<String>,
    pub repos_depth: Option<usize>,
    pub bots: Option<String>,
    pub say_box: Option<String>,
    pub git_author_emails: Option<String>,
    pub update_interval: Option<u64>,
    pub weather: Option<String>,
    pub celsius: Option<bool>,
    pub terminal_title: Option<String>,
    pub pomodoro: Option<u64>,
    pub break_minutes: Option<u64>,
    pub mock: bool,
    pub theme: Option<String>,
}

impl Config {
    /// Precedence: CLI flag / `RCT_*` (via clap) > legacy `TTC_*` from `env` > defaults.
    pub fn load(cli: CliOverrides, env: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let mut cfg = Config::default();
        cfg.apply_legacy_env(&env)?;
        cfg.apply_cli(cli)?;
        cfg.normalize();
        Ok(cfg)
    }

    /// Original tiny-care-terminal variables. Values that only made sense there
    /// (Twitter handles, cowsay/ANSI art) are ignored rather than fatal.
    fn apply_legacy_env(&mut self, env: &impl Fn(&str) -> Option<String>) -> Result<()> {
        if let Some(v) = env("TTC_BOTS")
            && let Ok(bots) = parse_bots(&v)
        {
            self.bots = bots;
        }
        if let Some(v) = env("TTC_SAY_BOX")
            && let Ok(animal) = parse_say_box(&v)
        {
            self.say_box = animal;
        }
        if let Some(v) = env("TTC_REPOS") {
            self.repos = parse_repos(&v);
        }
        if let Some(v) = env("TTC_REPOS_DEPTH") {
            self.repos_depth = parse_num("TTC_REPOS_DEPTH", &v)?;
        }
        if let Some(v) = env("TTC_WEATHER") {
            self.weather = non_empty(v);
        }
        if let Some(v) = env("TTC_CELSIUS") {
            self.celsius = parse_bool("TTC_CELSIUS", &v)?;
        }
        if let Some(v) = env("TTC_UPDATE_INTERVAL") {
            self.update_interval_mins = parse_num("TTC_UPDATE_INTERVAL", &v)?;
        }
        if let Some(v) = env("TTC_TERMINAL_TITLE") {
            self.terminal_title = parse_title(&v);
        }
        if let Some(v) = env("TTC_POMODORO") {
            self.pomodoro = parse_num("TTC_POMODORO", &v)?;
        }
        if let Some(v) = env("TTC_BREAK") {
            self.break_minutes = parse_num("TTC_BREAK", &v)?;
        }
        Ok(())
    }

    fn apply_cli(&mut self, cli: CliOverrides) -> Result<()> {
        if let Some(repos) = cli.repos {
            self.repos = parse_repos(&repos);
        }
        if let Some(depth) = cli.repos_depth {
            self.repos_depth = depth;
        }
        if let Some(bots) = cli.bots {
            self.bots = parse_bots(&bots)?;
        }
        if let Some(say_box) = cli.say_box {
            self.say_box = parse_say_box(&say_box)?;
        }
        if let Some(emails) = cli.git_author_emails {
            self.git_author_emails = parse_emails(&emails)?;
        }
        if let Some(mins) = cli.update_interval {
            self.update_interval_mins = mins;
        }
        if let Some(weather) = cli.weather {
            self.weather = non_empty(weather);
        }
        if let Some(celsius) = cli.celsius {
            self.celsius = celsius;
        }
        if let Some(title) = cli.terminal_title {
            self.terminal_title = parse_title(&title);
        }
        if let Some(pomodoro) = cli.pomodoro {
            self.pomodoro = pomodoro;
        }
        if let Some(break_minutes) = cli.break_minutes {
            self.break_minutes = break_minutes;
        }
        self.mock |= cli.mock;
        if let Some(theme) = cli.theme {
            self.theme = ThemeMode::parse(&theme).ok_or_else(|| {
                AppError::Config(format!("invalid theme: {theme} (auto|dark|light)"))
            })?;
        }
        Ok(())
    }

    fn normalize(&mut self) {
        self.repos_depth = self.repos_depth.clamp(1, 5);
        self.update_interval_mins = self.update_interval_mins.clamp(1, 24 * 60);
        self.pomodoro = self.pomodoro.clamp(1, MAX_POMODORO_MINS);
        self.break_minutes = self.break_minutes.clamp(1, MAX_BREAK_MINS);
    }
}

fn split_csv(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn non_empty(v: String) -> Option<String> {
    let trimmed = v.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn parse_repos(raw: &str) -> Vec<PathBuf> {
    split_csv(raw).iter().map(|p| expand_tilde(p)).collect()
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(home) = std::env::home_dir() {
        if path == "~" {
            return home;
        }
        if let Some(rest) = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\")) {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

fn parse_bots(raw: &str) -> Result<Vec<BotSource>> {
    split_csv(raw)
        .iter()
        .map(|b| {
            let source = BotSource::parse(b);
            source.rss_url().map(|_| source).ok_or_else(|| {
                AppError::Config(format!(
                    "invalid bot: {b} (expected user@instance or an https:// feed URL)"
                ))
            })
        })
        .collect()
}

fn parse_emails(raw: &str) -> Result<Vec<String>> {
    split_csv(raw)
        .into_iter()
        .map(|email| {
            let valid = email.contains('@') && !email.contains(['<', '>', ' ']);
            valid
                .then(|| email.to_lowercase())
                .ok_or_else(|| AppError::Config(format!("invalid git author email: {email}")))
        })
        .collect()
}

fn parse_say_box(v: &str) -> Result<Option<Animal>> {
    if v.trim().eq_ignore_ascii_case("random") {
        return Ok(None);
    }
    Animal::parse(v).map(Some).ok_or_else(|| {
        AppError::Config(format!(
            "invalid say box: {v} (parrot|cat|bunny|llama|random)"
        ))
    })
}

/// Empty or `false` (the original tiny-care opt-out) leaves the title untouched.
fn parse_title(v: &str) -> Option<String> {
    let title = crate::model::clean_text(v);
    (!title.is_empty() && !title.eq_ignore_ascii_case("false")).then_some(title)
}

fn parse_num<T: std::str::FromStr>(key: &str, v: &str) -> Result<T> {
    v.trim()
        .parse()
        .map_err(|_| AppError::Config(format!("{key}: expected a whole number, got {v:?}")))
}

fn parse_bool(key: &str, v: &str) -> Result<bool> {
    match v.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(AppError::Config(format!(
            "{key}: expected true/false, got {v:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn load(cli: CliOverrides, env: &[(&str, &str)]) -> Result<Config> {
        let env: HashMap<String, String> = env
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::load(cli, |k| env.get(k).cloned())
    }

    #[test]
    fn default_bots_have_rss_urls() {
        let cfg = load(CliOverrides::default(), &[]).unwrap();
        assert_eq!(cfg.bots.len(), 3);
        assert!(cfg.bots[0].rss_url().unwrap().contains("selfcare.rss"));
        assert_eq!(cfg.terminal_title.as_deref(), Some("Rusty care terminal"));
    }

    #[test]
    fn say_box_accepts_animals_and_random_only() {
        assert_eq!(parse_say_box("Cat").unwrap(), Some(Animal::Cat));
        assert_eq!(parse_say_box("RANDOM").unwrap(), None);
        assert!(parse_say_box("~/art/parrot.png").is_err());
    }

    #[test]
    fn cli_and_rct_beat_legacy_env() {
        let cli = CliOverrides {
            weather: Some("Leeds".into()),
            pomodoro: Some(50),
            ..Default::default()
        };
        let cfg = load(
            cli,
            &[
                ("TTC_WEATHER", "Paris"),
                ("TTC_POMODORO", "10"),
                ("TTC_BREAK", "7"),
            ],
        )
        .unwrap();
        assert_eq!(cfg.weather.as_deref(), Some("Leeds"));
        assert_eq!(cfg.pomodoro, 50);
        assert_eq!(cfg.break_minutes, 7);
    }

    #[test]
    fn legacy_values_from_the_original_app_are_ignored_not_fatal() {
        let cfg = load(
            CliOverrides::default(),
            &[
                ("TTC_BOTS", "tinycarebot,selfcare_bot,magicrealismbot"),
                ("TTC_SAY_BOX", "stegosaurus.cow"),
                ("TTC_TERMINAL_TITLE", "false"),
            ],
        )
        .unwrap();
        assert_eq!(cfg.bots, Config::default().bots);
        assert_eq!(cfg.say_box, Some(Animal::Parrot));
        assert_eq!(cfg.terminal_title, None);
    }

    #[test]
    fn explicit_bad_values_are_errors() {
        for cli in [
            CliOverrides {
                bots: Some("tinycarebot".into()),
                ..Default::default()
            },
            CliOverrides {
                say_box: Some("yeoman".into()),
                ..Default::default()
            },
            CliOverrides {
                theme: Some("solarized".into()),
                ..Default::default()
            },
            CliOverrides {
                git_author_emails: Some("Edward".into()),
                ..Default::default()
            },
        ] {
            assert!(load(cli, &[]).is_err());
        }
        assert!(load(CliOverrides::default(), &[("TTC_CELSIUS", "maybe")]).is_err());
    }

    #[test]
    fn values_are_clamped_the_same_from_every_source() {
        let cfg = load(
            CliOverrides {
                pomodoro: Some(0),
                repos_depth: Some(99),
                ..Default::default()
            },
            &[("TTC_BREAK", "500"), ("TTC_UPDATE_INTERVAL", "0")],
        )
        .unwrap();
        assert_eq!(cfg.pomodoro, 1);
        assert_eq!(cfg.repos_depth, 5);
        assert_eq!(cfg.break_minutes, MAX_BREAK_MINS);
        assert_eq!(cfg.update_interval_mins, 1);
    }

    #[test]
    fn git_author_emails_are_trimmed_and_lowercased() {
        let cfg = load(
            CliOverrides {
                git_author_emails: Some(" Ed@Example.com, ed@work.example ".into()),
                ..Default::default()
            },
            &[],
        )
        .unwrap();
        assert_eq!(cfg.git_author_emails, ["ed@example.com", "ed@work.example"]);
    }

    #[test]
    fn bots_take_accounts_and_feed_urls() {
        let cfg = load(
            CliOverrides {
                bots: Some("@a@example.social, https://example.com/b.rss".into()),
                ..Default::default()
            },
            &[],
        )
        .unwrap();
        assert_eq!(
            cfg.bots,
            [
                BotSource::Acct("a@example.social".into()),
                BotSource::FeedUrl("https://example.com/b.rss".into()),
            ]
        );
    }
}
