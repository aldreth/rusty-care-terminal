use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::config::{Config, MAX_BREAK_MINS, MAX_POMODORO_MINS};
use crate::model::{Animal, CareMessage, CareSource, CommitStats, WeatherSnapshot};
use crate::services::{self, Snapshot, mock};
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    Continue,
    Quit,
}

/// Full-screen commit list so terminal text selection stays in-bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusView {
    #[default]
    None,
    Today,
    Week,
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
        let elapsed = self.last_tick.elapsed();
        self.last_tick = Instant::now();
        if !self.active || !self.running {
            return;
        }
        if elapsed >= self.remaining {
            self.phase = match self.phase {
                PomodoroPhase::Work => PomodoroPhase::Break,
                PomodoroPhase::Break => PomodoroPhase::Work,
            };
            self.reset_phase();
        } else {
            self.remaining -= elapsed;
        }
    }

    fn adjust(&mut self, phase: PomodoroPhase, delta: i64) {
        let (mins, max) = match phase {
            PomodoroPhase::Work => (&mut self.work_mins, MAX_POMODORO_MINS),
            PomodoroPhase::Break => (&mut self.break_mins, MAX_BREAK_MINS),
        };
        *mins = mins.saturating_add_signed(delta).clamp(1, max);
        if self.phase == phase && !self.running {
            self.reset_phase();
        }
    }

    pub fn mmss(&self) -> String {
        let secs = self.remaining.as_secs();
        format!("{:02}:{:02}", secs / 60, secs % 60)
    }
}

pub struct App {
    pub config: Config,
    pub theme: Theme,
    pub commits: CommitStats,
    pub care: Vec<CareMessage>,
    pub weather: Option<WeatherSnapshot>,
    /// A refresh is in flight (first load included).
    pub loading: bool,
    /// First problem from the last refresh, shown in the footer.
    pub warning: Option<String>,
    pub pomodoro: PomodoroState,
    pub scroll_today: u16,
    pub scroll_week: u16,
    /// Which commit list receives j/k scrolling: true = today, false = week.
    pub scroll_today_focus: bool,
    pub show_help: bool,
    pub focus: FocusView,
    /// Say-box animal; `None` = RANDOM until the user presses `a`.
    pub animal: Option<Animal>,
    last_refresh: Instant,
    pending: Option<Receiver<Snapshot>>,
}

impl App {
    /// Builds the app and starts the first refresh in the background.
    pub fn new(config: Config) -> Self {
        let mut app = Self {
            theme: Theme::for_mode(config.theme),
            pomodoro: PomodoroState::new(config.pomodoro, config.break_minutes),
            animal: config.say_box,
            config,
            commits: CommitStats::default(),
            care: Vec::new(),
            weather: None,
            loading: false,
            warning: None,
            scroll_today: 0,
            scroll_week: 0,
            scroll_today_focus: true,
            show_help: false,
            focus: FocusView::None,
            last_refresh: Instant::now(),
            pending: None,
        };
        app.start_refresh();
        app
    }

    pub fn say_box_message(&self) -> &str {
        self.care
            .first()
            .map(|m| m.text.as_str())
            .unwrap_or(mock::DEFAULT_MESSAGE)
    }

    /// Label for care slot `index`, known from config before the first fetch lands
    /// so the panels don't shift when messages arrive.
    pub fn care_label(&self, index: usize) -> Option<String> {
        self.config.care_sources().get(index).map(CareSource::label)
    }

    pub fn current_animal(&self) -> Animal {
        self.animal
            .unwrap_or_else(|| Animal::for_message(self.say_box_message()))
    }

    /// Kicks off git / care / weather on a worker thread; ignored if one is running.
    pub fn start_refresh(&mut self) {
        if self.pending.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let config = self.config.clone();
        let spawned = thread::Builder::new()
            .name("refresh".into())
            .spawn(move || {
                let _ = tx.send(services::fetch_all(&config));
            });
        match spawned {
            Ok(_) => {
                self.pending = Some(rx);
                self.loading = true;
            }
            Err(e) => self.warning = Some(format!("could not start refresh: {e}")),
        }
    }

    /// Applies a finished refresh, if there is one. Never blocks.
    pub fn poll_refresh(&mut self) {
        let Some(rx) = &self.pending else {
            return;
        };
        match rx.try_recv() {
            Ok(snapshot) => self.apply(snapshot),
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                self.warning = Some("refresh failed unexpectedly".into());
            }
        }
        self.pending = None;
        self.loading = false;
        self.last_refresh = Instant::now();
    }

    fn apply(&mut self, snapshot: Snapshot) {
        self.commits = snapshot.commits;
        if !snapshot.care.is_empty() {
            self.care = snapshot.care;
        }
        match snapshot.weather {
            Some(Ok(weather)) => self.weather = Some(weather),
            // Keep the last good reading through a transient failure.
            Some(Err(_)) => {}
            None => self.weather = None,
        }
        self.warning = snapshot.warnings.into_iter().next();
        self.scroll_today = 0;
        self.scroll_week = 0;
    }

    fn should_auto_refresh(&self) -> bool {
        let interval = Duration::from_secs(self.config.update_interval_mins.saturating_mul(60));
        self.pending.is_none() && self.last_refresh.elapsed() >= interval
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AppAction {
        if key.kind != KeyEventKind::Press {
            return AppAction::Continue;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return if matches!(key.code, KeyCode::Char('c') | KeyCode::Char('d')) {
                AppAction::Quit
            } else {
                AppAction::Continue
            };
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.show_help {
                    self.show_help = false;
                } else if self.focus != FocusView::None {
                    self.focus = FocusView::None;
                } else {
                    return AppAction::Quit;
                }
            }
            KeyCode::Char('h') | KeyCode::Char('?') => self.show_help = !self.show_help,
            KeyCode::Char('t') => self.toggle_focus(FocusView::Today),
            KeyCode::Char('w') => self.toggle_focus(FocusView::Week),
            KeyCode::Char('r') => {
                self.show_help = false;
                self.focus = FocusView::None;
                self.start_refresh();
            }
            KeyCode::Tab | KeyCode::BackTab => self.scroll_today_focus = !self.scroll_today_focus,
            KeyCode::Char('a') => self.animal = Some(self.current_animal().next()),
            KeyCode::Char('j') | KeyCode::Down => self.scroll_by(1),
            KeyCode::Char('k') | KeyCode::Up => self.scroll_by(-1),
            KeyCode::Char('p') => {
                self.pomodoro.active = !self.pomodoro.active;
                self.pomodoro.phase = PomodoroPhase::Work;
                self.pomodoro.reset_phase();
            }
            KeyCode::Char('s') if self.pomodoro.active => {
                self.pomodoro.running = !self.pomodoro.running;
            }
            KeyCode::Char('e') if self.pomodoro.active => self.pomodoro.reset_phase(),
            KeyCode::Char('u') if self.pomodoro.active => {
                self.pomodoro.adjust(PomodoroPhase::Work, 1);
            }
            KeyCode::Char('U') if self.pomodoro.active => {
                self.pomodoro.adjust(PomodoroPhase::Work, -1);
            }
            KeyCode::Char('b') if self.pomodoro.active => {
                self.pomodoro.adjust(PomodoroPhase::Break, 1);
            }
            KeyCode::Char('B') if self.pomodoro.active => {
                self.pomodoro.adjust(PomodoroPhase::Break, -1);
            }
            _ => {}
        }
        AppAction::Continue
    }

    fn toggle_focus(&mut self, view: FocusView) {
        self.show_help = false;
        self.focus = if self.focus == view {
            FocusView::None
        } else {
            view
        };
        self.scroll_today_focus = view == FocusView::Today;
    }

    fn scroll_by(&mut self, delta: i16) {
        let scroll = if self.scroll_today_focus {
            &mut self.scroll_today
        } else {
            &mut self.scroll_week
        };
        *scroll = scroll.saturating_add_signed(delta);
    }

    /// Waits up to `timeout` for input; also advances timers and background work.
    pub fn poll_events(&mut self, timeout: Duration) -> anyhow::Result<AppAction> {
        self.pomodoro.tick();
        self.poll_refresh();
        if self.should_auto_refresh() {
            self.start_refresh();
        }
        if event::poll(timeout)?
            && let Event::Key(key) = event::read()?
        {
            return Ok(self.handle_key(key));
        }
        Ok(AppAction::Continue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offline_app() -> App {
        let config = Config {
            mock: true,
            ..Config::default()
        };
        App::new(config)
    }

    fn press(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> AppAction {
        app.handle_key(KeyEvent::new(code, modifiers))
    }

    fn wait_for_refresh(app: &mut App) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while app.loading && Instant::now() < deadline {
            app.poll_refresh();
            thread::sleep(Duration::from_millis(5));
        }
        assert!(!app.loading, "refresh did not finish");
    }

    #[test]
    fn refresh_runs_in_the_background_and_applies() {
        let mut app = offline_app();
        assert!(app.loading);
        wait_for_refresh(&mut app);
        assert_eq!(app.care.len(), crate::model::CARE_SLOTS);
        assert_eq!(app.warning, None);
    }

    #[test]
    fn care_labels_are_known_before_the_first_refresh() {
        let app = offline_app();
        assert!(app.care.is_empty());
        let labels: Vec<_> = (0..crate::model::CARE_SLOTS)
            .map(|i| app.care_label(i))
            .collect();
        let expected: Vec<_> = app
            .config
            .care_sources()
            .iter()
            .map(|s| Some(s.label()))
            .collect();
        assert_eq!(labels, expected);
    }

    #[test]
    fn refresh_is_not_started_twice() {
        let mut app = offline_app();
        press(&mut app, KeyCode::Char('r'), KeyModifiers::NONE);
        assert!(app.loading);
        wait_for_refresh(&mut app);
        press(&mut app, KeyCode::Char('r'), KeyModifiers::NONE);
        assert!(app.loading);
        wait_for_refresh(&mut app);
    }

    #[test]
    fn ctrl_c_quits_even_with_help_open() {
        let mut app = offline_app();
        press(&mut app, KeyCode::Char('h'), KeyModifiers::NONE);
        assert_eq!(
            press(&mut app, KeyCode::Char('c'), KeyModifiers::CONTROL),
            AppAction::Quit
        );
    }

    #[test]
    fn q_closes_overlays_before_quitting() {
        let mut app = offline_app();
        press(&mut app, KeyCode::Char('t'), KeyModifiers::NONE);
        assert_eq!(app.focus, FocusView::Today);
        assert_eq!(
            press(&mut app, KeyCode::Char('q'), KeyModifiers::NONE),
            AppAction::Continue
        );
        assert_eq!(
            press(&mut app, KeyCode::Char('q'), KeyModifiers::NONE),
            AppAction::Quit
        );
    }

    #[test]
    fn pomodoro_minutes_stay_in_range() {
        let mut app = offline_app();
        press(&mut app, KeyCode::Char('p'), KeyModifiers::NONE);
        for _ in 0..200 {
            press(&mut app, KeyCode::Char('U'), KeyModifiers::SHIFT);
            press(&mut app, KeyCode::Char('b'), KeyModifiers::NONE);
        }
        assert_eq!(app.pomodoro.work_mins, 1);
        assert_eq!(app.pomodoro.break_mins, MAX_BREAK_MINS);
        assert_eq!(app.pomodoro.mmss(), "01:00");
    }

    #[test]
    fn a_cycles_animals() {
        let mut app = offline_app();
        let first = app.current_animal();
        press(&mut app, KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(app.current_animal(), first.next());
    }
}
