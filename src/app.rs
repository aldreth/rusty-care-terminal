use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::config::Config;
use crate::model::{CareMessage, CommitStats};
use crate::services::{git, mastodon};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    Continue,
    Quit,
}

pub struct App {
    pub config: Config,
    pub commits: CommitStats,
    pub care: Vec<CareMessage>,
    pub status: String,
    pub last_refresh: Instant,
    pub loading: bool,
    pub error: Option<String>,
    pub warnings: Vec<String>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let mut app = Self {
            config,
            commits: CommitStats::default(),
            care: Vec::new(),
            status: "starting…".into(),
            last_refresh: Instant::now(),
            loading: true,
            error: None,
            warnings: Vec::new(),
        };
        app.refresh();
        app
    }

    pub fn refresh(&mut self) {
        self.loading = true;
        self.status = "refreshing…".into();
        self.warnings.clear();

        let (stats, git_warnings) = git::collect_commits(
            &self.config.repos,
            self.config.repos_depth,
            self.config.git_author.as_deref(),
        );
        self.commits = stats;
        self.warnings.extend(git_warnings);

        let (care, care_warnings) = mastodon::fetch_care_messages(&self.config);
        self.care = care;
        self.warnings.extend(care_warnings);

        let repo_note = if self.config.repos.is_empty() {
            "no repos configured (set RCT_REPOS or --repos)"
        } else {
            "git scanned"
        };

        let live = self.care.iter().filter(|m| !m.from_mock).count();
        let care_note = if self.config.mock {
            "care mock"
        } else if live == 0 {
            "care offline"
        } else {
            "care rss"
        };

        let warn_note = if self.warnings.is_empty() {
            String::new()
        } else {
            format!(" · {} warn", self.warnings.len())
        };

        self.status = format!(
            "{repo_note} · today {} · week {} · {care_note}{warn_note} · q quit · r refresh",
            self.commits.today_count(),
            self.commits.week_count()
        );
        self.error = self.warnings.first().cloned();
        self.last_refresh = Instant::now();
        self.loading = false;
    }

    pub fn should_auto_refresh(&self) -> bool {
        let interval = Duration::from_secs(self.config.update_interval_mins.saturating_mul(60));
        self.last_refresh.elapsed() >= interval
    }

    pub fn handle_event(&mut self, ev: Event) -> AppAction {
        if let Event::Key(key) = ev {
            if key.kind != KeyEventKind::Press {
                return AppAction::Continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return AppAction::Quit,
                KeyCode::Char('r') => self.refresh(),
                _ => {}
            }
        }
        AppAction::Continue
    }

    pub fn poll_events(&mut self, timeout: Duration) -> anyhow::Result<AppAction> {
        if event::poll(timeout)? {
            let ev = event::read()?;
            return Ok(self.handle_event(ev));
        }
        if self.should_auto_refresh() {
            self.refresh();
        }
        Ok(AppAction::Continue)
    }
}
