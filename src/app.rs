use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::config::Config;
use crate::model::{CareMessage, CommitStats};
use crate::services::{git, mock};

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
        };
        app.refresh();
        app
    }

    pub fn refresh(&mut self) {
        self.loading = true;
        self.status = "refreshing…".into();

        match git::collect_commits(&self.config.repos, self.config.repos_depth) {
            Ok(stats) => {
                self.commits = stats;
                self.error = None;
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }

        // MVP: always use mock care text; types already encode RSS intent.
        self.care = mock::mock_care_messages(&self.config);

        let repo_note = if self.config.repos.is_empty() {
            "no repos configured (set CARE_REPOS or config.toml)"
        } else {
            "git scanned"
        };

        self.status = format!(
            "{repo_note} · today {} · week {} · care mock · q quit · r refresh",
            self.commits.today_count(),
            self.commits.week_count()
        );
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
