//! Shared visual tone tokens for Panopticon TUI.
//!
//! These helpers keep status semantics consistent across lenses:
//! - success: stable/healthy outcome
//! - warning: degraded/attention-needed outcome
//! - error: failure/high-risk outcome
//! - info: neutral context identifiers
//! - accent: synthetic/special markers
//! - muted: helper text and metadata chrome

use ratatui::style::{Color, Modifier, Style};

pub fn success() -> Style {
    Style::default().fg(Color::Green)
}

pub fn warning() -> Style {
    Style::default().fg(Color::Yellow)
}

pub fn error() -> Style {
    Style::default().fg(Color::Red)
}

pub fn info() -> Style {
    Style::default().fg(Color::Cyan)
}

pub fn accent() -> Style {
    Style::default().fg(Color::Magenta)
}

pub fn muted() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub fn header() -> Style {
    Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
}
