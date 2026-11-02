use std::time::{Duration, Instant};

use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::config::Config;
use crate::model::{CareMessage, CommitStats, WeatherSnapshot};
use crate::services::{git, mastodon, weather};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    Continue,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PomodoroPhase {
    Work,
    Break,
}

#[derive(Debug, Clone)]
pub struct PomodoroState {
    pub active: bool,
    pub running: bool,
    pub phase: PomodoroPhase,
    pub remaining: Duration,
    pub work_mins: u64,
    pub break_mins: u64,
    last_tick: Instant,
}

impl PomodoroState {
    fn new(work_mins: u64, break_mins: u64) -> Self {
        Self {
            active: false,
            running: false,
            phase: PomodoroPhase::Work,
            remaining: Duration::from_secs(work_mins.saturating_mul(60)),
            work_mins,
            break_mins,
            last_tick: Instant::now(),
        }
    }

    fn reset_phase(&mut self) {
        let mins = match self.phase {
            PomodoroPhase::Work => self.work_mins,
            PomodoroPhase::Break => self.break_mins,
        };
        self.remaining = Duration::from_secs(mins.saturating_mul(60));
        self.running = false;
        self.last_tick = Instant::now();
    }

    fn tick(&mut self) {
        if !self.active || !self.running {
            self.last_tick = Instant::now();
            return;
        }
        let elapsed = self.last_tick.elapsed();
        self.last_tick = Instant::now();
        if elapsed >= self.remaining {
            self.remaining = Duration::ZERO;
            self.running = false;
            self.phase = match self.phase {
                PomodoroPhase::Work => PomodoroPhase::Break,
                PomodoroPhase::Break => PomodoroPhase::Work,
            };
            self.reset_phase();
        } else {
            self.remaining -= elapsed;
        }
    }

    pub fn mmss(&self) -> String {
        let secs = self.remaining.as_secs();
        format!("{:02}:{:02}", secs / 60, secs % 60)
    }
}

pub struct App {
    pub config: Config,
    pub commits: CommitStats,
    pub care: Vec<CareMessage>,
    pub weather: Option<WeatherSnapshot>,
    pub status: String,
    pub last_refresh: Instant,
    pub last_refresh_at: chrono::DateTime<Local>,
    pub loading: bool,
    pub error: Option<String>,
    pub warnings: Vec<String>,
    pub pomodoro: PomodoroState,
}

impl App {
    pub fn new(config: Config) -> Self {
        let pomodoro = PomodoroState::new(config.pomodoro, config.break_minutes);
        let mut app = Self {
            config,
            commits: CommitStats::default(),
            care: Vec::new(),
            weather: None,
            status: "starting…".into(),
            last_refresh: Instant::now(),
            last_refresh_at: Local::now(),
            loading: true,
            error: None,
            warnings: Vec::new(),
            pomodoro,
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

        if let Some(loc) = self.config.weather.clone() {
            match weather::fetch_weather(&loc, self.config.celsius) {
                Ok(snap) => self.weather = Some(snap),
                Err(e) => {
                    self.warnings.push(format!("weather: {e}"));
                }
            }
        }

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

        let weather_note = if self.config.weather.is_none() {
            ""
        } else if self.weather.is_some() {
            " · wx ok"
        } else {
            " · wx err"
        };

        let warn_note = if self.warnings.is_empty() {
            String::new()
        } else {
            format!(" · {} warn", self.warnings.len())
        };

        let updated = Local::now().format("%H:%M").to_string();
        self.status = format!(
            "{repo_note} · today {} · week {} · {care_note}{weather_note}{warn_note} · upd {updated} · q quit · r refresh · p pomo",
            self.commits.today_count(),
            self.commits.week_count()
        );
        self.error = self.warnings.first().cloned();
        self.last_refresh = Instant::now();
        self.last_refresh_at = Local::now();
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
                KeyCode::Char('p') => {
                    self.pomodoro.active = !self.pomodoro.active;
                    if self.pomodoro.active {
                        self.pomodoro.phase = PomodoroPhase::Work;
                        self.pomodoro.reset_phase();
                    } else {
                        self.pomodoro.running = false;
                    }
                }
                KeyCode::Char('s') if self.pomodoro.active => {
                    self.pomodoro.running = !self.pomodoro.running;
                    self.pomodoro.last_tick = Instant::now();
                }
                KeyCode::Char('e') if self.pomodoro.active => {
                    self.pomodoro.phase = PomodoroPhase::Work;
                    self.pomodoro.reset_phase();
                }
                KeyCode::Char('u') if self.pomodoro.active => {
                    self.pomodoro.work_mins = self.pomodoro.work_mins.saturating_add(1).min(120);
                    if matches!(self.pomodoro.phase, PomodoroPhase::Work) && !self.pomodoro.running {
                        self.pomodoro.reset_phase();
                    }
                }
                KeyCode::Char('b') if self.pomodoro.active => {
                    self.pomodoro.break_mins = self.pomodoro.break_mins.saturating_add(1).min(60);
                    if matches!(self.pomodoro.phase, PomodoroPhase::Break) && !self.pomodoro.running
                    {
                        self.pomodoro.reset_phase();
                    }
                }
                _ => {}
            }
        }
        AppAction::Continue
    }

    pub fn poll_events(&mut self, timeout: Duration) -> anyhow::Result<AppAction> {
        self.pomodoro.tick();
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
