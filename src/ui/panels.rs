use chrono::{DateTime, Local};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{BarChart, Block, Borders, Clear, Paragraph, Wrap};

use crate::app::{App, PomodoroPhase};
use crate::theme::Theme;
use crate::ui::art;

pub fn draw(frame: &mut Frame, app: &App) {
    let theme = Theme::for_mode(app.config.theme);
    frame.render_widget(Clear, frame.area());
    frame.render_widget(Block::default().style(theme.root_style()), frame.area());

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(frame.area());

    let body = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(root[0]);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(48),
            Constraint::Percentage(18),
            Constraint::Percentage(34),
        ])
        .split(body[0]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
        .split(body[1]);

    let side = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(top[2]);

    draw_today(frame, app, theme, top[0]);
    draw_chart(frame, app, theme, top[1]);
    draw_weather(frame, app, theme, side[0]);
    draw_care_panel(frame, app, theme, 1, "💖 care", side[1]);
    draw_care_panel(frame, app, theme, 2, "💬 care", side[2]);
    draw_week(frame, app, theme, bottom[0]);
    draw_say_box(frame, app, theme, bottom[1]);
    draw_status(frame, app, theme, root[1]);
}

fn border_block(title: &str, theme: Theme) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .style(theme.root_style())
        .title(Span::styled(
            title,
            theme.title_style().add_modifier(Modifier::BOLD),
        ))
}

fn draw_today(frame: &mut Frame, app: &App, theme: Theme, area: Rect) {
    let empty = if app.loading {
        "loading tiny commits…"
    } else {
        "tiny commit bot is looking for tiny commits!"
    };
    let title = if app.scroll_today_focus {
        "📝 Today · j/k"
    } else {
        "📝 Today"
    };
    let lines = commit_lines(&app.commits.today, empty, theme);
    let widget = Paragraph::new(lines)
        .block(border_block(title, theme))
        .style(theme.root_style())
        .wrap(Wrap { trim: true })
        .scroll((app.scroll_today, 0));
    frame.render_widget(widget, area);
}

fn draw_week(frame: &mut Frame, app: &App, theme: Theme, area: Rect) {
    let empty = if app.loading {
        "loading week…"
    } else {
        "no commits this week yet — soft start is fine"
    };
    let title = if !app.scroll_today_focus {
        "📝 Week · j/k"
    } else {
        "📝 Week"
    };
    let lines = commit_lines(&app.commits.week, empty, theme);
    let widget = Paragraph::new(lines)
        .block(border_block(title, theme))
        .style(theme.root_style())
        .wrap(Wrap { trim: true })
        .scroll((app.scroll_week, 0));
    frame.render_widget(widget, area);
}

fn commit_lines(
    entries: &[crate::model::CommitEntry],
    empty: &str,
    theme: Theme,
) -> Vec<Line<'static>> {
    if entries.is_empty() {
        return vec![Line::from(Span::styled(
            empty.to_string(),
            theme.muted_style(),
        ))];
    }
    entries
        .iter()
        .take(40)
        .map(|c| {
            let ago = relative_time(c.when);
            let time_style = Style::default().fg(time_color(theme, c.when));
            Line::from(vec![
                Span::styled(format!("{} ", c.repo), Style::default().fg(theme.hash)),
                Span::styled(
                    format!("{} ", short_hash(&c.hash)),
                    Style::default().fg(theme.hash).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} ", truncate(&c.subject, 52)),
                    Style::default().fg(theme.subject),
                ),
                Span::styled(format!("({ago})"), time_style),
            ])
        })
        .collect()
}

fn short_hash(hash: &str) -> &str {
    // Git hashes are ASCII hex; byte slicing is fine.
    &hash[..hash.len().min(7)]
}

fn relative_time(when: DateTime<Local>) -> String {
    let secs = (Local::now() - when).num_seconds().max(0);
    if secs < 60 {
        format!("{secs} seconds ago")
    } else if secs < 3600 {
        let m = secs / 60;
        format!("{} minute{} ago", m, if m == 1 { "" } else { "s" })
    } else if secs < 86_400 {
        let h = secs / 3600;
        format!("{} hour{} ago", h, if h == 1 { "" } else { "s" })
    } else {
        let d = secs / 86_400;
        format!("{} day{} ago", d, if d == 1 { "" } else { "s" })
    }
}

fn time_color(theme: Theme, when: DateTime<Local>) -> ratatui::style::Color {
    let secs = (Local::now() - when).num_seconds().max(0);
    if secs < 3600 {
        theme.time_fresh
    } else if secs < 86_400 {
        theme.time_warm
    } else {
        theme.muted
    }
}

fn draw_chart(frame: &mut Frame, app: &App, theme: Theme, area: Rect) {
    let today = app.commits.today_count();
    let week = app.commits.week_count();
    let data = [("today", today), ("week", week)];
    let chart = BarChart::default()
        .block(border_block("Commits", theme))
        .data(&data)
        .bar_width(5)
        .bar_gap(2)
        .bar_style(Style::default().fg(theme.bar))
        .value_style(
            Style::default()
                .fg(theme.bar_value)
                .add_modifier(Modifier::BOLD),
        )
        .label_style(theme.muted_style());
    frame.render_widget(chart, area);
}

fn draw_weather(frame: &mut Frame, app: &App, theme: Theme, area: Rect) {
    let lines = if app.loading && app.weather.is_none() {
        vec![Line::from(Span::styled(
            "loading weather…",
            theme.muted_style(),
        ))]
    } else if let Some(w) = &app.weather {
        vec![
            Line::from(Span::styled(
                w.place.clone(),
                theme.accent_style().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("{} · {}", w.temperature, w.summary),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                w.high_low.clone(),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(w.wind.clone(), theme.muted_style())),
            Line::from(Span::styled(
                format!("updated {}", w.fetched_at.format("%H:%M")),
                theme.muted_style(),
            )),
        ]
    } else if app.config.weather.is_some() {
        vec![
            Line::from(Span::styled("weather unavailable", theme.muted_style())),
            Line::from(""),
            Line::from(Span::styled(
                "Take a breath. Look out a window.",
                Style::default().fg(theme.fg),
            )),
        ]
    } else {
        vec![
            Line::from(Span::styled("somewhere cozy", theme.accent_style())),
            Line::from(""),
            Line::from(Span::styled(
                "Set RCT_WEATHER or --weather",
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled("(city name or lat,lon)", theme.muted_style())),
            Line::from(""),
            Line::from(Span::styled(
                "Take a breath. Look out a window.",
                Style::default().fg(theme.fg),
            )),
        ]
    };

    let widget = Paragraph::new(lines)
        .block(border_block("☀️ weather", theme))
        .style(theme.root_style())
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, area);
}

fn draw_care_panel(
    frame: &mut Frame,
    app: &App,
    theme: Theme,
    index: usize,
    title: &str,
    area: Rect,
) {
    let (label, body) = match app.care.get(index) {
        Some(msg) if app.loading && msg.text.is_empty() => ("…".into(), "loading care…".into()),
        Some(msg) => {
            let tag = if msg.from_mock { "mock" } else { "live" };
            (format!("{} · {tag}", msg.source_label), msg.text.clone())
        }
        None if app.loading => ("…".into(), "loading care…".into()),
        None => ("—".into(), "Hi! You're doing great!!!".into()),
    };
    let lines = vec![
        Line::from(Span::styled(label, theme.accent_style())),
        Line::from(""),
        Line::from(Span::styled(body, Style::default().fg(theme.fg))),
    ];
    let widget = Paragraph::new(lines)
        .block(border_block(title, theme))
        .style(theme.root_style())
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, area);
}

fn draw_say_box(frame: &mut Frame, app: &App, theme: Theme, area: Rect) {
    if app.pomodoro.active {
        draw_pomodoro(frame, app, theme, area);
        return;
    }

    let msg = app
        .care
        .first()
        .map(|m| m.text.as_str())
        .unwrap_or("Hi! You're doing great!!!");
    let source = app
        .care
        .first()
        .map(|m| {
            let tag = if m.from_mock { "mock" } else { "rss" };
            format!("{} · {tag}", m.source_label)
        })
        .unwrap_or_else(|| "care".into());

    let max_width = area.width.saturating_sub(2) as usize;
    let art_text = art::say_box(&app.config.say_box, msg, max_width);
    let mut lines: Vec<Line<'static>> = vec![
        Line::from(Span::styled(source, theme.accent_style())),
        Line::from(""),
    ];
    lines.extend(art_text.lines().map(|row| {
        Line::from(Span::styled(
            row.to_string(),
            Style::default().fg(theme.bar),
        ))
    }));
    let widget = Paragraph::new(lines)
        .block(border_block("✨ say", theme))
        .style(theme.root_style())
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}

fn draw_pomodoro(frame: &mut Frame, app: &App, theme: Theme, area: Rect) {
    let p = &app.pomodoro;
    let state = if p.running { "running" } else { "paused" };
    let phase = match p.phase {
        PomodoroPhase::Work => "🍅 focus",
        PomodoroPhase::Break => "🍵 break",
    };
    let lines = vec![
        Line::from(Span::styled(
            format!("{phase} · {state}"),
            theme.accent_style().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            p.mmss(),
            Style::default()
                .fg(theme.time_fresh)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("work {}m · break {}m", p.work_mins, p.break_mins),
            Style::default().fg(theme.fg),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "s start/pause · e reset · u/U work± · b/B break± · p exit",
            theme.muted_style(),
        )),
    ];
    let widget = Paragraph::new(lines)
        .block(border_block("🍅 pomodoro", theme))
        .style(theme.root_style())
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, area);
}

fn draw_status(frame: &mut Frame, app: &App, theme: Theme, area: Rect) {
    let mut text = app.status.clone();
    if app.pomodoro.active {
        let run = if app.pomodoro.running { "▶" } else { "⏸" };
        text = format!("{text} · {run} {}", app.pomodoro.mmss());
    }
    if let Some(err) = &app.error {
        let short = truncate(err, 48);
        if !text.contains(&short) {
            text = format!("{text} · {short}");
        }
    }
    if app.loading {
        text = format!("loading… · {text}");
    }
    let widget = Paragraph::new(text).style(theme.muted_style().bg(theme.bg));
    frame.render_widget(widget, area);
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{t}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_time_plurals() {
        let now = Local::now();
        assert!(relative_time(now).contains("second"));
        assert!(relative_time(now - chrono::Duration::minutes(2)).contains("minutes"));
        assert!(relative_time(now - chrono::Duration::hours(3)).contains("hours"));
    }

    #[test]
    fn time_color_thresholds() {
        let theme = Theme::dark();
        let now = Local::now();
        assert_eq!(time_color(theme, now), theme.time_fresh);
        assert_eq!(
            time_color(theme, now - chrono::Duration::hours(5)),
            theme.time_warm
        );
        assert_eq!(
            time_color(theme, now - chrono::Duration::days(2)),
            theme.muted
        );
    }
}
