mod app;
mod config;
mod error;
mod input;
mod model;
mod services;
mod theme;
mod ui;

use std::io::{self, IsTerminal, Write};
use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use clap::builder::BoolishValueParser;
use ratatui::DefaultTerminal;

use app::{App, AppAction};
use config::{CliOverrides, Config};
use input::InputSource;
use theme::ThemeMode;

/// Every flag can also be set with the `RCT_*` variable shown in `--help`.
#[derive(Debug, Parser)]
#[command(
    name = "rusty-care-terminal",
    version,
    about = "A gentle Rust care terminal inspired by tiny-care-terminal (Mastodon RSS, no Twitter).",
    after_help = concat!(
        "Configuration guide and bots to try: ",
        env!("CARGO_PKG_REPOSITORY"),
        "/blob/main/CONFIGURATION.md"
    )
)]
struct Cli {
    /// Comma-separated repo roots to scan for your commits.
    #[arg(long, env = "RCT_REPOS")]
    repos: Option<String>,

    /// How deep to look for repos under each root (1–5, default 1).
    #[arg(long, env = "RCT_REPOS_DEPTH")]
    repos_depth: Option<usize>,

    /// Comma-separated bots as user@instance or feed URLs (the first feeds the animal box).
    #[arg(long, env = "RCT_BOTS")]
    bots: Option<String>,

    /// Starting animal: parrot | cat | bunny | llama | random (default: parrot).
    #[arg(long, env = "RCT_ANIMAL")]
    animal: Option<String>,

    /// Your commit emails, comma-separated (default: each repo's user.email).
    #[arg(long, env = "RCT_GIT_AUTHOR_EMAILS")]
    git_author_emails: Option<String>,

    /// Auto-refresh interval in minutes (default 20).
    #[arg(long, env = "RCT_UPDATE_INTERVAL")]
    update_interval: Option<u64>,

    /// Weather location: city name or lat,lon.
    #[arg(long, env = "RCT_WEATHER")]
    weather: Option<String>,

    /// Celsius (default true); `--celsius false` for Fahrenheit.
    #[arg(long, env = "RCT_CELSIUS", value_parser = BoolishValueParser::new())]
    celsius: Option<bool>,

    /// Window title (default "Rusty care terminal"); empty or `false` leaves it alone.
    #[arg(long, env = "RCT_TERMINAL_TITLE")]
    terminal_title: Option<String>,

    /// Pomodoro focus minutes (1–120, default 25).
    #[arg(long, env = "RCT_POMODORO")]
    pomodoro: Option<u64>,

    /// Pomodoro break minutes (1–60, default 5).
    #[arg(long, env = "RCT_BREAK")]
    break_minutes: Option<u64>,

    /// Offline messages instead of live bot feeds.
    #[arg(long, env = "RCT_MOCK", value_parser = BoolishValueParser::new())]
    mock: bool,

    /// Colour theme: auto (default) | dark | light.
    #[arg(long, env = "RCT_THEME")]
    theme: Option<String>,
}

impl From<Cli> for CliOverrides {
    fn from(cli: Cli) -> Self {
        Self {
            repos: cli.repos,
            repos_depth: cli.repos_depth,
            bots: cli.bots,
            animal: cli.animal,
            git_author_emails: cli.git_author_emails,
            update_interval: cli.update_interval,
            weather: cli.weather,
            celsius: cli.celsius,
            terminal_title: cli.terminal_title,
            pomodoro: cli.pomodoro,
            break_minutes: cli.break_minutes,
            mock: cli.mock,
            theme: cli.theme,
        }
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config =
        Config::load(cli.into(), |key| std::env::var(key).ok()).context("loading config")?;

    if !io::stdout().is_terminal() || !io::stdin().is_terminal() {
        anyhow::bail!("rusty-care-terminal needs an interactive terminal");
    }

    // Installs a panic hook that restores the terminal before the message prints.
    let mut terminal = ratatui::try_init().context("setting up the terminal")?;
    let title = config.terminal_title.clone();
    if let Some(title) = &title {
        push_title(title);
    }
    let follow_terminal_scheme = cfg!(unix) && config.theme == ThemeMode::Auto;
    if follow_terminal_scheme {
        subscribe_color_scheme();
    }
    let result = run(&mut terminal, config);
    if follow_terminal_scheme {
        write_escape(input::UNSUBSCRIBE_COLOR_SCHEME);
    }
    if title.is_some() {
        pop_title();
    }
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal, config: Config) -> anyhow::Result<()> {
    let input = InputSource::start().context("reading terminal input")?;
    let mut app = App::new(config);
    loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;
        if app.poll_events(&input, Duration::from_millis(200))? == AppAction::Quit {
            return Ok(());
        }
    }
}

/// Asks the terminal (Ghostty, kitty, …) to report dark/light switches. The
/// subscription is dropped even on a panic, or the shell would receive the reports.
fn subscribe_color_scheme() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        write_escape(input::UNSUBSCRIBE_COLOR_SCHEME);
        previous(info);
    }));
    write_escape(input::SUBSCRIBE_COLOR_SCHEME);
}

fn write_escape(sequence: &str) {
    let mut out = io::stdout();
    let _ = out.write_all(sequence.as_bytes());
    let _ = out.flush();
}

/// Saves the current title on the xterm title stack, then sets ours.
/// Terminals without a title stack ignore the save/restore sequences.
fn push_title(title: &str) {
    let mut out = io::stdout();
    let _ = write!(out, "\x1b[22;0t");
    let _ = crossterm::execute!(out, crossterm::terminal::SetTitle(title));
}

fn pop_title() {
    let mut out = io::stdout();
    let _ = write!(out, "\x1b[23;0t");
    let _ = out.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn boolish_flags_accept_common_spellings() {
        let cli = Cli::try_parse_from(["rct", "--celsius", "no", "--mock"]).unwrap();
        assert_eq!(cli.celsius, Some(false));
        assert!(cli.mock);
    }
}
