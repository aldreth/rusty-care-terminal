use chrono::{DateTime, Local};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{BarChart, Block, BorderType, Borders, Clear, Paragraph, Wrap};

use crate::app::{App, FocusView, PomodoroPhase};
use crate::theme::Theme;
use crate::ui::art;

pub fn draw(frame: &mut Frame, app: &App) {
    let theme = Theme::for_mode(app.config.theme);

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

    if app.focus != FocusView::None {
        draw_focus_view(frame, app, theme, root[0]);
        draw_status(frame, app, theme, root[1]);
    } else {
        draw_today(frame, app, theme, top[0]);
        draw_chart(frame, app, theme, top[1]);
        draw_weather(frame, app, theme, side[0]);
        draw_care_panel(frame, app, theme, 1, "💖", side[1]);
        draw_care_panel(frame, app, theme, 2, "💬", side[2]);
        draw_week(frame, app, theme, bottom[0]);
        draw_say_box(frame, app, theme, bottom[1]);
        draw_status(frame, app, theme, root[1]);
    }

    if app.show_help {
        draw_help_modal(frame, theme);
    }
}

fn border_block(title: &str, theme: Theme) -> Block<'_> {
    border_block_selected(title, theme, false)
}

fn border_block_selected(title: &str, theme: Theme, selected: bool) -> Block<'_> {
    let (border_style, border_type, title_style) = if selected {
        (
            Style::default().fg(theme.hash).add_modifier(Modifier::BOLD),
            BorderType::Double,
            Style::default().fg(theme.hash).add_modifier(Modifier::BOLD),
        )
    } else {
        (
            theme.border_style(),
            BorderType::Plain,
            theme.title_style().add_modifier(Modifier::BOLD),
        )
    };
    // Keep the title string identical when selected — only border type/style
    // change — so the emoji and label do not shift on Tab.
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(border_style)
        .style(theme.root_style());
    if !title.is_empty() {
        block = block.title(Span::styled(title, title_style));
    }
    block
}

fn draw_today(frame: &mut Frame, app: &App, theme: Theme, area: Rect) {
    let empty = if app.loading {
        "loading tiny commits…"
    } else {
        "tiny commit bot is looking for tiny commits!"
    };
    let selected = app.scroll_today_focus;
    let lines = commit_lines(&app.commits.today, empty, theme);
    let widget = Paragraph::new(lines)
        .block(border_block_selected("📝 Today", theme, selected))
        .style(theme.root_style())
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_today, 0));
    frame.render_widget(widget, area);
}

fn draw_week(frame: &mut Frame, app: &App, theme: Theme, area: Rect) {
    let empty = if app.loading {
        "loading week…"
    } else {
        "no commits this week yet — soft start is fine"
    };
    let selected = !app.scroll_today_focus;
    let lines = commit_lines(&app.commits.week, empty, theme);
    let widget = Paragraph::new(lines)
        .block(border_block_selected("📝 Week", theme, selected))
        .style(theme.root_style())
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_week, 0));
    frame.render_widget(widget, area);
}

/// Group by repo heading (magenta path), then `hash - subject (ago)` lines.
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

    // One heading per repo (order = first appearance in the newest-first list).
    let mut repo_order: Vec<&str> = Vec::new();
    for c in entries {
        if !repo_order.contains(&c.repo.as_str()) {
            repo_order.push(c.repo.as_str());
        }
    }

    let mut lines = Vec::new();
    let mut remaining = 60usize;
    for (i, repo) in repo_order.iter().enumerate() {
        if remaining == 0 {
            break;
        }
        if i > 0 {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(
            (*repo).to_string(),
            Style::default().fg(theme.hash).add_modifier(Modifier::BOLD),
        )));
        for c in entries.iter().filter(|c| c.repo.as_str() == *repo) {
            if remaining == 0 {
                break;
            }
            remaining -= 1;
            let ago = relative_time(c.when);
            let time_style = Style::default().fg(time_color(theme, c.when));
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{} ", short_hash(&c.hash)),
                    Style::default().fg(theme.hash).add_modifier(Modifier::BOLD),
                ),
                Span::styled("- ".to_string(), Style::default().fg(theme.muted)),
                Span::styled(
                    format!("{} ", truncate(&c.subject, 56)),
                    Style::default().fg(theme.subject),
                ),
                Span::styled(format!("({ago})"), time_style),
            ]));
        }
    }
    lines
}

fn short_hash(hash: &str) -> &str {
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
    let block = border_block("Commits", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let today = app.commits.today_count();
    let week = app.commits.week_count();
    let data = [("today", today), ("week", week)];

    // Fat bars that fill most of the pane, centered horizontally.
    let bar_width = ((inner.width.saturating_sub(6)) / 3).clamp(5, 14);
    let bar_gap = (bar_width / 2).clamp(2, 5);
    let chart_w = bar_width
        .saturating_mul(2)
        .saturating_add(bar_gap)
        .saturating_add(2)
        .min(inner.width);
    let chart_h = inner.height.saturating_sub(0).max(3);
    let chart_area = Rect {
        x: inner.x + inner.width.saturating_sub(chart_w) / 2,
        y: inner.y,
        width: chart_w,
        height: chart_h,
    };

    let chart = BarChart::default()
        .data(&data)
        .bar_width(bar_width)
        .bar_gap(bar_gap)
        .bar_style(Style::default().fg(theme.bar))
        .value_style(
            Style::default()
                .fg(theme.bar_value)
                .add_modifier(Modifier::BOLD),
        )
        .label_style(theme.muted_style());
    frame.render_widget(chart, chart_area);
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
                format!("{} · {}", w.temperature, w.summary),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                w.high_low.clone(),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(w.place.clone(), theme.muted_style())),
            Line::from(Span::styled(w.wind.clone(), theme.muted_style())),
        ]
    } else if app.config.weather.is_some() {
        vec![
            Line::from(Span::styled("weather unavailable", theme.muted_style())),
            Line::from(Span::styled(
                "Take a breath. Look out a window.",
                Style::default().fg(theme.fg),
            )),
        ]
    } else {
        vec![
            Line::from(Span::styled(
                "Set RCT_WEATHER or --weather",
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled("(city or lat,lon)", theme.muted_style())),
        ]
    };

    let widget = Paragraph::new(lines)
        .block(border_block("☀️", theme))
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
    let mut lines = Vec::new();
    match app.care.get(index) {
        Some(msg) if app.loading && msg.text.is_empty() => {
            lines.push(Line::from(Span::styled(
                "loading care…",
                theme.muted_style(),
            )));
        }
        Some(msg) => {
            if let Some(label) = friendly_care_label(&msg.source_label) {
                lines.push(Line::from(Span::styled(label, theme.accent_style())));
                lines.push(Line::from(""));
            }
            lines.push(Line::from(Span::styled(
                msg.text.clone(),
                Style::default().fg(theme.fg),
            )));
        }
        None if app.loading => {
            lines.push(Line::from(Span::styled(
                "loading care…",
                theme.muted_style(),
            )));
        }
        None => {
            lines.push(Line::from(Span::styled(
                "Hi! You're doing great!!!",
                Style::default().fg(theme.fg),
            )));
        }
    };

    let widget = Paragraph::new(lines)
        .block(border_block(title, theme))
        .style(theme.root_style())
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, area);
}

/// Keep `@user@host`; strip legacy `→ url` / `· cached` suffixes and bare URLs.
fn friendly_care_label(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let base = trimmed
        .split(" → ")
        .next()
        .unwrap_or(trimmed)
        .split(" · ")
        .next()
        .unwrap_or(trimmed)
        .trim();
    if base.is_empty() || base.starts_with("http://") || base.starts_with("https://") {
        return None;
    }
    Some(base.to_string())
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
        .and_then(|m| friendly_care_label(&m.source_label));

    let block = border_block("", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line<'static>> = Vec::new();
    if let Some(label) = source {
        lines.push(Line::from(Span::styled(label, theme.accent_style())));
        lines.push(Line::from(""));
    }
    let art_budget = inner.height.saturating_sub(lines.len() as u16) as usize;
    lines.extend(art::say_box_lines(
        &app.config.say_box,
        msg,
        inner.width as usize,
        art_budget.max(6),
        theme,
    ));
    let widget = Paragraph::new(lines)
        .style(theme.root_style())
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, inner);
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
            "s start/pause · e reset · u/U work± · b/B break± · p exit · h help",
            theme.muted_style(),
        )),
    ];
    let widget = Paragraph::new(lines)
        .block(border_block("🍅", theme))
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
        let short = truncate(err, 40);
        text = format!("{text} · {short}");
    }
    if app.loading {
        text = format!("loading… · {text}");
    }
    let widget = Paragraph::new(text).style(theme.muted_style());
    frame.render_widget(widget, area);
}

fn draw_focus_view(frame: &mut Frame, app: &App, theme: Theme, area: Rect) {
    let (title, lines, scroll) = match app.focus {
        FocusView::None => return,
        FocusView::Today => (
            "📝 Today · t/Esc back · select to copy",
            commit_lines(&app.commits.today, "no commits today", theme),
            app.scroll_today,
        ),
        FocusView::Week => (
            "📝 Week · w/Esc back · select to copy",
            commit_lines(&app.commits.week, "no commits this week", theme),
            app.scroll_week,
        ),
    };

    let widget = Paragraph::new(lines)
        .block(border_block_selected(title, theme, true))
        .style(theme.root_style())
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(widget, area);
}

fn draw_help_modal(frame: &mut Frame, theme: Theme) {
    let area = centered_rect(62, 18, frame.area());
    frame.render_widget(Clear, area);

    let lines = vec![
        Line::from(Span::styled(
            "help",
            theme.accent_style().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        help_line("q / Esc", "quit (closes help / focus first)", theme),
        help_line("h", "toggle this help", theme),
        help_line("r", "refresh git / care / weather", theme),
        help_line("t", "fullscreen Today commits (copy-friendly)", theme),
        help_line("w", "fullscreen Week commits (copy-friendly)", theme),
        help_line("Tab", "select Today vs Week for scrolling", theme),
        help_line("j / k", "scroll selected commit list", theme),
        help_line("p", "pomodoro in the say-box", theme),
        Line::from(""),
        Line::from(Span::styled(
            "Theme: RCT_THEME / --theme (dark|light|auto) — no key toggle.",
            theme.muted_style(),
        )),
        Line::from(Span::styled(
            "Say-box art: RCT_SAY_BOX=parrot|bunny|llama|cat|RANDOM|/path.png",
            theme.muted_style(),
        )),
    ];

    let widget = Paragraph::new(lines)
        .block(border_block("h", theme))
        .style(theme.root_style())
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}

fn help_line(key: &str, desc: &str, theme: Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {key:<10}"),
            Style::default().fg(theme.hash).add_modifier(Modifier::BOLD),
        ),
        Span::styled(desc.to_string(), Style::default().fg(theme.fg)),
    ])
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
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

    #[test]
    fn friendly_label_keeps_acct_drops_url() {
        assert_eq!(
            friendly_care_label("@selfcaregentle@hachyderm.io").as_deref(),
            Some("@selfcaregentle@hachyderm.io")
        );
        assert_eq!(
            friendly_care_label(
                "@GoatsLive@mastodon.social → https://mastodon.social/@GoatsLive.rss"
            )
            .as_deref(),
            Some("@GoatsLive@mastodon.social")
        );
        assert_eq!(friendly_care_label("https://hachyderm.io/@x.rss"), None);
    }

    #[test]
    fn commit_lines_group_by_repo() {
        let theme = Theme::dark();
        let now = Local::now();
        let entries = vec![
            crate::model::CommitEntry {
                repo: "~/Code/a".into(),
                hash: "aaaaaaa".into(),
                author: "me".into(),
                subject: "one".into(),
                when: now,
            },
            crate::model::CommitEntry {
                repo: "~/Code/a".into(),
                hash: "bbbbbbb".into(),
                author: "me".into(),
                subject: "two".into(),
                when: now,
            },
            crate::model::CommitEntry {
                repo: "~/Code/b".into(),
                hash: "ccccccc".into(),
                author: "me".into(),
                subject: "three".into(),
                when: now,
            },
        ];
        let lines = commit_lines(&entries, "empty", theme);
        let text: Vec<String> = lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect();
        assert_eq!(text[0], "~/Code/a");
        assert!(text[1].starts_with("aaaaaaa"));
        assert!(!text[1].contains("~/Code"));
        assert!(text.iter().any(|l| l == "~/Code/b"));
        let b_idx = text.iter().position(|l| l == "~/Code/b").unwrap();
        assert!(text[b_idx + 1].starts_with("ccccccc"));
    }
}
