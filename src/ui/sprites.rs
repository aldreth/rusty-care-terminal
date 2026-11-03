//! Say-box characters as hand-pixelled sprites, shown with Unicode half-blocks (▀/▄/█).
//!
//! Art is drawn at a fixed native size and only scaled by **integer** factors (or
//! shrunk to fit). That keeps a chunky computery look instead of stretching tiny
//! silhouettes into mush. Custom images still use the same half-block path.
//!
//! Map legend:
//! `.` empty · `B` body · `D` body shade · `W` white · `K` black · `M` magenta
//! `Y` yellow · `R` red · `O` orange · `C` cream · `N` brown · `G` gray · `P` pink
//! `A` cream shade · `H` gray shade

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

#[derive(Clone, Copy, PartialEq, Eq)]
struct Rgb(u8, u8, u8);

impl Rgb {
    fn color(self) -> Color {
        Color::Rgb(self.0, self.1, self.2)
    }
}

const CYAN: Rgb = Rgb(64, 212, 232);
const CYAN_D: Rgb = Rgb(28, 150, 180);
const WHITE: Rgb = Rgb(248, 248, 252);
const BLACK: Rgb = Rgb(18, 18, 24);
const MAGENTA: Rgb = Rgb(255, 80, 170);
const YELLOW: Rgb = Rgb(255, 214, 64);
const ORANGE: Rgb = Rgb(255, 140, 48);
const RED: Rgb = Rgb(240, 64, 64);
const CREAM: Rgb = Rgb(255, 228, 196);
const CREAM_D: Rgb = Rgb(220, 180, 140);
const BROWN: Rgb = Rgb(150, 90, 50);
const GRAY: Rgb = Rgb(170, 176, 188);
const GRAY_D: Rgb = Rgb(110, 116, 128);
const PINK: Rgb = Rgb(255, 160, 190);

fn map_char(ch: char, body: Rgb, shade: Rgb) -> Option<Rgb> {
    match ch {
        'B' | '#' => Some(body),
        'D' => Some(shade),
        'W' => Some(WHITE),
        'K' => Some(BLACK),
        'M' => Some(MAGENTA),
        'Y' => Some(YELLOW),
        'O' => Some(ORANGE),
        'R' => Some(RED),
        'C' => Some(CREAM),
        'N' => Some(BROWN),
        'G' => Some(GRAY),
        'P' => Some(PINK),
        'A' => Some(CREAM_D),
        'H' => Some(GRAY_D),
        _ => None,
    }
}

/// Side-view party parrot: one eye, chunky beak sticking out, wing, perch feet.
const PARROT: &[&str] = &[
    "............................",
    ".......BBBBBBB..............",
    ".....BBBBBBBBBBB............",
    "....BBBBBBBBBBBBB...........",
    "....BBBWWBBBBBBBBB..........",
    "....BBBKWBBBBBBBBB..........",
    "....BBBBBBMMBBBBBB..........",
    ".....BBBBBMMMOYBBB..........",
    "......BBBBMMMOYYBB..........",
    ".......BBBMMMYYYB...........",
    "......BBBBBBBBBBBB..........",
    ".....BBBBBBBBBBBBBB.........",
    "....BBBBBBDDDBBBBBBB........",
    "...BBBBBBBDDDBBBBBBBB.......",
    "...BBBBBBBDDDBBBBBBBB.......",
    "....BBBBBBDDDBBBBBBB........",
    ".....BBBBBBBBBBBBBB.........",
    "......BBBBBBBBBBBB..........",
    ".......BBBRRRBBB............",
    "........B.R..R.B............",
    "........N......N............",
];

/// Front bunny with long ears, round face, pink nose, small chest.
const BUNNY: &[&str] = &[
    "......................",
    "...BB........BB.......",
    "..BPPB......BPPB......",
    "..BPPB......BPPB......",
    "..BPPB......BPPB......",
    "..BPPB......BPPB......",
    "...BBB......BBB.......",
    "....BBBBBBBBBB........",
    "...BBWWBBBBWWBB.......",
    "...BBKWBBBBKWBB.......",
    "...BBBBPPBBBBBB.......",
    "...BBBBBBBBBBBB.......",
    "....BBBAAAABBB........",
    ".....BBBBBABB.........",
    "......BBBBBB..........",
];

/// Side llama: tall ears, one eye, cream muzzle, long neck, body, four legs.
const LLAMA: &[&str] = &[
    "........................",
    ".............B...B......",
    ".............B...B......",
    "............BBBBBBB.....",
    "...........BBWWBBBBB....",
    "...........BBKWBBBBBK...",
    "...........BBBBBBBBB....",
    "............BBBBBBB.....",
    ".............BBBB.......",
    ".............BBB........",
    ".............BBB........",
    ".............BBB........",
    ".............BBB........",
    "......BBBBBBBBBBB.......",
    ".....BBBBBBBBBBBBB......",
    "....BBBBADDDDABBBBB.....",
    "....BB..BB..BB..BB......",
    "....BB..BB..BB..BB......",
    "....NN..NN..NN..NN......",
];

/// Front cat: tall triangle ears, round head, nose, whiskers.
const CAT: &[&str] = &[
    "......................",
    "..G............G......",
    ".GHG..........GHG.....",
    ".GHHG........GHHG.....",
    ".GHHHG......GHHHG.....",
    "..GHHHHHHHHHHHHG......",
    ".GHHWWHHHHHHWWHHG.....",
    ".GHHKWHHHHHHWKHHG.....",
    "G.G.HHHHMMHHHH.G.G....",
    ".GHHHHHHHHHHHHHHG.....",
    "..GHHHHHHHHHHHHG......",
    "...GGHHHHHHHHGG.......",
    ".....GGGGGGGG.........",
];

pub fn sprite_lines(name: &str, max_cols: usize, max_rows: usize) -> Vec<Line<'static>> {
    let (map, body, shade) = match name {
        "bunny" => (BUNNY, CREAM, CREAM_D),
        "llama" => (LLAMA, CREAM, CREAM_D),
        "cat" => (CAT, GRAY, GRAY_D),
        _ => (PARROT, CYAN, CYAN_D),
    };
    let (w, h, pixels) = rasterize(map, body, shade);
    let max_px_h = max_rows.saturating_mul(2).max(2);
    let (tw, th) = fit_size(w, h, max_cols.max(4), max_px_h);
    let scaled = scale_pixels(&pixels, w, h, tw, th);
    let mut lines = halfblocks(&scaled, tw, th);
    center_art(&mut lines, max_cols, max_rows);
    lines
}

fn rasterize(map: &[&str], body: Rgb, shade: Rgb) -> (usize, usize, Vec<Option<Rgb>>) {
    let h = map.len();
    let w = map.iter().map(|r| r.chars().count()).max().unwrap_or(0);
    let mut pixels = vec![None; w * h];
    for (y, row) in map.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            pixels[y * w + x] = map_char(ch, body, shade);
        }
    }
    (w, h, pixels)
}

/// Prefer integer upscale; only shrink when the native sprite cannot fit.
fn fit_size(src_w: usize, src_h: usize, max_cols: usize, max_px_h: usize) -> (usize, usize) {
    if src_w == 0 || src_h == 0 {
        return (1, 1);
    }
    let max_cols = max_cols.max(1);
    let max_px_h = max_px_h.max(1);

    let up_w = max_cols / src_w;
    let up_h = max_px_h / src_h;
    let up = up_w.min(up_h);
    if up >= 1 {
        let up = up.min(3);
        return (src_w * up, src_h * up);
    }

    let scale = (max_cols as f32 / src_w as f32).min(max_px_h as f32 / src_h as f32);
    let tw = ((src_w as f32) * scale).floor().max(1.0) as usize;
    let th = ((src_h as f32) * scale).floor().max(1.0) as usize;
    (tw.min(max_cols).max(1), th.min(max_px_h).max(1))
}

fn scale_pixels(
    src: &[Option<Rgb>],
    sw: usize,
    sh: usize,
    tw: usize,
    th: usize,
) -> Vec<Option<Rgb>> {
    if tw == sw && th == sh {
        return src.to_vec();
    }
    if tw.is_multiple_of(sw) && th.is_multiple_of(sh) {
        let sx = tw / sw;
        let sy = th / sh;
        let mut out = vec![None; tw * th];
        for y in 0..sh {
            for x in 0..sw {
                let p = src[y * sw + x];
                for dy in 0..sy {
                    for dx in 0..sx {
                        out[(y * sy + dy) * tw + (x * sx + dx)] = p;
                    }
                }
            }
        }
        return out;
    }
    let mut out = vec![None; tw * th];
    for y in 0..th {
        let sy = y * sh / th;
        for x in 0..tw {
            let sx = x * sw / tw;
            out[y * tw + x] = src[sy * sw + sx];
        }
    }
    out
}

fn halfblocks(pixels: &[Option<Rgb>], width: usize, height: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut y = 0;
    while y < height {
        let mut spans = Vec::with_capacity(width);
        for x in 0..width {
            let top = pixels[y * width + x];
            let bot = if y + 1 < height {
                pixels[(y + 1) * width + x]
            } else {
                None
            };
            spans.push(halfblock_span(top, bot));
        }
        lines.push(Line::from(spans));
        y += 2;
    }
    lines
}

fn halfblock_span(top: Option<Rgb>, bot: Option<Rgb>) -> Span<'static> {
    match (top, bot) {
        (Some(t), Some(b)) if t == b => Span::styled("█", Style::default().fg(t.color())),
        (Some(t), Some(b)) => Span::styled("▀", Style::default().fg(t.color()).bg(b.color())),
        (Some(t), None) => Span::styled("▀", Style::default().fg(t.color())),
        (None, Some(b)) => Span::styled("▄", Style::default().fg(b.color())),
        (None, None) => Span::raw(" "),
    }
}

fn center_art(lines: &mut Vec<Line<'static>>, max_cols: usize, max_rows: usize) {
    if lines.is_empty() {
        return;
    }
    let art_w = lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.chars().count()).sum::<usize>())
        .max()
        .unwrap_or(0);
    let pad_left = max_cols.saturating_sub(art_w) / 2;
    if pad_left > 0 {
        let pad = " ".repeat(pad_left);
        for line in lines.iter_mut() {
            let mut spans = vec![Span::raw(pad.clone())];
            spans.append(&mut line.spans);
            *line = Line::from(spans);
        }
    }
    let pad_top = max_rows.saturating_sub(lines.len()) / 2;
    if pad_top > 0 {
        let mut padded = vec![Line::from(""); pad_top];
        padded.append(lines);
        *lines = padded;
    }
}

/// Convert an RGBA image buffer into half-block lines (custom `RCT_SAY_BOX` images).
pub fn rgba_to_halfblocks(
    rgba: &[u8],
    width: u32,
    height: u32,
    max_cols: usize,
    max_rows: usize,
) -> Vec<Line<'static>> {
    let sw = width as usize;
    let sh = height as usize;
    let mut pixels = vec![None; sw * sh];
    for y in 0..sh {
        for x in 0..sw {
            let i = (y * sw + x) * 4;
            if i + 3 >= rgba.len() {
                continue;
            }
            let a = rgba[i + 3];
            if a < 128 {
                continue;
            }
            pixels[y * sw + x] = Some(Rgb(rgba[i], rgba[i + 1], rgba[i + 2]));
        }
    }
    let max_px_h = max_rows.saturating_mul(2).max(2);
    let (tw, th) = fit_size(sw, sh, max_cols.max(4), max_px_h);
    let scaled = scale_pixels(&pixels, sw, sh, tw, th);
    let mut lines = halfblocks(&scaled, tw, th);
    center_art(&mut lines, max_cols, max_rows);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parrot_renders_nonempty() {
        let lines = sprite_lines("parrot", 40, 14);
        assert!(lines.len() >= 6);
        let has_block = lines.iter().any(|l| {
            l.spans.iter().any(|s| {
                s.content.contains('▀') || s.content.contains('▄') || s.content.contains('█')
            })
        });
        assert!(has_block);
    }

    #[test]
    fn all_creatures_render() {
        for name in ["parrot", "bunny", "llama", "cat"] {
            let lines = sprite_lines(name, 36, 12);
            assert!(!lines.is_empty(), "{name} empty");
        }
    }

    #[test]
    fn integer_upscale_keeps_aspect() {
        let (w, h) = fit_size(10, 8, 40, 24);
        assert_eq!((w, h), (30, 24));
    }

    #[test]
    fn shrinks_when_needed() {
        let (w, h) = fit_size(40, 30, 20, 14);
        assert!(w <= 20);
        assert!(h <= 14);
    }
}
