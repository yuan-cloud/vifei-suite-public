//! Truth HUD — always-visible status strip confessing system truthfulness state.
//!
//! The Truth HUD is the most important UI element. It is the system's
//! confession about its own state. If it says "0 drops" and L0, the user
//! can trust the display. If it shows L3 with pressure, the user knows
//! the display is degraded.
//!
//! # Required fields (from BACKPRESSURE_POLICY projection invariants)
//!
//! 1. Current degradation ladder level (L0..L5)
//! 2. Aggregation mode + bin size (e.g., "1:1", "10:1", "collapsed")
//! 3. Backlog / queue pressure indicator
//! 4. Tier A drops counter (must be 0)
//! 5. Export safety state: UNKNOWN, CLEAN, DIRTY, REFUSED
//! 6. Projection invariants version
//!
//! # Visibility rules
//!
//! - Always visible in BOTH lenses (Incident and Forensic).
//! - At L4 (Freeze UI), non-HUD panes may freeze, but Truth HUD remains live.

use crate::{visual_tone, UiProfile};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use vifei_core::projection::{ExportSafetyState, LadderLevel, ViewModel};

/// Color for the degradation ladder level indicator.
fn level_style(level: LadderLevel) -> Style {
    match level {
        LadderLevel::L0 => Style::default().fg(Color::Green),
        LadderLevel::L1 | LadderLevel::L2 | LadderLevel::L3 => Style::default().fg(Color::Yellow),
        LadderLevel::L4 => Style::default().fg(Color::Red),
        LadderLevel::L5 => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    }
}

/// Color for the Tier A drops counter.
fn drops_style(drops: u64) -> Style {
    if drops > 0 {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    }
}

/// Color for the export safety state.
fn export_style(state: ExportSafetyState) -> Style {
    match state {
        ExportSafetyState::Unknown => Style::default().fg(Color::Gray),
        ExportSafetyState::Clean => Style::default().fg(Color::Green),
        ExportSafetyState::Dirty => Style::default().fg(Color::Red),
        ExportSafetyState::Refused => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    }
}

/// Color for the queue pressure indicator.
fn pressure_style(pressure_pct: u32) -> Style {
    if pressure_pct >= 80 {
        Style::default().fg(Color::Red)
    } else if pressure_pct >= 50 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    }
}

/// Render the Truth HUD strip into the given area.
///
/// The Truth HUD confesses at minimum (per BACKPRESSURE_POLICY):
/// - Current ladder level
/// - Aggregation mode and bin size
/// - Queue pressure indicator
/// - Tier A drops counter
/// - Export safety state
/// - projection_invariants_version
#[allow(dead_code)] // Compatibility wrapper; default profile path for direct tests.
pub fn render_truth_hud(frame: &mut Frame, area: Rect, vm: &ViewModel) {
    render_truth_hud_with_profile(frame, area, vm, UiProfile::Standard);
}

pub fn render_truth_hud_with_profile(
    frame: &mut Frame,
    area: Rect,
    vm: &ViewModel,
    profile: UiProfile,
) {
    let aggregation = vm
        .aggregation_bin_size
        .map(|bin| format!("{} (bin={bin})", vm.aggregation_mode));

    let queue_pressure_pct = (vm.queue_pressure() * 100.0) as u32;

    let hud_line = Line::from(vec![
        Span::styled(" Level: ", Style::default().fg(Color::White)),
        Span::styled(
            format!("{}", vm.degradation_level),
            level_style(vm.degradation_level),
        ),
        Span::raw(" | "),
        Span::styled("Agg: ", Style::default().fg(Color::White)),
        match &aggregation {
            Some(text) => Span::raw(text.as_str()),
            None => Span::raw(vm.aggregation_mode.as_str()),
        },
        Span::raw(" | "),
        Span::styled("Pressure: ", Style::default().fg(Color::White)),
        Span::styled(
            format!("{}%", queue_pressure_pct),
            pressure_style(queue_pressure_pct),
        ),
        Span::raw(" | "),
        Span::styled("Drops: ", Style::default().fg(Color::White)),
        Span::styled(format!("{}", vm.tier_a_drops), drops_style(vm.tier_a_drops)),
        Span::raw(" | "),
        Span::styled("Export: ", Style::default().fg(Color::White)),
        Span::styled(
            format!("{}", vm.export_safety_state),
            export_style(vm.export_safety_state),
        ),
    ]);

    let version_line = Line::from(vec![
        Span::styled(" Version: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            &vm.projection_invariants_version,
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let block = Block::default()
        .title(match profile {
            UiProfile::Standard => " Truth HUD ",
            UiProfile::Showcase => " Truth HUD · Showcase · confession strip ",
        })
        .borders(Borders::ALL)
        .border_type(match profile {
            UiProfile::Standard => BorderType::Plain,
            UiProfile::Showcase => BorderType::Rounded,
        })
        .border_style(visual_tone::panel_border_for(profile));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let paragraph = Paragraph::new(vec![hud_line, version_line]);
    frame.render_widget(paragraph, inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};
    use std::collections::BTreeMap;
    use vifei_core::projection::ViewModel;

    /// Create a default ViewModel for testing.
    fn test_viewmodel() -> ViewModel {
        ViewModel {
            tier_a_summaries: BTreeMap::new(),
            aggregation_mode: "1:1".to_string(),
            aggregation_bin_size: None,
            degradation_level: LadderLevel::L0,
            queue_pressure_fixed: 0,
            tier_a_drops: 0,
            export_safety_state: ExportSafetyState::Unknown,
            projection_invariants_version: "projection-invariants-v0.1".to_string(),
        }
    }

    /// Extract the text content from a rendered buffer area.
    fn buffer_text(terminal: &Terminal<TestBackend>, area: Rect) -> String {
        let buf = terminal.backend().buffer();
        let mut text = String::new();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                text.push(buf[(x, y)].symbol().chars().next().unwrap_or(' '));
            }
        }
        text
    }

    #[test]
    fn truth_hud_renders_all_required_fields() {
        let backend = TestBackend::new(100, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let vm = test_viewmodel();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 5);
                render_truth_hud(frame, area, &vm);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 100, 5));

        // All 6 required fields must be present
        assert!(text.contains("Level:"), "Missing degradation level");
        assert!(text.contains("L0"), "Missing level value");
        assert!(text.contains("Agg:"), "Missing aggregation mode");
        assert!(text.contains("1:1"), "Missing aggregation value");
        assert!(text.contains("Pressure:"), "Missing pressure indicator");
        assert!(text.contains("0%"), "Missing pressure value");
        assert!(text.contains("Drops:"), "Missing drops counter");
        assert!(text.contains("Export:"), "Missing export safety state");
        assert!(text.contains("UNKNOWN"), "Missing export value");
        assert!(text.contains("Version:"), "Missing version label");
        assert!(
            text.contains("projection-invariants-v0.1"),
            "Missing version value"
        );
    }

    #[test]
    fn truth_hud_shows_degraded_level() {
        let backend = TestBackend::new(100, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut vm = test_viewmodel();
        vm.degradation_level = LadderLevel::L3;

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 5);
                render_truth_hud(frame, area, &vm);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 100, 5));
        assert!(text.contains("L3"), "Should display L3 degradation level");
    }

    #[test]
    fn truth_hud_shows_aggregation_with_bin_size() {
        let backend = TestBackend::new(100, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut vm = test_viewmodel();
        vm.aggregation_mode = "10:1".to_string();
        vm.aggregation_bin_size = Some(10);

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 5);
                render_truth_hud(frame, area, &vm);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 100, 5));
        assert!(text.contains("10:1"), "Should display aggregation mode");
        assert!(text.contains("bin=10"), "Should display bin size");
    }

    #[test]
    fn truth_hud_shows_nonzero_drops() {
        let backend = TestBackend::new(100, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut vm = test_viewmodel();
        vm.tier_a_drops = 5;

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 5);
                render_truth_hud(frame, area, &vm);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 100, 5));
        assert!(
            text.contains("Drops:") && text.contains('5'),
            "Should display nonzero drops count"
        );
    }

    #[test]
    fn truth_hud_shows_export_clean() {
        let backend = TestBackend::new(100, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut vm = test_viewmodel();
        vm.export_safety_state = ExportSafetyState::Clean;

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 5);
                render_truth_hud(frame, area, &vm);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 100, 5));
        assert!(text.contains("CLEAN"), "Should display CLEAN export state");
    }

    #[test]
    fn truth_hud_shows_queue_pressure() {
        let backend = TestBackend::new(100, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut vm = test_viewmodel();
        // Set 75% pressure (0.75 * 1_000_000 = 750_000)
        vm.queue_pressure_fixed = 750_000;

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 5);
                render_truth_hud(frame, area, &vm);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 100, 5));
        assert!(text.contains("75%"), "Should display 75% queue pressure");
    }

    #[test]
    fn level_style_colors() {
        // L0 should be green (healthy)
        assert_eq!(level_style(LadderLevel::L0).fg, Some(Color::Green));
        // L1, L2, L3 should be yellow (degraded)
        assert_eq!(level_style(LadderLevel::L1).fg, Some(Color::Yellow));
        assert_eq!(level_style(LadderLevel::L2).fg, Some(Color::Yellow));
        assert_eq!(level_style(LadderLevel::L3).fg, Some(Color::Yellow));
        // L4 should be red (critical)
        assert_eq!(level_style(LadderLevel::L4).fg, Some(Color::Red));
        // L5 should be bold red (safe failure posture)
        let l5 = level_style(LadderLevel::L5);
        assert_eq!(l5.fg, Some(Color::Red));
    }

    #[test]
    fn drops_style_colors() {
        // 0 drops = green
        assert_eq!(drops_style(0).fg, Some(Color::Green));
        // >0 drops = red
        assert_eq!(drops_style(1).fg, Some(Color::Red));
    }

    #[test]
    fn export_style_colors() {
        assert_eq!(
            export_style(ExportSafetyState::Unknown).fg,
            Some(Color::Gray)
        );
        assert_eq!(
            export_style(ExportSafetyState::Clean).fg,
            Some(Color::Green)
        );
        assert_eq!(export_style(ExportSafetyState::Dirty).fg, Some(Color::Red));
        assert_eq!(
            export_style(ExportSafetyState::Refused).fg,
            Some(Color::Red)
        );
    }

    #[test]
    fn pressure_style_thresholds() {
        // <50% = green
        assert_eq!(pressure_style(0).fg, Some(Color::Green));
        assert_eq!(pressure_style(49).fg, Some(Color::Green));
        // 50-79% = yellow
        assert_eq!(pressure_style(50).fg, Some(Color::Yellow));
        assert_eq!(pressure_style(79).fg, Some(Color::Yellow));
        // >=80% = red
        assert_eq!(pressure_style(80).fg, Some(Color::Red));
        assert_eq!(pressure_style(100).fg, Some(Color::Red));
    }
}
