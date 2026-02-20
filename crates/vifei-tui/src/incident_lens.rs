//! Incident Lens — default landing view for incident triage.
//!
//! The Incident Lens answers: "What happened, and what should I investigate first?"
//!
//! # Layout
//!
//! - Top: Action Now (anomalies needing triage first)
//! - Middle: Run context (which runs, status, event count)
//! - Bottom: Event breakdown (counts by type)
//!
//! # Constitution
//!
//! See `PLANS.md` § D5: "Correctness target: Deep investigation. Entry behavior: Incident triage."

use crate::{visual_tone, UiProfile};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};
use vifei_core::reducer::State;

/// Render the Incident Lens into the given area.
///
/// Displays run summaries, event breakdowns, and anomalies drawn from
/// the reducer State, plus contextual info from the App.
#[allow(dead_code)] // Compatibility wrapper; default profile path for direct tests.
pub fn render_incident_lens(
    frame: &mut Frame,
    area: Rect,
    state: &State,
    eventlog_path: &str,
    total_events: usize,
    show_onboarding: bool,
) {
    render_incident_lens_with_profile(
        frame,
        area,
        state,
        eventlog_path,
        total_events,
        show_onboarding,
        UiProfile::Standard,
    );
}

pub fn render_incident_lens_with_profile(
    frame: &mut Frame,
    area: Rect,
    state: &State,
    eventlog_path: &str,
    total_events: usize,
    show_onboarding: bool,
    profile: UiProfile,
) {
    let block = Block::default()
        .title(match profile {
            UiProfile::Standard => " Incident Lens (Tab to toggle) ",
            UiProfile::Showcase => " Incident Lens · Showcase · Tab toggle ",
        })
        .borders(Borders::ALL)
        .border_type(match profile {
            UiProfile::Standard => BorderType::Plain,
            UiProfile::Showcase => BorderType::Rounded,
        })
        .border_style(visual_tone::panel_border_for(profile));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if show_onboarding {
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(anomalies_height(state, inner.width)),
                Constraint::Length(run_summary_height(state)),
                Constraint::Length(event_breakdown_height(state)),
            ])
            .split(inner);

        render_onboarding_strip(frame, sections[0], profile);
        render_anomalies(frame, sections[1], state, profile);
        render_run_summary(
            frame,
            sections[2],
            state,
            eventlog_path,
            total_events,
            profile,
        );
        render_event_breakdown(frame, sections[3], state, profile);
    } else {
        // Split inner area into three sections: anomalies, run summary, event breakdown
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(anomalies_height(state, inner.width)),
                Constraint::Length(run_summary_height(state)),
                Constraint::Length(event_breakdown_height(state)),
            ])
            .split(inner);

        render_anomalies(frame, sections[0], state, profile);
        render_run_summary(
            frame,
            sections[1],
            state,
            eventlog_path,
            total_events,
            profile,
        );
        render_event_breakdown(frame, sections[2], state, profile);
    }
}

fn render_onboarding_strip(frame: &mut Frame, area: Rect, profile: UiProfile) {
    let lines = vec![
        Line::from(Span::styled(
            "First run: Tab switch lens | q quit",
            visual_tone::warning_for(profile).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "Forensic controls: j/k move, Enter expand",
            visual_tone::muted_for(profile),
        )),
    ];

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

/// Height needed for the run summary section.
fn run_summary_height(state: &State) -> u16 {
    // Header + one line per run + 1 blank line, minimum 3
    let runs = state.run_metadata.len() as u16;
    (2 + runs).max(3)
}

/// Height needed for the event breakdown section.
fn event_breakdown_height(state: &State) -> u16 {
    // Header + one line per event type + 1 blank line, minimum 3
    let types = state.event_counts_by_type.len() as u16;
    (2 + types).max(3)
}

/// Height needed for anomalies section.
fn anomalies_height(state: &State, width: u16) -> u16 {
    let count =
        state.error_log.len() + state.clock_skew_events.len() + state.policy_decisions.len();
    let anomaly_lines = (count as u16).max(1);
    let hint = next_action_line(count > 0, width);
    let hint_lines = wrapped_line_count(&hint, width);
    // Header + priority + anomalies + blank + next-action hint (possibly wrapped)
    (3 + anomaly_lines + 1 + hint_lines).max(6)
}

fn wrapped_line_count(text: &str, width: u16) -> u16 {
    let safe_width = width.max(1) as usize;
    let chars = text.chars().count();
    ((chars.saturating_sub(1) / safe_width) + 1) as u16
}

fn next_action_line(has_anomalies: bool, width: u16) -> String {
    let narrow = width <= 72;
    if has_anomalies {
        if narrow {
            "Next action: Tab->Forensic, j/k move, Enter expand. Keys: Tab=toggle, q=quit"
                .to_string()
        } else {
            "Next action: Tab to Forensic, then j/k + Enter on anomaly events. Keys: Tab=toggle lens, q=quit".to_string()
        }
    } else if narrow {
        "Next action: monitor Run Context; Tab->Forensic for event audit. Keys: Tab=toggle, q=quit"
            .to_string()
    } else {
        "Next action: monitor Run Context; Tab to Forensic for event-level audit. Keys: Tab=toggle lens, q=quit".to_string()
    }
}

/// Render the run summary section.
fn render_run_summary(
    frame: &mut Frame,
    area: Rect,
    state: &State,
    eventlog_path: &str,
    total_events: usize,
    profile: UiProfile,
) {
    let mut lines = vec![Line::from(vec![
        Span::styled("Run Context", visual_tone::header()),
        Span::raw("  "),
        Span::styled(
            format!("{} ({} events)", eventlog_path, total_events),
            visual_tone::muted_for(profile),
        ),
    ])];

    if state.run_metadata.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no runs)",
            visual_tone::muted_for(profile),
        )));
    } else {
        for (run_id, info) in &state.run_metadata {
            let status_span = if info.ended {
                match info.exit_code {
                    Some(0) => Span::styled("OK", visual_tone::success()),
                    Some(code) => Span::styled(format!("exit {}", code), visual_tone::error()),
                    None => Span::styled("ended", visual_tone::warning()),
                }
            } else {
                Span::styled("running", visual_tone::warning())
            };

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(&info.agent, visual_tone::info_for(profile)),
                Span::raw(format!(" ({})", run_id)),
                Span::raw(" ["),
                status_span,
                Span::raw(format!("] {} events", info.event_count)),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

/// Render the event type breakdown section.
fn render_event_breakdown(frame: &mut Frame, area: Rect, state: &State, profile: UiProfile) {
    let mut lines = vec![Line::from(Span::styled(
        "Event Breakdown (Context)",
        visual_tone::header(),
    ))];

    if state.event_counts_by_type.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no events)",
            visual_tone::muted_for(profile),
        )));
    } else {
        for (event_type, count) in &state.event_counts_by_type {
            let style = match event_type.as_str() {
                "Error" => visual_tone::error(),
                "ClockSkewDetected" => visual_tone::warning(),
                "PolicyDecision" | "RedactionApplied" => visual_tone::accent_for(profile),
                _ => Style::default(),
            };

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("{:<22}", event_type), style),
                Span::raw(format!("{:>6}", count)),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

/// Render the anomalies section (errors, clock skew, policy decisions).
fn render_anomalies(frame: &mut Frame, area: Rect, state: &State, profile: UiProfile) {
    let mut lines = vec![Line::from(Span::styled(
        "Action Now (Anomalies)",
        visual_tone::header(),
    ))];

    let has_anomalies = !state.error_log.is_empty()
        || !state.clock_skew_events.is_empty()
        || !state.policy_decisions.is_empty();

    lines.push(Line::from(vec![
        Span::styled("Priority:", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(format!(
            " ERR={} SKEW={} POLICY={}",
            state.error_log.len(),
            state.clock_skew_events.len(),
            state.policy_decisions.len()
        )),
    ]));

    if !has_anomalies {
        lines.push(Line::from(Span::styled(
            "  (none detected)",
            visual_tone::success(),
        )));
    } else {
        // Errors
        for err in &state.error_log {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("ERR ", visual_tone::error()),
                Span::raw(format!("@{}: ", err.commit_index)),
                Span::styled(&err.message, visual_tone::error()),
            ]));
        }

        // Clock skew
        for skew in &state.clock_skew_events {
            let delta_ms = skew.delta_ns / 1_000_000;
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("SKEW", visual_tone::warning()),
                Span::raw(format!(" @{}: {}ms backward", skew.commit_index, delta_ms)),
            ]));
        }

        // Policy decisions
        for pd in &state.policy_decisions {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("POLICY", visual_tone::accent_for(profile)),
                Span::raw(format!(
                    " @{}: {} → {} ({})",
                    pd.commit_index, pd.from_level, pd.to_level, pd.trigger
                )),
            ]));
        }
    }

    // Help line at the bottom
    lines.push(Line::from(""));
    let next_action = next_action_line(has_anomalies, area.width);
    lines.push(Line::from(Span::styled(
        next_action,
        visual_tone::muted_for(profile),
    )));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};
    use vifei_core::reducer::{ClockSkewEntry, ErrorEntry, PolicyTransition, RunInfo, State};

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

    fn empty_state() -> State {
        State::new()
    }

    fn populated_state() -> State {
        let mut state = State::new();
        state.run_metadata.insert(
            "run-001".to_string(),
            RunInfo {
                agent: "test-agent".to_string(),
                args: Some("--test".to_string()),
                ended: true,
                exit_code: Some(0),
                reason: Some("completed".to_string()),
                event_count: 10,
            },
        );
        state.event_counts_by_type.insert("RunStart".to_string(), 1);
        state.event_counts_by_type.insert("ToolCall".to_string(), 5);
        state
            .event_counts_by_type
            .insert("ToolResult".to_string(), 5);
        state.event_counts_by_type.insert("RunEnd".to_string(), 1);
        state.event_counts_by_type.insert("Error".to_string(), 2);
        state
    }

    #[test]
    fn incident_lens_renders_run_summary() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = populated_state();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                render_incident_lens(frame, area, &state, "test.jsonl", 12, false);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 100, 30));
        assert!(text.contains("Run Context"), "Missing Run Context header");
        assert!(
            text.contains("test-agent"),
            "Missing agent name in run summary"
        );
        assert!(text.contains("run-001"), "Missing run ID");
        assert!(text.contains("OK"), "Missing status for successful run");
        assert!(text.contains("10 events"), "Missing event count per run");
    }

    #[test]
    fn incident_lens_renders_event_breakdown() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = populated_state();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                render_incident_lens(frame, area, &state, "test.jsonl", 12, false);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 100, 30));
        assert!(
            text.contains("Event Breakdown"),
            "Missing Event Breakdown header"
        );
        assert!(text.contains("ToolCall"), "Missing ToolCall type");
        assert!(text.contains("Error"), "Missing Error type");
    }

    #[test]
    fn incident_lens_renders_anomalies_none() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = empty_state();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                render_incident_lens(frame, area, &state, "test.jsonl", 0, false);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 100, 30));
        assert!(
            text.contains("Action Now (Anomalies)"),
            "Missing Action Now header"
        );
        assert!(
            text.contains("none detected"),
            "Missing 'none detected' for empty anomalies"
        );
    }

    #[test]
    fn incident_lens_renders_errors() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = empty_state();
        state.error_log.push(ErrorEntry {
            commit_index: 42,
            kind: "runtime".to_string(),
            message: "tool crashed".to_string(),
            severity: Some("high".to_string()),
        });

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                render_incident_lens(frame, area, &state, "test.jsonl", 50, false);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 100, 30));
        assert!(text.contains("ERR"), "Missing ERR label");
        assert!(text.contains("@42"), "Missing commit_index for error");
        assert!(text.contains("tool crashed"), "Missing error message");
    }

    #[test]
    fn incident_lens_renders_clock_skew() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = empty_state();
        state.clock_skew_events.push(ClockSkewEntry {
            commit_index: 10,
            expected_ns: 2_000_000_000,
            actual_ns: 1_500_000_000,
            delta_ns: 500_000_000,
        });

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                render_incident_lens(frame, area, &state, "test.jsonl", 20, false);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 100, 30));
        assert!(text.contains("SKEW"), "Missing SKEW label");
        assert!(text.contains("@10"), "Missing commit_index for skew");
        assert!(text.contains("500ms"), "Missing delta for clock skew");
    }

    #[test]
    fn incident_lens_renders_policy_decisions() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = empty_state();
        state.policy_decisions.push(PolicyTransition {
            commit_index: 100,
            from_level: "L0".to_string(),
            to_level: "L1".to_string(),
            trigger: "queue_pressure".to_string(),
            queue_pressure_micro: 800_000,
        });

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                render_incident_lens(frame, area, &state, "test.jsonl", 200, false);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 100, 30));
        assert!(text.contains("POLICY"), "Missing POLICY label");
        assert!(text.contains("L0"), "Missing from_level");
        assert!(text.contains("L1"), "Missing to_level");
    }

    #[test]
    fn incident_lens_shows_failed_run() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = empty_state();
        state.run_metadata.insert(
            "run-fail".to_string(),
            RunInfo {
                agent: "failing-agent".to_string(),
                args: None,
                ended: true,
                exit_code: Some(1),
                reason: Some("error".to_string()),
                event_count: 5,
            },
        );

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                render_incident_lens(frame, area, &state, "test.jsonl", 5, false);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 100, 30));
        assert!(text.contains("exit 1"), "Missing exit code for failed run");
    }

    #[test]
    fn incident_lens_shows_running_run() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = empty_state();
        state.run_metadata.insert(
            "run-active".to_string(),
            RunInfo {
                agent: "active-agent".to_string(),
                args: None,
                ended: false,
                exit_code: None,
                reason: None,
                event_count: 3,
            },
        );

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                render_incident_lens(frame, area, &state, "test.jsonl", 3, false);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 100, 30));
        assert!(text.contains("running"), "Missing 'running' status");
    }

    #[test]
    fn incident_lens_shows_help_keys() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = empty_state();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                render_incident_lens(frame, area, &state, "test.jsonl", 0, false);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 100, 30));
        assert!(text.contains("Next action"), "Missing next-action hint");
        assert!(text.contains("Tab=toggle lens"), "Missing keybindings help");
        assert!(text.contains("q=quit"), "Missing quit keybinding");
    }

    #[test]
    fn incident_lens_next_action_changes_with_anomalies() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let clean_state = empty_state();
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                render_incident_lens(frame, area, &clean_state, "test.jsonl", 0, false);
            })
            .unwrap();
        let clean_text = buffer_text(&terminal, Rect::new(0, 0, 100, 30));
        assert!(
            clean_text.contains("monitor Run Context"),
            "Expected no-anomaly next action"
        );

        let mut anomaly_state = empty_state();
        anomaly_state.error_log.push(ErrorEntry {
            commit_index: 42,
            kind: "runtime".to_string(),
            message: "tool crashed".to_string(),
            severity: Some("high".to_string()),
        });
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                render_incident_lens(frame, area, &anomaly_state, "test.jsonl", 1, false);
            })
            .unwrap();
        let anomaly_text = buffer_text(&terminal, Rect::new(0, 0, 100, 30));
        assert!(
            anomaly_text.contains("j/k + Enter on anomaly events"),
            "Expected anomaly next action"
        );
    }

    #[test]
    fn incident_lens_narrow_keeps_next_action_hint_visible() {
        let backend = TestBackend::new(72, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = empty_state();
        state.error_log.push(ErrorEntry {
            commit_index: 7,
            kind: "runtime".into(),
            message: "boom".into(),
            severity: Some("high".into()),
        });

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 72, 24);
                render_incident_lens(frame, area, &state, "test.jsonl", 1, false);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 72, 24));
        assert!(
            text.contains("Next action: Tab->Forensic"),
            "Expected compact narrow next-action hint"
        );
    }

    #[test]
    fn incident_lens_orders_triage_before_context() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = populated_state();
        state.error_log.push(ErrorEntry {
            commit_index: 7,
            kind: "runtime".into(),
            message: "boom".into(),
            severity: Some("high".into()),
        });

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                render_incident_lens(frame, area, &state, "test.jsonl", 12, false);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 100, 30));
        let action_idx = text.find("Action Now (Anomalies)").unwrap();
        let context_idx = text.find("Run Context").unwrap();
        assert!(
            action_idx < context_idx,
            "Triage section must render before run context"
        );
    }

    #[test]
    fn incident_lens_shows_onboarding_strip_when_enabled() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = empty_state();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                render_incident_lens(frame, area, &state, "test.jsonl", 0, true);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 100, 30));
        assert!(text.contains("First run:"), "Missing onboarding title");
        assert!(
            text.contains("Forensic controls"),
            "Missing onboarding control hints"
        );
    }
}
