mod app;
mod config;
mod error;
mod model;
mod services;
mod theme;
mod ui;

use std::io::{self, stdout};
use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use app::{App, AppAction};
use config::{CliOverrides, Config};
use theme::ThemeMode;

#[derive(Debug, Parser)]
#[command(
    name = "rusty-care-terminal",
    about = "A gentle Rust care terminal inspired by tiny-care-terminal (Mastodon RSS, no Twitter)."
)]
struct Cli {
    /// Comma-separated repo roots to scan (overrides RCT_REPOS).
    #[arg(long, env = "RCT_REPOS")]
    repos: Option<String>,

    /// Max walk depth under each repo root (overrides RCT_REPOS_DEPTH).
    #[arg(long, env = "RCT_REPOS_DEPTH")]
    repos_depth: Option<usize>,

    /// Comma-separated @user@instance care sources (overrides RCT_BOTS).
    #[arg(long, env = "RCT_BOTS")]
    bots: Option<String>,

    /// Comma-separated explicit RSS/Atom URLs (overrides RCT_BOT_FEEDS).
    #[arg(long, env = "RCT_BOT_FEEDS")]
    bot_feeds: Option<String>,

    /// Starting say-box animal: parrot | cat | bunny | llama | random (`a` cycles in-app).
    #[arg(long, env = "RCT_SAY_BOX")]
    say_box: Option<String>,

    /// Your identity for sole/joint authorship (comma-separated name/email).
    #[arg(long, env = "RCT_GIT_AUTHOR")]
    git_author: Option<String>,

    /// Auto-refresh interval in minutes (overrides RCT_UPDATE_INTERVAL).
    #[arg(long, env = "RCT_UPDATE_INTERVAL")]
    update_interval: Option<u64>,

    /// Weather location: city name or lat,lon (overrides RCT_WEATHER).
    #[arg(long, env = "RCT_WEATHER")]
    weather: Option<String>,

    /// Use Celsius (default true). Set false / RCT_CELSIUS=false for Fahrenheit.
    #[arg(long, env = "RCT_CELSIUS")]
    celsius: Option<bool>,

    /// Pomodoro work minutes (overrides RCT_POMODORO).
    #[arg(long, env = "RCT_POMODORO")]
    pomodoro: Option<u64>,

    /// Pomodoro break minutes (overrides RCT_BREAK).
    #[arg(long, env = "RCT_BREAK")]
    break_minutes: Option<u64>,

    /// Force mock care messages (skip live Mastodon RSS).
    #[arg(long, env = "RCT_MOCK", default_value_t = false)]
    mock: bool,

    /// UI theme: auto (default) | dark | light. Overrides RCT_THEME (no key toggle).
    #[arg(long, env = "RCT_THEME")]
    theme: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let theme = match cli.theme.as_deref() {
        Some(raw) => Some(ThemeMode::parse(raw).ok_or_else(|| {
            anyhow::anyhow!("invalid --theme / RCT_THEME: {raw} (dark|light|auto)")
        })?),
        None => None,
    };

    let config = Config::load(CliOverrides {
        repos: cli.repos,
        repos_depth: cli.repos_depth,
        bots: cli.bots,
        bot_feeds: cli.bot_feeds,
        say_box: cli.say_box,
        git_author: cli.git_author,
        update_interval: cli.update_interval,
        weather: cli.weather,
        celsius: cli.celsius,
        pomodoro: cli.pomodoro,
        break_minutes: cli.break_minutes,
        mock: cli.mock,
        theme,
    })
    .context("loading config")?;

    let mut terminal = setup_terminal(&config.terminal_title)?;
    let result = run(&mut terminal, config);
    restore_terminal(&mut terminal)?;
    result
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: Config,
) -> anyhow::Result<()> {
    let mut app = App::new(config);

    loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;
        match app.poll_events(Duration::from_millis(200))? {
            AppAction::Quit => break,
            AppAction::Continue => {}
        }
    }
    Ok(())
}

fn setup_terminal(title: &str) -> anyhow::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut out = stdout();
    // No mouse capture — lets the terminal select/copy text (hashes, etc.).
    execute!(out, EnterAlternateScreen)?;
    let _ = execute!(out, crossterm::terminal::SetTitle(title));
    let backend = CrosstermBackend::new(out);
    Terminal::new(backend).context("creating terminal")
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
