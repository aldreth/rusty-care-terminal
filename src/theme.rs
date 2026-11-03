use std::process::Command;

use ratatui::style::{Color, Style};

/// UI appearance. `Auto` (default) follows the terminal/OS when `RCT_THEME` is unset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    Dark,
    Light,
    #[default]
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
            Self::Auto => detect_preferred_theme(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedTheme {
    Dark,
    Light,
}

/// Palette inspired by the original tiny-care-terminal screenshot.
///
/// Background is never set — the terminal’s own theme (e.g. Solarized) shows through.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
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

    /// High-saturation accents for dark terminal themes.
    pub fn dark() -> Self {
        Self {
            fg: Color::Reset,
            border: Color::Rgb(90, 200, 220),
            title: Color::Reset,
            hash: Color::Rgb(255, 105, 180),
            subject: Color::Reset,
            time_fresh: Color::Rgb(120, 230, 90),
            time_warm: Color::Rgb(240, 210, 70),
            muted: Color::Rgb(130, 140, 150),
            accent: Color::Rgb(255, 140, 200),
            bar: Color::Rgb(80, 210, 230),
            bar_value: Color::Rgb(16, 24, 32),
        }
    }

    /// Deeper accents for light terminal themes (still no painted background).
    pub fn light() -> Self {
        Self {
            fg: Color::Reset,
            border: Color::Rgb(25, 110, 145),
            title: Color::Rgb(15, 85, 120),
            hash: Color::Rgb(170, 25, 110),
            subject: Color::Reset,
            time_fresh: Color::Rgb(15, 120, 55),
            time_warm: Color::Rgb(150, 100, 10),
            muted: Color::Rgb(100, 108, 118),
            accent: Color::Rgb(150, 40, 100),
            bar: Color::Rgb(20, 130, 165),
            bar_value: Color::Rgb(250, 250, 250),
        }
    }

    /// Foreground only — never sets a background cell color.
    pub fn root_style(self) -> Style {
        Style::default().fg(self.fg)
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

/// Prefer the terminal’s colors, then the desktop preference, then dark.
fn detect_preferred_theme() -> ResolvedTheme {
    if let Some(theme) = from_colorfgbg() {
        return theme;
    }
    if let Some(theme) = from_os_preference() {
        return theme;
    }
    ResolvedTheme::Dark
}

/// `COLORFGBG` is set by many terminals as `fg;bg` palette indices.
fn from_colorfgbg() -> Option<ResolvedTheme> {
    let v = std::env::var("COLORFGBG").ok()?;
    let bg = v.split(';').next_back()?;
    let idx = bg.trim().parse::<u8>().ok()?;
    // Background 7 / 15 ≈ white/light; other common indices are dark.
    Some(if idx == 7 || idx == 15 {
        ResolvedTheme::Light
    } else {
        ResolvedTheme::Dark
    })
}

fn from_os_preference() -> Option<ResolvedTheme> {
    #[cfg(target_os = "macos")]
    {
        if let Some(theme) = from_macos() {
            return Some(theme);
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(theme) = from_windows() {
            return Some(theme);
        }
    }
    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        if let Some(theme) = from_linux() {
            return Some(theme);
        }
    }

    // GTK_THEME / GTK2_RC_FILES often include "dark" when apps force a dark theme.
    if let Ok(theme) = std::env::var("GTK_THEME")
        && theme.to_ascii_lowercase().contains("dark")
    {
        return Some(ResolvedTheme::Dark);
    }

    None
}

#[cfg(target_os = "macos")]
fn from_macos() -> Option<ResolvedTheme> {
    let output = Command::new("defaults")
        .args(["read", "-g", "AppleInterfaceStyle"])
        .output()
        .ok()?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().eq_ignore_ascii_case("Dark") {
            return Some(ResolvedTheme::Dark);
        }
    }
    // Missing key ⇒ Light on macOS.
    Some(ResolvedTheme::Light)
}

#[cfg(target_os = "windows")]
fn from_windows() -> Option<ResolvedTheme> {
    let output = Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
            "/v",
            "AppsUseLightTheme",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("0x1") {
        Some(ResolvedTheme::Light)
    } else if stdout.contains("0x0") {
        Some(ResolvedTheme::Dark)
    } else {
        None
    }
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn from_linux() -> Option<ResolvedTheme> {
    // GNOME / Cinnamon / COSMIC-style setting.
    if let Ok(output) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
        && output.status.success()
    {
        let stdout = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
        if stdout.contains("prefer-dark") {
            return Some(ResolvedTheme::Dark);
        }
        if stdout.contains("prefer-light") {
            return Some(ResolvedTheme::Light);
        }
        // "default" is ambiguous — fall through to portal / gtk-theme.
    }

    if let Ok(output) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "gtk-theme"])
        .output()
        && output.status.success()
    {
        let stdout = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
        if stdout.contains("dark") {
            return Some(ResolvedTheme::Dark);
        }
    }

    // XDG Desktop Portal appearance (works across desktops when a portal is running).
    if let Ok(output) = Command::new("dbus-send")
        .args([
            "--session",
            "--print-reply=literal",
            "--dest=org.freedesktop.portal.Desktop",
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.Settings.ReadOne",
            "string:org.freedesktop.appearance",
            "string:color-scheme",
        ])
        .output()
        && output.status.success()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Reply looks like: `   variant       uint32 1` (1=dark, 2=light, 0=no pref).
        if let Some(theme) = parse_portal_color_scheme(&stdout) {
            return Some(theme);
        }
    }

    None
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn parse_portal_color_scheme(stdout: &str) -> Option<ResolvedTheme> {
    let tokens: Vec<&str> = stdout.split_whitespace().collect();
    for window in tokens.windows(2) {
        if window[0] == "uint32" {
            return match window[1] {
                "1" => Some(ResolvedTheme::Dark),
                "2" => Some(ResolvedTheme::Light),
                _ => None, // 0 = no preference
            };
        }
    }
    None
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
    fn default_mode_is_auto() {
        assert_eq!(ThemeMode::default(), ThemeMode::Auto);
    }

    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    #[test]
    fn parses_portal_reply() {
        assert_eq!(
            parse_portal_color_scheme("   variant       uint32 1"),
            Some(ResolvedTheme::Dark)
        );
        assert_eq!(
            parse_portal_color_scheme("variant uint32 2"),
            Some(ResolvedTheme::Light)
        );
        assert_eq!(parse_portal_color_scheme("variant uint32 0"), None);
    }
}
