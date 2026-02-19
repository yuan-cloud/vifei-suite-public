//! Shared visual tone tokens for Vifei TUI.
//!
//! These helpers keep status semantics consistent across lenses:
//! - success: stable/healthy outcome
//! - warning: degraded/attention-needed outcome
//! - error: failure/high-risk outcome
//! - info: neutral context identifiers
//! - accent: synthetic/special markers
//! - muted: helper text and metadata chrome

use crate::UiProfile;
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

pub fn panel_border_for(profile: UiProfile) -> Style {
    match profile {
        UiProfile::Standard => Style::default().fg(Color::Gray),
        UiProfile::Showcase => Style::default()
            .fg(Color::LightBlue)
            .add_modifier(Modifier::BOLD),
    }
}

pub fn selected_for(profile: UiProfile) -> Style {
    match profile {
        UiProfile::Standard => Style::default().add_modifier(Modifier::BOLD),
        UiProfile::Showcase => Style::default()
            .fg(Color::White)
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    }
}

pub fn info_for(profile: UiProfile) -> Style {
    match profile {
        UiProfile::Standard => info(),
        UiProfile::Showcase => Style::default()
            .fg(Color::LightCyan)
            .add_modifier(Modifier::BOLD),
    }
}

pub fn warning_for(profile: UiProfile) -> Style {
    match profile {
        UiProfile::Standard => warning(),
        UiProfile::Showcase => Style::default()
            .fg(Color::LightYellow)
            .add_modifier(Modifier::BOLD),
    }
}

pub fn accent_for(profile: UiProfile) -> Style {
    match profile {
        UiProfile::Standard => accent(),
        UiProfile::Showcase => Style::default()
            .fg(Color::LightMagenta)
            .add_modifier(Modifier::BOLD),
    }
}

pub fn muted_for(profile: UiProfile) -> Style {
    match profile {
        UiProfile::Standard => muted(),
        UiProfile::Showcase => Style::default().fg(Color::Gray),
    }
}
