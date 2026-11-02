mod app;
mod config;
mod error;
mod model;
mod services;
mod ui;

use std::io::{self, stdout};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{App, AppAction};
use config::Config;

#[derive(Debug, Parser)]
#[command(
    name = "rusty-care-terminal",
    about = "A gentle Rust care terminal inspired by tiny-care-terminal (Mastodon RSS, no Twitter)."
)]
struct Cli {
    /// Path to TOML config (overrides RCT_CONFIG / discovery).
    #[arg(long, env = "RCT_CONFIG")]
    config: Option<PathBuf>,

    /// Force mock care messages (skip live Mastodon RSS).
    #[arg(long, env = "RCT_MOCK", default_value_t = false)]
    mock: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = Config::load(cli.config, cli.mock).context("loading config")?;

    let mut terminal = setup_terminal(&config.terminal_title)?;
    let result = run(&mut terminal, config);
    restore_terminal(&mut terminal)?;
    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, config: Config) -> anyhow::Result<()> {
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
    execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    // Best-effort title; ignore failures on limited terminals.
    let _ = execute!(out, crossterm::terminal::SetTitle(title));
    let backend = CrosstermBackend::new(out);
    Terminal::new(backend).context("creating terminal")
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
