use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{BarChart, Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, PomodoroPhase};
use crate::ui::art;

// Soft care-terminal palette (avoid harsh pure cyan/magenta).
const BORDER: Color = Color::Rgb(110, 170, 175);
const TITLE: Color = Color::Rgb(140, 200, 195);
const ACCENT: Color = Color::Rgb(210, 180, 120);
const MUTED: Color = Color::Rgb(120, 130, 135);
const BAR: Color = Color::Rgb(180, 130, 160);

pub fn draw(frame: &mut Frame, app: &App) {
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

    draw_today(frame, app, top[0]);
    draw_chart(frame, app, top[1]);
    draw_weather(frame, app, side[0]);
    draw_care_panel(frame, app, 1, "💖 care", side[1]);
    draw_care_panel(frame, app, 2, "💬 care", side[2]);
    draw_week(frame, app, bottom[0]);
    draw_say_box(frame, app, bottom[1]);
    draw_status(frame, app, root[1]);
}

fn border_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .title(Span::styled(
            title,
            Style::default().fg(TITLE).add_modifier(Modifier::BOLD),
        ))
}

fn draw_today(frame: &mut Frame, app: &App, area: Rect) {
    let empty = if app.loading {
        "loading tiny commits…"
    } else {
        "tiny commit bot is looking for tiny commits!"
    };
    let lines = commit_lines(&app.commits.today, empty);
    let widget = Paragraph::new(lines)
        .block(border_block("📝 last 24 hours"))
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, area);
}

fn draw_week(frame: &mut Frame, app: &App, area: Rect) {
    let empty = if app.loading {
        "loading week…"
    } else {
        "no commits this week yet — soft start is fine"
    };
    let lines = commit_lines(&app.commits.week, empty);
    let widget = Paragraph::new(lines)
        .block(border_block("📝 last week"))
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, area);
}

fn commit_lines(entries: &[crate::model::CommitEntry], empty: &str) -> Vec<Line<'static>> {
    if entries.is_empty() {
        return vec![Line::from(Span::styled(
            empty.to_string(),
            Style::default().fg(MUTED),
        ))];
    }
    entries
        .iter()
        .take(40)
        .map(|c| {
            let text = format!("[{}] {} — {}", c.repo, c.hash, truncate(&c.subject, 60));
            Line::from(Span::raw(text))
        })
        .collect()
}

fn draw_chart(frame: &mut Frame, app: &App, area: Rect) {
    let today = app.commits.today_count();
    let week = app.commits.week_count();
    let data = [("24h", today), ("7d", week)];
    let chart = BarChart::default()
        .block(border_block("commits"))
        .data(&data)
        .bar_width(5)
        .bar_gap(2)
        .bar_style(Style::default().fg(BAR))
        .value_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .label_style(Style::default().fg(TITLE));
    frame.render_widget(chart, area);
}

fn draw_weather(frame: &mut Frame, app: &App, area: Rect) {
    let lines = if app.loading && app.weather.is_none() {
        vec![Line::from(Span::styled(
            "loading weather…",
            Style::default().fg(MUTED),
        ))]
    } else if let Some(w) = &app.weather {
        vec![
            Line::from(Span::styled(
                w.place.clone(),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(format!("{} · {}", w.temperature, w.summary)),
            Line::from(w.high_low.clone()),
            Line::from(Span::styled(w.wind.clone(), Style::default().fg(MUTED))),
            Line::from(Span::styled(
                format!("updated {}", w.fetched_at.format("%H:%M")),
                Style::default().fg(MUTED),
            )),
        ]
    } else if app.config.weather.is_some() {
        vec![
            Line::from(Span::styled(
                "weather unavailable",
                Style::default().fg(MUTED),
            )),
            Line::from(""),
            Line::from("Take a breath. Look out a window."),
        ]
    } else {
        vec![
            Line::from(Span::styled(
                "somewhere cozy",
                Style::default().fg(ACCENT),
            )),
            Line::from(""),
            Line::from("Set RCT_WEATHER or --weather"),
            Line::from("(city name or lat,lon)"),
            Line::from(""),
            Line::from("Take a breath. Look out a window."),
        ]
    };

    let widget = Paragraph::new(lines)
        .block(border_block("🌤 weather"))
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, area);
}

fn draw_care_panel(frame: &mut Frame, app: &App, index: usize, title: &str, area: Rect) {
    let (label, body) = match app.care.get(index) {
        Some(msg) if app.loading && msg.text.is_empty() => {
            ("…".into(), "loading care…".into())
        }
        Some(msg) => {
            let tag = if msg.from_mock { "mock" } else { "live" };
            (
                format!("{} · {tag}", msg.source_label),
                msg.text.clone(),
            )
        }
        None if app.loading => ("…".into(), "loading care…".into()),
        None => ("—".into(), "Hi! You're doing great!!!".into()),
    };
    let lines = vec![
        Line::from(Span::styled(label, Style::default().fg(ACCENT))),
        Line::from(""),
        Line::from(body),
    ];
    let widget = Paragraph::new(lines)
        .block(border_block(title))
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, area);
}

fn draw_say_box(frame: &mut Frame, app: &App, area: Rect) {
    if app.pomodoro.active {
        draw_pomodoro(frame, app, area);
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
        Line::from(Span::styled(source, Style::default().fg(ACCENT))),
        Line::from(""),
    ];
    lines.extend(
        art_text
            .lines()
            .map(|row| Line::from(Span::raw(row.to_string()))),
    );
    let widget = Paragraph::new(lines)
        .block(border_block("✨ say"))
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}

fn draw_pomodoro(frame: &mut Frame, app: &App, area: Rect) {
    let p = &app.pomodoro;
    let state = if p.running { "running" } else { "paused" };
    let phase = match p.phase {
        PomodoroPhase::Work => "🍅 focus",
        PomodoroPhase::Break => "🍵 break",
    };
    let lines = vec![
        Line::from(Span::styled(
            format!("{phase} · {state}"),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            p.mmss(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!(
            "work {}m · break {}m",
            p.work_mins, p.break_mins
        )),
        Line::from(""),
        Line::from(Span::styled(
            "s start/pause · e reset · u/U work± · b/B break± · p exit",
            Style::default().fg(MUTED),
        )),
    ];
    let widget = Paragraph::new(lines)
        .block(border_block("🍅 pomodoro"))
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, area);
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
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
    let widget = Paragraph::new(text).style(Style::default().fg(MUTED));
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
