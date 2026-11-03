//! Say-box creatures as outlined pixel sprites, drawn with half-blocks (▀ ▄ █).
//!
//! Each creature has hand-drawn size variants. Layout picks the tallest variant
//! and integer scale that fits, so pixels stay square and crisp at any terminal
//! size instead of being resampled into mush.
//!
//! Map legend: `.` transparent · `K` outline · `B` parrot blue · `P` beak ·
//! `O` cat orange · `W` white · `p` pink · `C` llama cream · `R` red ·
//! `Y` yellow · `T` teal

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::model::Animal;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Rgb(u8, u8, u8);

impl Rgb {
    fn color(self) -> Color {
        Color::Rgb(self.0, self.1, self.2)
    }
}

fn palette(ch: char) -> Option<Rgb> {
    match ch {
        'K' => Some(Rgb(20, 20, 24)),
        'B' => Some(Rgb(98, 178, 255)),
        'P' => Some(Rgb(178, 92, 128)),
        'O' => Some(Rgb(255, 160, 64)),
        'W' => Some(Rgb(240, 240, 245)),
        'p' => Some(Rgb(255, 150, 180)),
        'C' => Some(Rgb(245, 225, 190)),
        'R' => Some(Rgb(230, 70, 80)),
        'Y' => Some(Rgb(255, 205, 60)),
        'T' => Some(Rgb(40, 190, 180)),
        _ => None,
    }
}

/// The classic party parrot from the original tiny-care-terminal screenshot.
const PARROT_LARGE: &[&str] = &[
    ".......KKKKKKKK.................",
    "......KKBBBBBBKKK...............",
    ".....KKBBBBBBBBBKK..............",
    "....KKBBBBBBBBBBBK..............",
    "...KKBBBBBBBBBBBBKK.............",
    "..KKBBBKBBBKKKBBKBK.............",
    "..KBBBBKBBKPPPKBKBK.............",
    ".KKBBBBBBKPPPPPKBBKK............",
    ".KBBBBBBBKPPPPPKBBBKK...........",
    "KKBBBBBBBKPPPPPKBBBBK...........",
    "KBBBBBBBBKPPPPPKBBBBK...........",
    "KBBBBBBBBKPPPPPKBBBBK...........",
    "KBBBBBBBBKPPPPPKBBBBK...........",
    "KBBBBBBBBBKPPPKBBBBBK...........",
    "KBBBBBBBBBKPPPKBBBBBK...........",
    "KBBBBBBBBBBKKKBBBBBBKK..........",
    "KBBBBBBBBBBBKBBBBBBBBKK.........",
    "KKBBBBBBBBBBBBBBBBBBBBKK........",
    ".KBBBBBBBBBBBBBBBBBBBBBKKK......",
    ".KBBBBBBBBBBBBBBBBBBBBBBBKK.....",
    ".KBKKBBBBBBBBBBBBBBBBBBBBBKKK...",
    ".KBBKKKKKBBBBBBBBBBBBBBBBBBBKK..",
    ".KBBBBBKKKKBBBBBBBBBBBBBBBBBBKKK",
    ".KBBBBBBBBBBBBBBBBBBBBBBBBBBBBBK",
    ".KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK",
];

const PARROT_SMALL: &[&str] = &[
    "....KKKKKKK.........",
    "..KKBBBBBBBKK.......",
    ".KBBBBBBBBBBBK......",
    ".KBBBBBBBBBBBK......",
    "KBBKBKKKKKBKBBK.....",
    "KBBKBKPPPKBKBBK.....",
    "KBBBBKPPPKBBBBK.....",
    "KBBBBKPPPKBBBBK.....",
    "KBBBBBKPKBBBBBKK....",
    "KBBBBBBKBBBBBBBBK...",
    "KBBBBBBBBBBBBBBBBK..",
    "KBKKBBBBBBBBBBBBBBK.",
    "KBBBKKBBBBBBBBBBBBBK",
    "KKKKKKKKKKKKKKKKKKKK",
];

const PARROT_TINY: &[&str] = &[
    "...KKKKKKK......",
    ".KKBBBBBBBKK....",
    "KBKBBKKKBBKBK...",
    "KBKBBKPKBBKBK...",
    "KBBBBKPKBBBBBK..",
    "KBBBBBKBBBBBBBK.",
    "KBKKBBBBBBBBBBBK",
    "KKKKKKKKKKKKKKKK",
];

const CAT_TINY: &[&str] = &[
    ".K.......K..",
    "KOK.....KOK.",
    "KOpKKKKKpOK.",
    "KOOOOOOOOOK.",
    "KOKOOOOOKOK.",
    "KOOOOpOOOOKK",
    ".KWOOOOOWOKK",
    ".KKKKKKKKKK.",
];

const BUNNY_TINY: &[&str] = &[
    ".KK...KK..",
    "KWpK.KpWK.",
    "KWpK.KpWK.",
    ".KWKKKWK..",
    "KWWWWWWWK.",
    "KWKWWWKWK.",
    "KpWWKWWpK.",
    "KWWWWWWWWK",
    ".KKKKKKKK.",
];

const LLAMA_TINY: &[&str] = &[
    "..K..K.......",
    ".KpKKpK......",
    "KCKCCCK......",
    "KpCCCCK......",
    ".KKKCCK....K.",
    "...KCCKKKKKCK",
    "...KCRYTRYTCK",
    "...KCCCCCCCCK",
    "....KCKKKKCK.",
    "....KKK..KKK.",
];

const CAT: &[&str] = &[
    ".K..........K...",
    "KOK........KOK..",
    "KOOK......KOOK..",
    "KOpOKKKKKKOpOK..",
    "KOOOOOOOOOOOOK..",
    "KOOKOOOOOOKOOK..",
    "KOOKOOOOOOKOOK..",
    "KOOOOOKpKOOOOK..",
    ".KOOOOOKOOOOK...",
    "..KOOOOOOOOK....",
    ".KOOOOOOOOOOK.KK",
    "KOOWWOOOOWWOOKOK",
    "KOOWWOOOOWWOOOK.",
    ".KKKKKKKKKKKKK..",
];

const BUNNY: &[&str] = &[
    "..KK....KK......",
    ".KWpK..KpWK.....",
    ".KWpK..KpWK.....",
    ".KWpK..KpWK.....",
    ".KWpK..KpWK.....",
    "..KWKKKKWK......",
    ".KWWWWWWWWK.....",
    "KWWKWWWWKWWK....",
    "KWWKWWWWKWWK....",
    "KWpWWKpKWWpWK...",
    ".KWWWWKWWWWK....",
    ".KWWWWWWWWWWK...",
    "KWWWWWWWWWWWWK..",
    "KWWWWWWWWWWWWK..",
    ".KKKKKKKKKKKK...",
];

const LLAMA_XL: &[&str] = &[
    ".....K....K.................",
    "....KpK..KpK................",
    "....KpK..KpK................",
    "...KKpKKKKpKK...............",
    "..KCCCCCCCCCK...............",
    "..KCCCCCCCCCK...............",
    ".KCCKKCCCCCCK...............",
    "KCCCKKCCCCCCK...............",
    "KpCCCCCCCCCCK...............",
    "KCCCCCCCCCCCK...............",
    ".KKKKCCCCCCCK...............",
    "....KCCCCCCCK...............",
    "....KCCCCCCCK...............",
    "....KCCCCCCCK...............",
    "....KCCCCCCCK...........KKK.",
    "....KCCCCCCCKKKKKKKKKKKKCCCK",
    "...KCCCCCCCCCCCCCCCCCCCCCCCK",
    "...KCCRRYYTTRRYYTTRRYYTTCCCK",
    "...KCCRRYYTTRRYYTTRRYYTTCCCK",
    "...KCCRRYYTTRRYYTTRRYYTTCCCK",
    "...KCCCCCCCCCCCCCCCCCCCCCCCK",
    "...KCCCCCCCCCCCCCCCCCCCCCCCK",
    "....KCCCKKKKKKKKKKKKKKKCCCK.",
    "....KCCCK.............KCCCK.",
    "....KCCCK.............KCCCK.",
    "....KCCCK.............KCCCK.",
    "....KKKKK.............KKKKK.",
];

const LLAMA: &[&str] = &[
    "....K...K............",
    "...KpK.KpK...........",
    "...KpKKKpK...........",
    "..KCCCCCCK...........",
    ".KCCKCCCCK...........",
    "KCCCCCCCCK...........",
    "KpCCCCCCCK...........",
    ".KKKKCCCCK...........",
    "....KCCCCK...........",
    "....KCCCCK...........",
    "....KCCCCK........KK.",
    "....KCCCCKKKKKKKKKCCK",
    "...KCCCCCCCCCCCCCCCCK",
    "...KCRYTRYTRYTRYTRCCK",
    "...KCRYTRYTRYTRYTRCCK",
    "...KCCCCCCCCCCCCCCCCK",
    "....KCCKKKKKKKKKKCCK.",
    "....KCCK........KCCK.",
    "....KCCK........KCCK.",
    "....KKKK........KKKK.",
];

const LLAMA_SMALL: &[&str] = &[
    "...K..K..........",
    "..KpKKpK.........",
    "..KCCCCK.........",
    ".KCKCCCK.........",
    "KCCCCCCK.........",
    "KpCCCCCK.........",
    ".KKKCCCK.........",
    "...KCCCK.........",
    "...KCCCK......KK.",
    "...KCCCKKKKKKKCCK",
    "..KCCCCCCCCCCCCCK",
    "..KCRYTRYTRYTRCCK",
    "..KCRYTRYTRYTRCCK",
    "..KCCCCCCCCCCCCCK",
    "...KCCKKKKKKKCCK.",
    "...KCCK.....KCCK.",
    "...KKKK.....KKKK.",
];

/// A pixel grid with an anchor: the column of the head's top-left outline pixel,
/// where the speech-bubble tail lands.
#[derive(Clone)]
pub struct Sprite {
    width: usize,
    height: usize,
    pixels: Vec<Option<Rgb>>,
    anchor_x: usize,
}

impl Sprite {
    fn from_map(map: &[&str]) -> Self {
        let height = map.len();
        let width = map.iter().map(|r| r.chars().count()).max().unwrap_or(0);
        let mut pixels = vec![None; width * height];
        for (y, row) in map.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                pixels[y * width + x] = palette(ch);
            }
        }
        let anchor_x = (0..width)
            .find(|&x| pixels[x].is_some())
            .unwrap_or(width / 2);
        Self {
            width,
            height,
            pixels,
            anchor_x,
        }
    }

    fn rows(&self, scale: usize) -> usize {
        (self.height * scale).div_ceil(2)
    }

    fn cols(&self, scale: usize) -> usize {
        self.width * scale
    }

    fn pixel(&self, x: usize, y: usize, scale: usize) -> Option<Rgb> {
        let (sx, sy) = (x / scale, y / scale);
        if sx < self.width && sy < self.height {
            self.pixels[sy * self.width + sx]
        } else {
            None
        }
    }
}

/// Hand-drawn variants for an animal, largest first.
pub fn creature(animal: Animal) -> Vec<Sprite> {
    match animal {
        Animal::Cat => vec![Sprite::from_map(CAT), Sprite::from_map(CAT_TINY)],
        Animal::Bunny => vec![Sprite::from_map(BUNNY), Sprite::from_map(BUNNY_TINY)],
        Animal::Llama => vec![
            Sprite::from_map(LLAMA_XL),
            Sprite::from_map(LLAMA),
            Sprite::from_map(LLAMA_SMALL),
            Sprite::from_map(LLAMA_TINY),
        ],
        Animal::Parrot => vec![
            Sprite::from_map(PARROT_LARGE),
            Sprite::from_map(PARROT_SMALL),
            Sprite::from_map(PARROT_TINY),
        ],
    }
}

/// A sprite laid out in terminal cells.
pub struct Placed {
    pub lines: Vec<Line<'static>>,
    pub cols: usize,
    /// Cell column (from the sprite's left edge) where the bubble tail should land.
    pub anchor_col: usize,
}

/// Within this share of the tallest fit, a less-upscaled variant wins, so
/// hand-drawn detail beats chunky pixels without one animal shrinking next to
/// another that happens to double neatly.
const NEAR_TALLEST_PERCENT: usize = 85;

/// The tallest variant × integer scale (1–3) that fits, preferring the lowest
/// scale among candidates nearly as tall.
pub fn fit(variants: &[Sprite], max_cols: usize, max_rows: usize) -> Option<Placed> {
    let candidates: Vec<(&Sprite, usize)> = variants
        .iter()
        .flat_map(|sprite| (1..=3).map(move |scale| (sprite, scale)))
        .filter(|(sprite, scale)| {
            sprite.cols(*scale) <= max_cols && sprite.rows(*scale) <= max_rows
        })
        .collect();
    let tallest = candidates
        .iter()
        .map(|(sprite, scale)| sprite.rows(*scale))
        .max()?;
    let (sprite, scale) = candidates
        .into_iter()
        .filter(|(sprite, scale)| sprite.rows(*scale) * 100 >= tallest * NEAR_TALLEST_PERCENT)
        .min_by_key(|(sprite, scale)| (*scale, std::cmp::Reverse(sprite.rows(*scale))))?;
    Some(Placed {
        lines: halfblocks(sprite, scale),
        cols: sprite.cols(scale),
        anchor_col: sprite.anchor_x * scale,
    })
}

fn halfblocks(sprite: &Sprite, scale: usize) -> Vec<Line<'static>> {
    let width = sprite.cols(scale);
    let height = sprite.height * scale;
    (0..height)
        .step_by(2)
        .map(|y| {
            let spans: Vec<Span<'static>> = (0..width)
                .map(|x| cell(sprite.pixel(x, y, scale), sprite.pixel(x, y + 1, scale)))
                .collect();
            Line::from(spans)
        })
        .collect()
}

fn cell(top: Option<Rgb>, bottom: Option<Rgb>) -> Span<'static> {
    match (top, bottom) {
        (Some(t), Some(b)) if t == b => Span::styled("█", Style::default().fg(t.color())),
        (Some(t), Some(b)) => Span::styled("▀", Style::default().fg(t.color()).bg(b.color())),
        (Some(t), None) => Span::styled("▀", Style::default().fg(t.color())),
        (None, Some(b)) => Span::styled("▄", Style::default().fg(b.color())),
        (None, None) => Span::raw(" "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_creature_has_a_variant() {
        for animal in Animal::ALL {
            assert!(!creature(animal).is_empty(), "{animal:?}");
        }
    }

    #[test]
    fn maps_are_rectangular() {
        for map in [
            PARROT_LARGE,
            PARROT_SMALL,
            PARROT_TINY,
            CAT,
            CAT_TINY,
            BUNNY,
            BUNNY_TINY,
            LLAMA_XL,
            LLAMA,
            LLAMA_SMALL,
            LLAMA_TINY,
        ] {
            let w = map[0].chars().count();
            assert!(map.iter().all(|r| r.chars().count() == w));
        }
    }

    #[test]
    fn picks_large_parrot_when_roomy_and_small_when_tight() {
        let roomy = fit(&creature(Animal::Parrot), 40, 14).expect("fits");
        assert_eq!(roomy.cols, 32);
        let tight = fit(&creature(Animal::Parrot), 40, 8).expect("fits");
        assert_eq!(tight.cols, 20);
        let tiny = fit(&creature(Animal::Parrot), 40, 4).expect("fits");
        assert_eq!(tiny.cols, 16);
        assert!(fit(&creature(Animal::Parrot), 40, 3).is_none());
    }

    #[test]
    fn upscales_by_whole_pixels() {
        let big = fit(&creature(Animal::Cat), 60, 30).expect("fits");
        assert_eq!(big.cols, 48);
        assert_eq!(big.lines.len(), 21);
    }

    #[test]
    fn llama_faces_left() {
        for map in [LLAMA_XL, LLAMA, LLAMA_SMALL, LLAMA_TINY] {
            let llama = Sprite::from_map(map);
            assert!(llama.anchor_x < llama.width / 2);
        }
    }

    #[test]
    fn llama_is_as_big_as_the_others() {
        for rows in 14..=30 {
            let height = |animal| fit(&creature(animal), 60, rows).expect("fits").lines.len();
            let llama = height(Animal::Llama);
            for animal in Animal::ALL {
                let other = height(animal);
                assert!(
                    llama * 100 >= other * 85,
                    "llama {llama} vs {animal:?} {other} at {rows} rows"
                );
            }
        }
    }

    #[test]
    fn anchor_is_head_top_left() {
        let placed = fit(&creature(Animal::Parrot), 40, 14).expect("fits");
        assert_eq!(placed.anchor_col, 7);
    }
}
