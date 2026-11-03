use ratatui::style::{Color, Style};

/// UI appearance. `Auto` picks dark vs light from the terminal when possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    Auto,
}

impl ThemeMode {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "dark" | "d" => Some(Self::Dark),
            "light" | "l" => Some(Self::Light),
            "auto" | "a" => Some(Self::Auto),
            _ => None,
        }
    }

    pub fn resolve(self) -> ResolvedTheme {
        match self {
            Self::Dark => ResolvedTheme::Dark,
            Self::Light => ResolvedTheme::Light,
            Self::Auto => detect_terminal_theme(),
        }
    }

    pub fn toggle(self) -> Self {
        match self.resolve() {
            ResolvedTheme::Dark => Self::Light,
            ResolvedTheme::Light => Self::Dark,
        }
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedTheme {
    Dark,
    Light,
}

/// Palette inspired by the original tiny-care-terminal screenshot.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub border: Color,
    pub title: Color,
    /// Repo paths + commit hashes (magenta / deep pink).
    pub hash: Color,
    pub subject: Color,
    /// Fresh relative times (lime / forest).
    pub time_fresh: Color,
    /// Older same-day relative times (yellow / amber).
    pub time_warm: Color,
    pub muted: Color,
    pub accent: Color,
    pub bar: Color,
    pub bar_value: Color,
}

impl Theme {
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode.resolve() {
            ResolvedTheme::Dark => Self::dark(),
            ResolvedTheme::Light => Self::light(),
        }
    }

    /// High-saturation accents on black — matches the classic screenshot.
    pub fn dark() -> Self {
        Self {
            bg: Color::Black,
            fg: Color::White,
            border: Color::Rgb(90, 200, 220),
            title: Color::White,
            hash: Color::Rgb(255, 105, 180),
            subject: Color::White,
            time_fresh: Color::Rgb(120, 230, 90),
            time_warm: Color::Rgb(240, 210, 70),
            muted: Color::Rgb(130, 140, 150),
            accent: Color::Rgb(255, 140, 200),
            bar: Color::Rgb(80, 210, 230),
            bar_value: Color::White,
        }
    }

    /// Deeper accents on a light ground so the same roles stay readable.
    pub fn light() -> Self {
        Self {
            bg: Color::Rgb(250, 250, 252),
            fg: Color::Rgb(28, 30, 36),
            border: Color::Rgb(25, 110, 145),
            title: Color::Rgb(15, 85, 120),
            hash: Color::Rgb(170, 25, 110),
            subject: Color::Rgb(28, 30, 36),
            time_fresh: Color::Rgb(15, 120, 55),
            time_warm: Color::Rgb(150, 100, 10),
            muted: Color::Rgb(100, 108, 118),
            accent: Color::Rgb(150, 40, 100),
            bar: Color::Rgb(20, 130, 165),
            bar_value: Color::Rgb(250, 250, 252),
        }
    }

    pub fn root_style(self) -> Style {
        Style::default().bg(self.bg).fg(self.fg)
    }

    pub fn border_style(self) -> Style {
        Style::default().fg(self.border)
    }

    pub fn title_style(self) -> Style {
        Style::default().fg(self.title)
    }

    pub fn muted_style(self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn accent_style(self) -> Style {
        Style::default().fg(self.accent)
    }
}

/// Best-effort light/dark detection via `COLORFGBG` (common in xterm-likes).
fn detect_terminal_theme() -> ResolvedTheme {
    if let Ok(v) = std::env::var("COLORFGBG") {
        // Format: "fg;bg" where bg is a palette index 0–15 (or higher).
        if let Some(bg) = v.split(';').next_back()
            && let Ok(idx) = bg.trim().parse::<u8>()
        {
            // Light backgrounds are typically 7 / 15 (white) or similar high indices.
            return if idx == 7 || idx >= 15 {
                ResolvedTheme::Light
            } else {
                ResolvedTheme::Dark
            };
        }
    }
    ResolvedTheme::Dark
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_theme_names() {
        assert_eq!(ThemeMode::parse("dark"), Some(ThemeMode::Dark));
        assert_eq!(ThemeMode::parse("LIGHT"), Some(ThemeMode::Light));
        assert_eq!(ThemeMode::parse("auto"), Some(ThemeMode::Auto));
        assert_eq!(ThemeMode::parse("nope"), None);
    }

    #[test]
    fn toggle_flips_resolved() {
        assert_eq!(ThemeMode::Dark.toggle(), ThemeMode::Light);
        assert_eq!(ThemeMode::Light.toggle(), ThemeMode::Dark);
    }
}
