use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::model::Animal;
use crate::theme::Theme;
use crate::ui::sprites;

const MARGIN: usize = 1;
const TAIL_ROWS: usize = 2;
const MAX_BUBBLE_TEXT: usize = 56;

/// Speech bubble with a `\` tail landing on a pixel animal's head.
///
/// Lines are pre-laid-out for `max_width` × `max_height` and must not be re-wrapped.
pub fn animal_box_lines(
    animal: Animal,
    message: &str,
    max_width: usize,
    max_height: usize,
    theme: Theme,
) -> Vec<Line<'static>> {
    let style = Style::default().fg(theme.fg);
    let text_width = max_width
        .saturating_sub(MARGIN + 4)
        .clamp(1, MAX_BUBBLE_TEXT);
    let wrapped = wrap_text(message, text_width);
    let bubble_rows = wrapped.len() + 2;

    let mut lines = Vec::new();
    let art_rows = max_height.saturating_sub(bubble_rows + TAIL_ROWS);
    let art_cols = max_width.saturating_sub(MARGIN);

    if let Some(placed) = sprites::fit(&sprites::creature(animal), art_cols, art_rows) {
        // Tail cells sit at `tail` and `tail + 1`; the head's top-left
        // outline pixel is one cell further right, so the diagonal meets it.
        let min_tail = MARGIN + 2;
        let mut left = (MARGIN + 1).max((min_tail + 2).saturating_sub(placed.anchor_col));
        if left + placed.cols > max_width {
            left = max_width.saturating_sub(placed.cols);
        }
        let tail = (left + placed.anchor_col).saturating_sub(2);
        let min_inner = (tail + 3).saturating_sub(MARGIN + 4);
        push_bubble(&mut lines, &wrapped, min_inner, style);
        lines.push(Line::from(Span::styled(
            format!("{}\\", " ".repeat(tail)),
            style,
        )));
        lines.push(Line::from(Span::styled(
            format!("{}\\", " ".repeat(tail + 1)),
            style,
        )));
        let pad = " ".repeat(left);
        for line in placed.lines {
            let mut spans = vec![Span::raw(pad.clone())];
            spans.extend(line.spans);
            lines.push(Line::from(spans));
        }
    } else {
        push_bubble(&mut lines, &wrapped, 0, style);
    }

    lines.truncate(max_height.max(1));
    lines
}

fn push_bubble(lines: &mut Vec<Line<'static>>, wrapped: &[String], min_inner: usize, style: Style) {
    let width = wrapped
        .iter()
        .map(|l| l.width())
        .max()
        .unwrap_or(0)
        .max(min_inner);
    let pad = " ".repeat(MARGIN);
    let edge = format!("{pad}+{}+", "-".repeat(width + 2));
    lines.push(Line::from(Span::styled(edge.clone(), style)));
    for row in wrapped {
        lines.push(Line::from(Span::styled(
            format!("{pad}| {row}{} |", " ".repeat(width - row.width())),
            style,
        )));
    }
    lines.push(Line::from(Span::styled(edge, style)));
}

/// Word-wraps to `width` terminal columns, splitting words that don't fit.
pub(crate) fn wrap_text(message: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let mut current = String::new();
    for word in message.split_whitespace() {
        let word_width = word.width();
        if word_width > width {
            if !current.is_empty() {
                rows.push(std::mem::take(&mut current));
            }
            for ch in word.chars() {
                if current.width() + ch.width().unwrap_or(0) > width {
                    rows.push(std::mem::take(&mut current));
                }
                current.push(ch);
            }
            continue;
        }
        let next_width = if current.is_empty() {
            word_width
        } else {
            current.width() + 1 + word_width
        };
        if next_width > width && !current.is_empty() {
            rows.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        rows.push(current);
    }
    if rows.is_empty() {
        rows.push(String::new());
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn wraps_by_terminal_columns_not_chars() {
        assert_eq!(
            wrap_text("日本語のタイトル", 4),
            ["日本", "語の", "タイ", "トル"]
        );
        assert_eq!(wrap_text("✨ cat ✨", 5), ["✨", "cat", "✨"]);
    }

    #[test]
    fn wraps_long_message_instead_of_ellipsis() {
        let msg = "Make yourself a healthy meal. It doesn't have to be fancy to count.";
        let out = animal_box_lines(Animal::Parrot, msg, 60, 20, Theme::dark());
        let joined: Vec<String> = out.iter().map(text_of).collect();
        assert!(!joined.join("\n").contains("..."));
        assert!(joined.join("\n").contains("healthy"));
    }

    #[test]
    fn never_exceeds_the_area() {
        for (w, h) in [(20, 6), (30, 10), (44, 12), (80, 24), (120, 40), (12, 3)] {
            for animal in Animal::ALL {
                let out = animal_box_lines(
                    animal,
                    "please remember to breathe deeply",
                    w,
                    h,
                    Theme::dark(),
                );
                assert!(
                    out.len() <= h.max(1),
                    "{animal:?} {w}x{h} rows {}",
                    out.len()
                );
                for line in &out {
                    let cols: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
                    assert!(cols <= w, "{animal:?} {w}x{h} line {cols} cols");
                }
            }
        }
    }

    #[test]
    fn tail_lands_on_the_head() {
        for (animal, w, h) in [
            (Animal::Parrot, 60, 24),
            (Animal::Parrot, 40, 14),
            (Animal::Cat, 60, 30),
            (Animal::Bunny, 40, 16),
            (Animal::Llama, 70, 30),
        ] {
            let out = animal_box_lines(animal, "breathe", w, h, Theme::dark());
            let texts: Vec<String> = out.iter().map(text_of).collect();
            let tail = texts.iter().position(|l| l.trim() == "\\").expect("tail");
            let tail2_col = texts[tail + 1].chars().position(|c| c == '\\').unwrap();
            assert_eq!(
                texts[tail].chars().position(|c| c == '\\').unwrap() + 1,
                tail2_col
            );
            // The diagonal ends at the bottom-right corner of its cell, so the head's
            // top-left pixel must fill the top half of the next cell down-right.
            let below = texts[tail + 2].chars().nth(tail2_col + 1).unwrap();
            assert!(
                matches!(below, '█' | '▀'),
                "{animal:?} {w}x{h}: got {below:?}"
            );
        }
    }

    #[test]
    fn tail_starts_inside_the_bubble() {
        for animal in Animal::ALL {
            let out = animal_box_lines(animal, "hi", 80, 30, Theme::dark());
            let texts: Vec<String> = out.iter().map(text_of).collect();
            let bottom = texts
                .iter()
                .rposition(|l| l.trim_start().starts_with('+'))
                .unwrap();
            let edge = &texts[bottom];
            let start = edge.find('+').unwrap();
            let end = edge.rfind('+').unwrap();
            let tail = texts[bottom + 1].find('\\').expect("tail");
            assert!(
                tail > start && tail < end,
                "{animal:?}: tail {tail} outside {start}..{end}"
            );
        }
    }
}
