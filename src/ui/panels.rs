use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{BarChart, Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::ui::art;

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
    draw_weather_stub(frame, app, side[0]);
    draw_care_panel(frame, app, 1, "💖 care", side[1]);
    draw_care_panel(frame, app, 2, "💬 care", side[2]);
    draw_week(frame, app, bottom[0]);
    draw_say_box(frame, app, bottom[1]);
    draw_status(frame, app, root[1]);
}

fn border_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
}

fn draw_today(frame: &mut Frame, app: &App, area: Rect) {
    let lines = commit_lines(&app.commits.today, "tiny commit bot is looking for tiny commits!");
    let widget = Paragraph::new(lines)
        .block(border_block("📝 last 24 hours"))
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, area);
}

fn draw_week(frame: &mut Frame, app: &App, area: Rect) {
    let lines = commit_lines(&app.commits.week, "no commits this week yet — soft start is fine");
    let widget = Paragraph::new(lines)
        .block(border_block("📝 last week"))
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, area);
}

fn commit_lines(entries: &[crate::model::CommitEntry], empty: &str) -> Vec<Line<'static>> {
    if entries.is_empty() {
        return vec![Line::from(Span::styled(
            empty.to_string(),
            Style::default().fg(Color::DarkGray),
        ))];
    }
    entries
        .iter()
        .take(40)
        .map(|c| {
            let text = format!(
                "[{}] {} — {}",
                c.repo,
                c.hash,
                truncate(&c.subject, 60)
            );
            Line::from(Span::raw(text))
        })
        .collect()
}

fn draw_chart(frame: &mut Frame, app: &App, area: Rect) {
    let today = app.commits.today_count().max(0);
    let week = app.commits.week_count().max(0);
    // BarChart needs non-zero visual height; show at least a stub bar of 0.
    let data = [("24h", today), ("7d", week)];
    let chart = BarChart::default()
        .block(border_block("commits"))
        .data(&data)
        .bar_width(5)
        .bar_gap(2)
        .bar_style(Style::default().fg(Color::Magenta))
        .value_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .label_style(Style::default().fg(Color::Cyan));
    frame.render_widget(chart, area);
}

fn draw_weather_stub(frame: &mut Frame, app: &App, area: Rect) {
    let place = app
        .config
        .weather
        .as_deref()
        .unwrap_or("somewhere cozy");
    let text = format!(
        "{place}\n\n(weather arrives in a later milestone)\n\nTake a breath. Look out a window."
    );
    let widget = Paragraph::new(text)
        .block(border_block("🌤 weather"))
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, area);
}

fn draw_care_panel(frame: &mut Frame, app: &App, index: usize, title: &str, area: Rect) {
    let (label, body) = match app.care.get(index) {
        Some(msg) => {
            let tag = if msg.from_mock { "mock" } else { "live" };
            (
                format!("{} · {tag}", msg.source_label),
                msg.text.clone(),
            )
        }
        None => ("—".into(), "Hi! You're doing great!!!".into()),
    };
    let lines = vec![
        Line::from(Span::styled(label, Style::default().fg(Color::Yellow))),
        Line::from(""),
        Line::from(body),
    ];
    let widget = Paragraph::new(lines)
        .block(border_block(title))
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, area);
}

fn draw_say_box(frame: &mut Frame, app: &App, area: Rect) {
    let msg = app
        .care
        .first()
        .map(|m| m.text.as_str())
        .unwrap_or("Hi! You're doing great!!!");
    let source = app
        .care
        .first()
        .map(|m| {
            let tag = if m.from_mock { "mock RSS path" } else { "rss" };
            format!("{} · {tag}", m.source_label)
        })
        .unwrap_or_else(|| "care".into());

    let art_text = art::say_box(&app.config.say_box, msg);
    let mut lines: Vec<Line<'static>> = vec![
        Line::from(Span::styled(source, Style::default().fg(Color::Yellow))),
        Line::from(""),
    ];
    // One ratatui Line per ASCII row — a single Line collapses newlines into mush.
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

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let mut text = app.status.clone();
    if let Some(err) = &app.error {
        text = format!("error: {err} · {text}");
    }
    if app.loading {
        text = format!("loading… · {text}");
    }
    let widget = Paragraph::new(text).style(Style::default().fg(Color::Gray));
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
