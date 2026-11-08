use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::config::{Config, MAX_BREAK_MINS, MAX_POMODORO_MINS};
use crate::input::{Input, InputSource};
use crate::model::{Animal, BotMessage, BotSource, CommitStats, WeatherSnapshot};
use crate::services::{self, Snapshot, mock};
use crate::theme::{OsThemeWatch, ResolvedTheme, Theme, ThemeMode};

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
    pub messages: Vec<BotMessage>,
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
    /// Animal; `None` = RANDOM until the user presses `a`.
    pub animal: Option<Animal>,
    last_refresh: Instant,
    pending: Option<Receiver<Snapshot>>,
    /// Follows desktop light/dark changes; only in auto theme mode.
    os_theme: Option<OsThemeWatch>,
}

impl App {
    /// Builds the app and starts the first refresh in the background.
    pub fn new(config: Config) -> Self {
        let mut app = Self {
            theme: Theme::for_mode(config.theme),
            pomodoro: PomodoroState::new(config.pomodoro, config.break_minutes),
            animal: config.animal,
            config,
            commits: CommitStats::default(),
            messages: Vec::new(),
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
            os_theme: None,
        };
        if app.config.theme == ThemeMode::Auto {
            app.os_theme = OsThemeWatch::spawn();
        }
        app.start_refresh();
        app
    }

    /// A feed post's title is a better fit for the speech bubble than its body.
    pub fn animal_box_message(&self) -> &str {
        self.messages
            .first()
            .map(|m| m.title.as_deref().unwrap_or(&m.text))
            .unwrap_or(mock::DEFAULT_MESSAGE)
    }

    /// Label for bot slot `index`, known from config before the first fetch lands
    /// so the panels don't shift when messages arrive.
    pub fn bot_label(&self, index: usize) -> Option<String> {
        self.config.bots.get(index).map(BotSource::label)
    }

    pub fn current_animal(&self) -> Animal {
        self.animal
            .unwrap_or_else(|| Animal::for_message(self.animal_box_message()))
    }

    /// Kicks off git / bots / weather on a worker thread; ignored if one is running.
    pub fn start_refresh(&mut self) {
        if self.pending.is_some() {
            return;
        }
        if let Some(watch) = &self.os_theme {
            watch.check_now();
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
        if !snapshot.messages.is_empty() {
            self.messages = snapshot.messages;
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

    pub fn handle_input(&mut self, input: Input) -> AppAction {
        match input {
            Input::Key(key) => self.handle_key(key),
            Input::ColorScheme(scheme) => {
                self.follow_terminal_scheme(scheme);
                AppAction::Continue
            }
        }
    }

    /// A terminal that reports its own dark/light scheme (DEC mode 2031) knows
    /// better than the OS setting, so the OS watch stops once one arrives.
    fn follow_terminal_scheme(&mut self, scheme: ResolvedTheme) {
        if self.config.theme == ThemeMode::Auto {
            self.theme = Theme::for_resolved(scheme);
            self.os_theme = None;
        }
    }

    /// Waits up to `timeout` for input; also advances timers and background work.
    pub fn poll_events(
        &mut self,
        input: &InputSource,
        timeout: Duration,
    ) -> anyhow::Result<AppAction> {
        self.pomodoro.tick();
        self.poll_refresh();
        if let Some(flipped) = self.os_theme.as_mut().and_then(OsThemeWatch::changed) {
            self.theme = Theme::for_resolved(flipped);
        }
        if self.should_auto_refresh() {
            self.start_refresh();
        }
        Ok(match input.next(timeout)? {
            Some(input) => self.handle_input(input),
            None => AppAction::Continue,
        })
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
        assert_eq!(app.messages.len(), crate::model::BOT_SLOTS);
        assert_eq!(app.warning, None);
    }

    #[test]
    fn bot_labels_are_known_before_the_first_refresh() {
        let app = offline_app();
        assert!(app.messages.is_empty());
        let labels: Vec<_> = (0..crate::model::BOT_SLOTS)
            .map(|i| app.bot_label(i))
            .collect();
        let expected: Vec<_> = app.config.bots.iter().map(|s| Some(s.label())).collect();
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
    fn terminal_scheme_reports_switch_auto_theme_only() {
        let mut app = offline_app();
        app.handle_input(Input::ColorScheme(ResolvedTheme::Light));
        assert_eq!(app.theme.border, Theme::light().border);
        assert!(app.os_theme.is_none());
        app.handle_input(Input::ColorScheme(ResolvedTheme::Dark));
        assert_eq!(app.theme.border, Theme::dark().border);

        let mut pinned = App::new(Config {
            mock: true,
            theme: ThemeMode::Dark,
            ..Config::default()
        });
        pinned.handle_input(Input::ColorScheme(ResolvedTheme::Light));
        assert_eq!(pinned.theme.border, Theme::dark().border);
    }

    #[test]
    fn pomodoro_keys_only_work_once_it_is_open() {
        let mut app = offline_app();
        press(&mut app, KeyCode::Char('s'), KeyModifiers::NONE);
        press(&mut app, KeyCode::Char('u'), KeyModifiers::NONE);
        assert!(!app.pomodoro.running);
        assert_eq!(app.pomodoro.work_mins, app.config.pomodoro);

        press(&mut app, KeyCode::Char('p'), KeyModifiers::NONE);
        assert!(app.pomodoro.active);
        press(&mut app, KeyCode::Char('u'), KeyModifiers::NONE);
        assert_eq!(app.pomodoro.work_mins, app.config.pomodoro + 1);
        press(&mut app, KeyCode::Char('s'), KeyModifiers::NONE);
        assert!(app.pomodoro.running);
        press(&mut app, KeyCode::Char('e'), KeyModifiers::NONE);
        assert!(!app.pomodoro.running);
        press(&mut app, KeyCode::Char('p'), KeyModifiers::NONE);
        assert!(!app.pomodoro.active);
    }

    #[test]
    fn pomodoro_alternates_work_and_break() {
        let mut app = offline_app();
        press(&mut app, KeyCode::Char('p'), KeyModifiers::NONE);
        press(&mut app, KeyCode::Char('s'), KeyModifiers::NONE);
        app.pomodoro.remaining = Duration::ZERO;
        app.pomodoro.tick();
        assert_eq!(app.pomodoro.phase, PomodoroPhase::Break);
        assert!(!app.pomodoro.running, "each phase waits for `s`");
        assert_eq!(
            app.pomodoro.remaining,
            Duration::from_secs(app.pomodoro.break_mins * 60)
        );

        // Paused: time doesn't move.
        let before = app.pomodoro.remaining;
        thread::sleep(Duration::from_millis(5));
        app.pomodoro.tick();
        assert_eq!(app.pomodoro.remaining, before);

        app.pomodoro.running = true;
        thread::sleep(Duration::from_millis(5));
        app.pomodoro.tick();
        assert!(app.pomodoro.remaining < before);

        app.pomodoro.remaining = Duration::ZERO;
        app.pomodoro.tick();
        assert_eq!(app.pomodoro.phase, PomodoroPhase::Work);
    }

    #[test]
    fn focus_views_and_scrolling() {
        let mut app = offline_app();
        press(&mut app, KeyCode::Char('h'), KeyModifiers::NONE);
        press(&mut app, KeyCode::Char('w'), KeyModifiers::NONE);
        assert!(!app.show_help, "opening a list closes help");
        assert_eq!(app.focus, FocusView::Week);
        assert!(!app.scroll_today_focus);

        press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
        press(&mut app, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!((app.scroll_today, app.scroll_week), (0, 2));
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        press(&mut app, KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(app.scroll_today, 0, "scrolling stops at the top");

        press(&mut app, KeyCode::Char('w'), KeyModifiers::NONE);
        assert_eq!(app.focus, FocusView::None);
        press(&mut app, KeyCode::Char('h'), KeyModifiers::NONE);
        assert_eq!(
            press(&mut app, KeyCode::Esc, KeyModifiers::NONE),
            AppAction::Continue
        );
        assert!(!app.show_help);
    }

    #[test]
    fn only_key_presses_and_known_ctrl_keys_count() {
        let mut app = offline_app();
        let mut release = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        release.kind = KeyEventKind::Release;
        assert_eq!(app.handle_key(release), AppAction::Continue);
        assert_eq!(
            press(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL),
            AppAction::Continue
        );
        assert_eq!(
            press(&mut app, KeyCode::Char('d'), KeyModifiers::CONTROL),
            AppAction::Quit
        );
    }

    fn snapshot(weather: Option<Result<WeatherSnapshot, String>>) -> Snapshot {
        Snapshot {
            commits: CommitStats::default(),
            messages: Vec::new(),
            weather,
            warnings: vec!["first".into(), "second".into()],
        }
    }

    #[test]
    fn a_failed_refresh_keeps_the_last_good_weather_and_messages() {
        let mut app = offline_app();
        wait_for_refresh(&mut app);
        let messages = app.messages.clone();
        let sunny = WeatherSnapshot {
            place: "London".into(),
            summary: "Clear sky".into(),
            temperature: "20°C".into(),
            high_low: String::new(),
            wind: String::new(),
        };
        app.apply(snapshot(Some(Ok(sunny))));
        app.apply(snapshot(Some(Err("offline".into()))));
        assert_eq!(
            app.weather.as_ref().map(|w| w.place.as_str()),
            Some("London")
        );
        assert_eq!(app.messages, messages);
        assert_eq!(app.warning.as_deref(), Some("first"));

        app.apply(snapshot(None));
        assert!(app.weather.is_none(), "location removed");
    }

    #[test]
    fn a_crashed_refresh_is_reported() {
        let mut app = offline_app();
        wait_for_refresh(&mut app);
        let (tx, rx) = mpsc::channel();
        drop(tx);
        app.pending = Some(rx);
        app.loading = true;
        app.poll_refresh();
        assert!(!app.loading);
        assert_eq!(app.warning.as_deref(), Some("refresh failed unexpectedly"));
    }

    #[test]
    fn feed_titles_go_in_the_animal_box() {
        let mut app = offline_app();
        wait_for_refresh(&mut app);
        app.messages[0].title = Some("A blog post".into());
        assert_eq!(app.animal_box_message(), "A blog post");
        app.messages.clear();
        assert_eq!(app.animal_box_message(), mock::DEFAULT_MESSAGE);
    }

    #[test]
    fn auto_refresh_waits_for_the_interval() {
        let mut app = offline_app();
        wait_for_refresh(&mut app);
        assert!(!app.should_auto_refresh());
        app.last_refresh =
            Instant::now() - Duration::from_secs(app.config.update_interval_mins * 60);
        assert!(app.should_auto_refresh());
    }

    #[test]
    fn a_cycles_animals() {
        let mut app = offline_app();
        let first = app.current_animal();
        press(&mut app, KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(app.current_animal(), first.next());
    }
}
