//! Pixel sprites for the say-box, rendered with Unicode half-blocks (▀/▄).
//!
//! Each sprite is a compact character map:
//! `.` transparent · `B` body · `K` beak · `E` eye · `P` pupil · `O` outline

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

#[derive(Clone, Copy)]
struct Rgb(u8, u8, u8);

impl Rgb {
    fn color(self) -> Color {
        Color::Rgb(self.0, self.1, self.2)
    }
}

const CYAN: Rgb = Rgb(80, 200, 230);
const MAGENTA: Rgb = Rgb(255, 105, 180);
const WHITE: Rgb = Rgb(240, 240, 245);
const BLACK: Rgb = Rgb(20, 20, 28);
const OUTLINE: Rgb = Rgb(40, 60, 80);
const CREAM: Rgb = Rgb(255, 230, 200);
const BROWN: Rgb = Rgb(160, 100, 60);
const GRAY: Rgb = Rgb(180, 180, 190);

fn map_char(ch: char, body: Rgb) -> Option<Rgb> {
    match ch {
        'B' | '#' => Some(body),
        'K' => Some(MAGENTA),
        'E' => Some(WHITE),
        'P' => Some(BLACK),
        'O' => Some(OUTLINE),
        'C' => Some(CREAM),
        'N' => Some(BROWN),
        'G' => Some(GRAY),
        _ => None,
    }
}

/// Party parrot — filled silhouette inspired by the classic meme bird.
const PARROT: &[&str] = &[
    "......................",
    "........OOOOOO........",
    "......OOEEEEEEOO......",
    ".....OEBPEEBPEEO......",
    ".....OEBBBBKBBEO......",
    "......OBBBBBBBO.......",
    ".....OOBBBBBBBOO......",
    "....OOBBBBBBBBBOO.....",
    "...OOBBBB..BBBBBOO....",
    "...OBBBB....BBBBO.....",
    "..OOBBB......BBBOO....",
    "..OBBB........BBBO....",
    "..OBB..........BBO....",
    "...OO..........OO.....",
];

const BUNNY: &[&str] = &[
    "............",
    "..O....O....",
    ".OBO..OBO...",
    ".OBO..OBO...",
    "..OBBBBBO...",
    ".OBEPEPEBO..",
    ".OBBBBBBBB..",
    "..ONN..NNO..",
    "...OOOOOO...",
];

const LLAMA: &[&str] = &[
    ".............",
    "....OOOO.....",
    "...OEEEEO....",
    "...OEPPEO....",
    "...OCCCCCO...",
    "..OCCCCCCCO..",
    ".OCCC...CCCO.",
    ".OCC.....CCO.",
    ".ON.......NO.",
    "..O.......O..",
];

const CAT: &[&str] = &[
    ".............",
    ".O.........O.",
    ".OO.......OO.",
    "..OGGGGGGGO..",
    ".OGGEPEPEGGO.",
    ".OGGGKGGGGO..",
    "..OGGGGGGGO..",
    "...OOOOOOO...",
];

pub fn sprite_lines(name: &str, max_cols: usize, max_rows: usize) -> Vec<Line<'static>> {
    let (map, body) = match name {
        "bunny" => (BUNNY, CREAM),
        "llama" => (LLAMA, CREAM),
        "cat" => (CAT, GRAY),
        _ => (PARROT, CYAN),
    };
    let (w, h, pixels) = rasterize(map, body);
    let (tw, th) = fit_size(w, h, max_cols, max_rows.saturating_mul(2));
    let scaled = nearest_neighbor(&pixels, w, h, tw, th);
    halfblocks(&scaled, tw, th)
}

fn rasterize(map: &[&str], body: Rgb) -> (usize, usize, Vec<Option<Rgb>>) {
    let h = map.len();
    let w = map.iter().map(|r| r.chars().count()).max().unwrap_or(0);
    let mut pixels = vec![None; w * h];
    for (y, row) in map.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            pixels[y * w + x] = map_char(ch, body);
        }
    }
    (w, h, pixels)
}

fn fit_size(src_w: usize, src_h: usize, max_cols: usize, max_px_h: usize) -> (usize, usize) {
    if src_w == 0 || src_h == 0 {
        return (1, 1);
    }
    let max_cols = max_cols.max(8);
    let max_px_h = max_px_h.max(8);
    // Prefer filling height; clamp width.
    let scale_h = max_px_h as f32 / src_h as f32;
    let scale_w = max_cols as f32 / src_w as f32;
    let scale = scale_h.min(scale_w).max(1.0);
    let tw = ((src_w as f32) * scale).round().max(1.0) as usize;
    let th = ((src_h as f32) * scale).round().max(1.0) as usize;
    (tw.min(max_cols), th.min(max_px_h))
}

fn nearest_neighbor(
    src: &[Option<Rgb>],
    sw: usize,
    sh: usize,
    tw: usize,
    th: usize,
) -> Vec<Option<Rgb>> {
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
        (Some(t), Some(b)) => Span::styled("▀", Style::default().fg(t.color()).bg(b.color())),
        (Some(t), None) => Span::styled("▀", Style::default().fg(t.color())),
        (None, Some(b)) => Span::styled("▄", Style::default().fg(b.color())),
        (None, None) => Span::raw(" "),
    }
}

/// Convert an RGBA image buffer into half-block lines.
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
    let (tw, th) = fit_size(sw, sh, max_cols, max_rows.saturating_mul(2));
    let scaled = nearest_neighbor(&pixels, sw, sh, tw, th);
    halfblocks(&scaled, tw, th)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parrot_renders_nonempty() {
        let lines = sprite_lines("parrot", 40, 12);
        assert!(lines.len() >= 4);
        let has_block = lines.iter().any(|l| {
            l.spans
                .iter()
                .any(|s| s.content.contains('▀') || s.content.contains('▄'))
        });
        assert!(has_block);
    }
}
