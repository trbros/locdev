//! The app's colors ("warm carbon" dark, "paper" light), converted from the
//! oklch tokens in `apps/desktop/src/index.css`. Keep both in step.

use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Copy)]
pub struct Theme {
    /// Main panel background (`--background`).
    pub bg: Color,
    /// Sidebar and status bar (`--chrome`).
    pub chrome: Color,
    /// Section cards (`--card`).
    pub card: Color,
    /// Selected rows (`--accent`).
    pub accent: Color,
    pub fg: Color,
    pub muted: Color,
    /// Labels quieter than `muted`.
    pub dim: Color,
    pub border: Color,
    pub ember: Color,
    pub phosphor: Color,
    pub amber: Color,
    pub signal: Color,
    pub danger: Color,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

pub const DARK: Theme = Theme {
    bg: rgb(0x13, 0x10, 0x0e),
    chrome: rgb(0x0e, 0x0c, 0x0a),
    card: rgb(0x1a, 0x16, 0x14),
    accent: rgb(0x28, 0x23, 0x1f),
    fg: rgb(0xeb, 0xe7, 0xe1),
    muted: rgb(0x99, 0x91, 0x88),
    dim: rgb(0x5a, 0x54, 0x4e),
    border: rgb(0x2e, 0x2a, 0x27),
    ember: rgb(0xf6, 0x84, 0x43),
    phosphor: rgb(0x65, 0xdb, 0x7c),
    amber: rgb(0xf6, 0xb8, 0x4d),
    signal: rgb(0x57, 0xb8, 0xe3),
    danger: rgb(0xf4, 0x5b, 0x4e),
};

pub const LIGHT: Theme = Theme {
    bg: rgb(0xf9, 0xf6, 0xf2),
    chrome: rgb(0xed, 0xe9, 0xe2),
    card: rgb(0xfe, 0xfd, 0xfa),
    accent: rgb(0xe3, 0xdc, 0xd2),
    fg: rgb(0x24, 0x1e, 0x1a),
    muted: rgb(0x6a, 0x62, 0x5a),
    dim: rgb(0x9d, 0x97, 0x90),
    border: rgb(0xd8, 0xd3, 0xcd),
    ember: rgb(0xbd, 0x46, 0x00),
    phosphor: rgb(0x1b, 0x92, 0x47),
    amber: rgb(0xcb, 0x7f, 0x00),
    signal: rgb(0x00, 0x7e, 0xb0),
    danger: rgb(0xd0, 0x2d, 0x27),
};

impl Theme {
    pub fn text(&self) -> Style {
        Style::new().fg(self.fg)
    }
    pub fn muted(&self) -> Style {
        Style::new().fg(self.muted)
    }
    pub fn dim(&self) -> Style {
        Style::new().fg(self.dim)
    }
    pub fn bold(&self) -> Style {
        Style::new().fg(self.fg).add_modifier(Modifier::BOLD)
    }
    /// Section titles: small caps feel, like the app's eyebrows.
    pub fn heading(&self) -> Style {
        Style::new().fg(self.muted).add_modifier(Modifier::BOLD)
    }
    pub fn key(&self) -> Style {
        Style::new().fg(self.ember).add_modifier(Modifier::BOLD)
    }
}
