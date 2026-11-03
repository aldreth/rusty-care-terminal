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

/// Full-screen view of one panel so terminal text selection stays in-bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusView {
    #[default]
    None,
    Today,
    Week,
    /// Care slot: 0 = say-box source, 1/2 = side care panels.
    Care(usize),
    Weather,
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
    pub scroll_today: u16,
    pub scroll_week: u16,
    /// Which commit list receives j/k scrolling: true = today, false = week.
    pub scroll_today_focus: bool,
    pub show_help: bool,
    pub focus: FocusView,
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
            scroll_today: 0,
            scroll_week: 0,
            scroll_today_focus: true,
            show_help: false,
            focus: FocusView::None,
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

        // Keep the footer short: shortcuts + a single warning if something failed.
        self.status =
            "q quit · r refresh · t theme · h help · 1-3/g/w focus-to-copy · Tab/j/k · p pomo"
                .into();
        self.error = self.warnings.first().cloned();
        self.last_refresh = Instant::now();
        self.last_refresh_at = Local::now();
        self.scroll_today = 0;
        self.scroll_week = 0;
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
                KeyCode::Char('q') => {
                    if self.show_help {
                        self.show_help = false;
                    } else if self.focus != FocusView::None {
                        self.focus = FocusView::None;
                    } else {
                        return AppAction::Quit;
                    }
                }
                KeyCode::Esc => {
                    if self.show_help {
                        self.show_help = false;
                    } else if self.focus != FocusView::None {
                        self.focus = FocusView::None;
                    } else {
                        return AppAction::Quit;
                    }
                }
                KeyCode::Char('h') => {
                    self.show_help = !self.show_help;
                }
                KeyCode::Char('1') => self.focus = FocusView::Care(0),
                KeyCode::Char('2') => self.focus = FocusView::Care(1),
                KeyCode::Char('3') => self.focus = FocusView::Care(2),
                KeyCode::Char('g') => self.focus = FocusView::Today,
                KeyCode::Char('w') => {
                    // Weather focus (also closes help if open).
                    self.show_help = false;
                    self.focus = FocusView::Weather;
                }
                KeyCode::Char('G') => self.focus = FocusView::Week,
                KeyCode::Char('r') => {
                    self.show_help = false;
                    self.focus = FocusView::None;
                    self.refresh();
                }
                KeyCode::Tab => {
                    self.scroll_today_focus = !self.scroll_today_focus;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    if self.scroll_today_focus {
                        self.scroll_today = self.scroll_today.saturating_add(1);
                    } else {
                        self.scroll_week = self.scroll_week.saturating_add(1);
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if self.scroll_today_focus {
                        self.scroll_today = self.scroll_today.saturating_sub(1);
                    } else {
                        self.scroll_week = self.scroll_week.saturating_sub(1);
                    }
                }
                KeyCode::Char('t') => {
                    self.config.theme = self.config.theme.toggle();
                }
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
                    if matches!(self.pomodoro.phase, PomodoroPhase::Work) && !self.pomodoro.running
                    {
                        self.pomodoro.reset_phase();
                    }
                }
                KeyCode::Char('U') if self.pomodoro.active => {
                    self.pomodoro.work_mins = self.pomodoro.work_mins.saturating_sub(1).max(1);
                    if matches!(self.pomodoro.phase, PomodoroPhase::Work) && !self.pomodoro.running
                    {
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
                KeyCode::Char('B') if self.pomodoro.active => {
                    self.pomodoro.break_mins = self.pomodoro.break_mins.saturating_sub(1).max(1);
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
