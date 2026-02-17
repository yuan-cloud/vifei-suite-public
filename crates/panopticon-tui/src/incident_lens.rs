//! Incident Lens — default landing view for incident triage.
//!
//! The Incident Lens answers: "What happened, and what should I investigate first?"
//!
//! # Layout
//!
//! - Top: Run summary (which runs, status, event count)
//! - Middle: Event type breakdown (counts by Tier A type)
//! - Bottom: Anomalies (errors, clock skew, policy decisions)
//!
//! # Constitution
//!
//! See `PLANS.md` § D5: "Correctness target: Deep investigation. Entry behavior: Incident triage."

use panopticon_core::reducer::State;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Render the Incident Lens into the given area.
///
/// Displays run summaries, event breakdowns, and anomalies drawn from
/// the reducer State, plus contextual info from the App.
pub fn render_incident_lens(
    frame: &mut Frame,
    area: Rect,
    state: &State,
    eventlog_path: &str,
    total_events: usize,
    show_onboarding: bool,
) {
    let block = Block::default()
        .title(" Incident Lens (Tab to toggle) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if show_onboarding {
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(run_summary_height(state)),
                Constraint::Length(event_breakdown_height(state)),
                Constraint::Min(3),
            ])
            .split(inner);

        render_onboarding_strip(frame, sections[0]);
        render_run_summary(frame, sections[1], state, eventlog_path, total_events);
        render_event_breakdown(frame, sections[2], state);
        render_anomalies(frame, sections[3], state);
    } else {
        // Split inner area into three sections: run summary, event breakdown, anomalies
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(run_summary_height(state)),
                Constraint::Length(event_breakdown_height(state)),
                Constraint::Min(3),
            ])
            .split(inner);

        render_run_summary(frame, sections[0], state, eventlog_path, total_events);
        render_event_breakdown(frame, sections[1], state);
        render_anomalies(frame, sections[2], state);
    }
}

fn render_onboarding_strip(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(Span::styled(
            "First run: Tab switch lens | q quit",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "Forensic controls: j/k move, Enter expand",
            Style::default().fg(Color::DarkGray),
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

/// Render the run summary section.
fn render_run_summary(
    frame: &mut Frame,
    area: Rect,
    state: &State,
    eventlog_path: &str,
    total_events: usize,
) {
    let mut lines = vec![Line::from(vec![
        Span::styled(
            "Run Summary",
            Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} ({} events)", eventlog_path, total_events),
            Style::default().fg(Color::DarkGray),
        ),
    ])];

    if state.run_metadata.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no runs)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (run_id, info) in &state.run_metadata {
            let status_span = if info.ended {
                match info.exit_code {
                    Some(0) => Span::styled("OK", Style::default().fg(Color::Green)),
                    Some(code) => {
                        Span::styled(format!("exit {}", code), Style::default().fg(Color::Red))
                    }
                    None => Span::styled("ended", Style::default().fg(Color::Yellow)),
                }
            } else {
                Span::styled("running", Style::default().fg(Color::Yellow))
            };

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(&info.agent, Style::default().fg(Color::Cyan)),
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
fn render_event_breakdown(frame: &mut Frame, area: Rect, state: &State) {
    let mut lines = vec![Line::from(Span::styled(
        "Event Breakdown",
        Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    ))];

    if state.event_counts_by_type.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no events)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (event_type, count) in &state.event_counts_by_type {
            let style = match event_type.as_str() {
                "Error" => Style::default().fg(Color::Red),
                "ClockSkewDetected" => Style::default().fg(Color::Yellow),
                "PolicyDecision" | "RedactionApplied" => Style::default().fg(Color::Magenta),
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
fn render_anomalies(frame: &mut Frame, area: Rect, state: &State) {
    let mut lines = vec![Line::from(Span::styled(
        "Anomalies",
        Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    ))];

    let has_anomalies = !state.error_log.is_empty()
        || !state.clock_skew_events.is_empty()
        || !state.policy_decisions.is_empty();

    if !has_anomalies {
        lines.push(Line::from(Span::styled(
            "  (none detected)",
            Style::default().fg(Color::Green),
        )));
    } else {
        // Errors
        for err in &state.error_log {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("ERR ", Style::default().fg(Color::Red)),
                Span::raw(format!("@{}: ", err.commit_index)),
                Span::styled(&err.message, Style::default().fg(Color::Red)),
            ]));
        }

        // Clock skew
        for skew in &state.clock_skew_events {
            let delta_ms = skew.delta_ns / 1_000_000;
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("SKEW", Style::default().fg(Color::Yellow)),
                Span::raw(format!(" @{}: {}ms backward", skew.commit_index, delta_ms)),
            ]));
        }

        // Policy decisions
        for pd in &state.policy_decisions {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("POLICY", Style::default().fg(Color::Magenta)),
                Span::raw(format!(
                    " @{}: {} → {} ({})",
                    pd.commit_index, pd.from_level, pd.to_level, pd.trigger
                )),
            ]));
        }
    }

    // Help line at the bottom
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Keys: Tab=toggle lens, q=quit",
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use panopticon_core::reducer::{ClockSkewEntry, ErrorEntry, PolicyTransition, RunInfo, State};
    use ratatui::{backend::TestBackend, Terminal};

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
        assert!(text.contains("Run Summary"), "Missing Run Summary header");
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
        assert!(text.contains("Anomalies"), "Missing Anomalies header");
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
        assert!(text.contains("Tab=toggle lens"), "Missing keybindings help");
        assert!(text.contains("q=quit"), "Missing quit keybinding");
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
