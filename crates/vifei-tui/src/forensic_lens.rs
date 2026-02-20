//! Forensic Lens — deep investigation view with timeline scrubber and event inspector.
//!
//! The Forensic Lens answers: "What exactly happened at this point in the run?"
//!
//! # Layout
//!
//! - Left: Timeline scrubber — navigate events by commit_index
//! - Right: Event inspector — full details for the selected event
//!
//! # Constitution
//!
//! See `PLANS.md` § D5: "Correctness target: Deep investigation."
//! Events ordered by commit_index (NEVER by timestamp — D6).

use crate::{visual_tone, UiProfile};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};
use vifei_core::event::{CommittedEvent, EventPayload};

/// Forensic Lens navigation and display state.
#[derive(Debug, Default)]
pub struct ForensicState {
    /// Index into the events list (NOT commit_index — this is the cursor position).
    pub cursor: usize,
    /// Whether the inspector pane is expanded (showing full details).
    pub expanded: bool,
}

impl ForensicState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Move cursor up by one.
    pub fn move_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Move cursor down by one, clamped to event count.
    pub fn move_down(&mut self, event_count: usize) {
        if event_count > 0 && self.cursor < event_count - 1 {
            self.cursor += 1;
        }
    }

    /// Toggle expanded/collapsed state.
    pub fn toggle_expand(&mut self) {
        self.expanded = !self.expanded;
    }
}

/// Render the Forensic Lens into the given area.
#[allow(dead_code)] // Compatibility wrapper; default profile path for direct tests.
pub fn render_forensic_lens(
    frame: &mut Frame,
    area: Rect,
    events: &[CommittedEvent],
    forensic: &ForensicState,
) {
    render_forensic_lens_with_profile(frame, area, events, forensic, UiProfile::Standard);
}

pub fn render_forensic_lens_with_profile(
    frame: &mut Frame,
    area: Rect,
    events: &[CommittedEvent],
    forensic: &ForensicState,
    profile: UiProfile,
) {
    let block = Block::default()
        .title(match profile {
            UiProfile::Standard => " Forensic Lens (Tab to toggle) ",
            UiProfile::Showcase => " Forensic Lens · Showcase · Tab toggle ",
        })
        .borders(Borders::ALL)
        .border_type(match profile {
            UiProfile::Standard => BorderType::Plain,
            UiProfile::Showcase => BorderType::Rounded,
        })
        .border_style(visual_tone::panel_border_for(profile));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if events.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  (no events)",
            visual_tone::muted_for(profile),
        )));
        frame.render_widget(empty, inner);
        return;
    }

    // Desktop gets side-by-side panes; narrow/mobile-like widths stack timeline above inspector.
    let columns = if inner.width >= 100 {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(inner)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(inner)
    };

    render_timeline(frame, columns[0], events, forensic, profile);
    render_inspector(frame, columns[1], events, forensic, profile);
}

/// Render the timeline scrubber (left pane).
fn render_timeline(
    frame: &mut Frame,
    area: Rect,
    events: &[CommittedEvent],
    forensic: &ForensicState,
    profile: UiProfile,
) {
    let block = Block::default()
        .title(match profile {
            UiProfile::Standard => " Timeline ",
            UiProfile::Showcase => " Timeline · j/k move · Enter expand ",
        })
        .borders(Borders::ALL)
        .border_type(match profile {
            UiProfile::Standard => BorderType::Plain,
            UiProfile::Showcase => BorderType::Rounded,
        })
        .border_style(visual_tone::panel_border_for(profile));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Calculate visible window around cursor
    let visible_height = inner.height as usize;
    let (start, end) = visible_window(forensic.cursor, events.len(), visible_height);

    let mut lines = Vec::with_capacity(end - start);
    for (i, ev) in events.iter().enumerate().take(end).skip(start) {
        let is_selected = i == forensic.cursor;

        let prefix = if is_selected { "▸ " } else { "  " };
        let synth_marker = if ev.synthesized { "[S] " } else { "" };
        let type_name = ev.payload.event_type_name();

        let line_style = if is_selected {
            visual_tone::selected_for(profile)
        } else {
            Style::default()
        };

        let type_color = event_type_color(type_name);

        lines.push(Line::from(vec![
            Span::styled(prefix, line_style),
            Span::styled(
                format!("{:>4} ", ev.commit_index),
                visual_tone::muted_for(profile),
            ),
            Span::styled(synth_marker, visual_tone::accent_for(profile)),
            Span::styled(type_name, Style::default().fg(type_color)),
        ]));
    }

    // Help line at bottom
    let selected = &events[forensic.cursor];
    let next_action = if forensic.expanded {
        format!(
            "Next: #{} {} | Enter=collapse | j/k",
            selected.commit_index,
            selected.payload.event_type_name()
        )
    } else {
        format!(
            "Next: #{} {} | Enter=expand | j/k",
            selected.commit_index,
            selected.payload.event_type_name()
        )
    };
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        next_action,
        visual_tone::muted_for(profile),
    )));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// Render the event inspector (right pane).
fn render_inspector(
    frame: &mut Frame,
    area: Rect,
    events: &[CommittedEvent],
    forensic: &ForensicState,
    profile: UiProfile,
) {
    let block = Block::default()
        .title(match profile {
            UiProfile::Standard => " Inspector ",
            UiProfile::Showcase => " Inspector · event details ",
        })
        .borders(Borders::ALL)
        .border_type(match profile {
            UiProfile::Standard => BorderType::Plain,
            UiProfile::Showcase => BorderType::Rounded,
        })
        .border_style(visual_tone::panel_border_for(profile));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if forensic.cursor >= events.len() {
        return;
    }

    let ev = &events[forensic.cursor];
    let mut lines = Vec::new();

    // Header: commit_index and event type
    lines.push(Line::from(vec![
        Span::styled("Event #", Style::default().fg(Color::White)),
        Span::styled(
            format!("{}", ev.commit_index),
            visual_tone::info_for(profile).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            ev.payload.event_type_name(),
            Style::default().fg(event_type_color(ev.payload.event_type_name())),
        ),
        if ev.synthesized {
            Span::styled("  [SYNTHESIZED]", visual_tone::accent_for(profile))
        } else {
            Span::raw("")
        },
    ]));
    lines.push(Line::from(""));

    // Metadata
    lines.push(Line::from(vec![
        Span::styled("  run_id:   ", visual_tone::muted_for(profile)),
        Span::raw(&ev.run_id),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  event_id: ", visual_tone::muted_for(profile)),
        Span::raw(&ev.event_id),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  tier:     ", visual_tone::muted_for(profile)),
        Span::raw(format!("{}", ev.tier)),
    ]));

    if let Some(ref pr) = ev.payload_ref {
        lines.push(Line::from(vec![
            Span::styled("  blob_ref: ", visual_tone::muted_for(profile)),
            Span::styled(pr, Style::default().fg(Color::Blue)),
        ]));
    }
    lines.push(Line::from(""));

    // Payload details (always shown when selected; expanded shows more)
    render_payload_details(&mut lines, &ev.payload, forensic.expanded);

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

/// Render payload-specific details into the lines buffer.
fn render_payload_details<'a>(
    lines: &mut Vec<Line<'a>>,
    payload: &'a EventPayload,
    expanded: bool,
) {
    let label_style = visual_tone::muted();

    match payload {
        EventPayload::RunStart { agent, args } => {
            lines.push(Line::from(vec![
                Span::styled("  agent: ", label_style),
                Span::styled(agent, visual_tone::info()),
            ]));
            if let Some(a) = args {
                lines.push(Line::from(vec![
                    Span::styled("  args:  ", label_style),
                    Span::raw(truncate_or_full(a, expanded)),
                ]));
            }
        }

        EventPayload::RunEnd { exit_code, reason } => {
            if let Some(code) = exit_code {
                let code_style = if *code == 0 {
                    visual_tone::success()
                } else {
                    visual_tone::error()
                };
                lines.push(Line::from(vec![
                    Span::styled("  exit_code: ", label_style),
                    Span::styled(format!("{}", code), code_style),
                ]));
            }
            if let Some(r) = reason {
                lines.push(Line::from(vec![
                    Span::styled("  reason:    ", label_style),
                    Span::raw(truncate_or_full(r, expanded)),
                ]));
            }
        }

        EventPayload::ToolCall { tool, args } => {
            lines.push(Line::from(vec![
                Span::styled("  tool: ", label_style),
                Span::styled(tool, visual_tone::info()),
            ]));
            if let Some(a) = args {
                lines.push(Line::from(vec![
                    Span::styled("  args: ", label_style),
                    Span::raw(truncate_or_full(a, expanded)),
                ]));
            }
        }

        EventPayload::ToolResult {
            tool,
            result,
            status,
        } => {
            lines.push(Line::from(vec![
                Span::styled("  tool:   ", label_style),
                Span::styled(tool, visual_tone::info()),
            ]));
            if let Some(s) = status {
                let status_style = if s == "success" || s == "ok" {
                    visual_tone::success()
                } else {
                    visual_tone::warning()
                };
                lines.push(Line::from(vec![
                    Span::styled("  status: ", label_style),
                    Span::styled(s, status_style),
                ]));
            }
            if let Some(r) = result {
                lines.push(Line::from(vec![
                    Span::styled("  result: ", label_style),
                    Span::raw(truncate_or_full(r, expanded)),
                ]));
            }
        }

        EventPayload::PolicyDecision {
            from_level,
            to_level,
            trigger,
            queue_pressure,
        } => {
            lines.push(Line::from(vec![
                Span::styled("  transition: ", label_style),
                Span::styled(from_level, visual_tone::warning()),
                Span::raw(" → "),
                Span::styled(to_level, visual_tone::warning()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  trigger:    ", label_style),
                Span::raw(trigger.as_str()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  pressure:   ", label_style),
                Span::raw(format!("{:.1}%", queue_pressure * 100.0)),
            ]));
        }

        EventPayload::RedactionApplied {
            target_event_id,
            field_path,
            reason,
        } => {
            lines.push(Line::from(vec![
                Span::styled("  target: ", label_style),
                Span::raw(target_event_id.as_str()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  field:  ", label_style),
                Span::raw(field_path.as_str()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  reason: ", label_style),
                Span::raw(truncate_or_full(reason, expanded)),
            ]));
        }

        EventPayload::Error {
            kind,
            message,
            severity,
        } => {
            lines.push(Line::from(vec![
                Span::styled("  kind:     ", label_style),
                Span::raw(kind.as_str()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  message:  ", label_style),
                Span::styled(truncate_or_full(message, expanded), visual_tone::error()),
            ]));
            if let Some(s) = severity {
                lines.push(Line::from(vec![
                    Span::styled("  severity: ", label_style),
                    Span::raw(s.as_str()),
                ]));
            }
        }

        EventPayload::ClockSkewDetected {
            expected_ns,
            actual_ns,
            delta_ns,
        } => {
            lines.push(Line::from(vec![
                Span::styled("  expected: ", label_style),
                Span::raw(format!("{}ns", expected_ns)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  actual:   ", label_style),
                Span::raw(format!("{}ns", actual_ns)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  delta:    ", label_style),
                Span::styled(
                    format!("{}ms backward", delta_ns / 1_000_000),
                    visual_tone::warning(),
                ),
            ]));
        }

        EventPayload::Generic { event_type, data } => {
            lines.push(Line::from(vec![
                Span::styled("  type: ", label_style),
                Span::raw(event_type.as_str()),
            ]));
            if expanded {
                for (k, v) in data {
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {}: ", k), label_style),
                        Span::raw(v.as_str()),
                    ]));
                }
            } else if !data.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("  data: ", label_style),
                    Span::raw(format!("{} fields (Enter to expand)", data.len())),
                ]));
            }
        }
    }

    if !expanded {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Press Enter to expand details",
            visual_tone::muted(),
        )));
    }
}

/// Color for event type names in the timeline.
fn event_type_color(type_name: &str) -> Color {
    match type_name {
        "RunStart" | "RunEnd" => Color::Cyan,
        "ToolCall" | "ToolResult" => Color::White,
        "PolicyDecision" => Color::Magenta,
        "RedactionApplied" => Color::Magenta,
        "Error" => Color::Red,
        "ClockSkewDetected" => Color::Yellow,
        _ => Color::Gray,
    }
}

/// Truncate text unless expanded. Uses char boundaries to avoid UTF-8 panics.
fn truncate_or_full(s: &str, expanded: bool) -> String {
    if expanded || s.len() <= 60 {
        s.to_string()
    } else {
        // Find the last char boundary at or before byte 59
        let end = floor_char_boundary(s, 59);
        format!("{}…", &s[..end])
    }
}

/// Find the largest byte index `<= pos` that is a valid char boundary.
fn floor_char_boundary(s: &str, pos: usize) -> usize {
    if pos >= s.len() {
        return s.len();
    }
    let mut i = pos;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Calculate the visible window of events around the cursor.
fn visible_window(cursor: usize, total: usize, height: usize) -> (usize, usize) {
    // Reserve 2 lines for the help text at the bottom
    let usable = height.saturating_sub(2);
    if usable == 0 || total == 0 {
        return (0, 0);
    }

    let half = usable / 2;
    let start = cursor.saturating_sub(half);
    let end = (start + usable).min(total);
    let start = if end == total {
        end.saturating_sub(usable)
    } else {
        start
    };

    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};
    use vifei_core::event::{CommittedEvent, EventPayload, Tier};

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

    fn test_event(index: u64, payload: EventPayload, synthesized: bool) -> CommittedEvent {
        CommittedEvent {
            commit_index: index,
            run_id: "run-1".into(),
            event_id: format!("e-{}", index),
            source_id: "test".into(),
            source_seq: Some(index),
            timestamp_ns: index * 1_000_000_000,
            tier: Tier::A,
            payload,
            payload_ref: None,
            synthesized,
        }
    }

    fn sample_events() -> Vec<CommittedEvent> {
        vec![
            test_event(
                0,
                EventPayload::RunStart {
                    agent: "test-agent".into(),
                    args: Some("--verbose".into()),
                },
                false,
            ),
            test_event(
                1,
                EventPayload::ToolCall {
                    tool: "read_file".into(),
                    args: Some("/etc/config".into()),
                },
                false,
            ),
            test_event(
                2,
                EventPayload::ToolResult {
                    tool: "read_file".into(),
                    result: Some("contents here".into()),
                    status: Some("success".into()),
                },
                false,
            ),
            test_event(
                3,
                EventPayload::Error {
                    kind: "runtime".into(),
                    message: "something failed".into(),
                    severity: Some("high".into()),
                },
                true,
            ),
            test_event(
                4,
                EventPayload::RunEnd {
                    exit_code: Some(0),
                    reason: Some("completed".into()),
                },
                false,
            ),
        ]
    }

    #[test]
    fn forensic_lens_renders_timeline() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let events = sample_events();
        let state = ForensicState::new();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 120, 30);
                render_forensic_lens(frame, area, &events, &state);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 120, 30));
        assert!(text.contains("Timeline"), "Missing Timeline title");
        assert!(text.contains("RunStart"), "Missing RunStart in timeline");
        assert!(text.contains("ToolCall"), "Missing ToolCall in timeline");
        assert!(text.contains("Error"), "Missing Error in timeline");
        assert!(text.contains("Next:"), "Missing next-action hint");
    }

    #[test]
    fn forensic_lens_hint_changes_with_expand_state() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let events = sample_events();

        let collapsed = ForensicState::new();
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 120, 30);
                render_forensic_lens(frame, area, &events, &collapsed);
            })
            .unwrap();
        let collapsed_text = buffer_text(&terminal, Rect::new(0, 0, 120, 30));
        assert!(
            collapsed_text.contains("Enter=expand"),
            "Expected expand hint when collapsed"
        );

        let expanded = ForensicState {
            cursor: 0,
            expanded: true,
        };
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 120, 30);
                render_forensic_lens(frame, area, &events, &expanded);
            })
            .unwrap();
        let expanded_text = buffer_text(&terminal, Rect::new(0, 0, 120, 30));
        assert!(
            expanded_text.contains("Enter=collapse"),
            "Expected collapse hint when expanded"
        );
    }

    #[test]
    fn forensic_lens_renders_inspector() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let events = sample_events();
        let state = ForensicState::new();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 120, 30);
                render_forensic_lens(frame, area, &events, &state);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 120, 30));
        assert!(text.contains("Inspector"), "Missing Inspector title");
        assert!(text.contains("Event #"), "Missing Event header");
        assert!(text.contains("test-agent"), "Missing agent in inspector");
    }

    #[test]
    fn forensic_lens_synthesized_marker() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let events = sample_events();
        // Event at index 3 is synthesized
        let state = ForensicState {
            cursor: 3,
            expanded: false,
        };

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 120, 30);
                render_forensic_lens(frame, area, &events, &state);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 120, 30));
        assert!(
            text.contains("[S]"),
            "Missing [S] marker for synthesized event"
        );
        assert!(
            text.contains("SYNTHESIZED"),
            "Missing SYNTHESIZED label in inspector"
        );
    }

    #[test]
    fn forensic_lens_navigation() {
        let mut state = ForensicState::new();
        assert_eq!(state.cursor, 0);

        state.move_down(5);
        assert_eq!(state.cursor, 1);

        state.move_down(5);
        assert_eq!(state.cursor, 2);

        state.move_up();
        assert_eq!(state.cursor, 1);

        // Cannot go below 0
        state.move_up();
        state.move_up();
        assert_eq!(state.cursor, 0);

        // Cannot exceed event count
        state.cursor = 4;
        state.move_down(5);
        assert_eq!(state.cursor, 4);
    }

    #[test]
    fn forensic_lens_expand_collapse() {
        let mut state = ForensicState::new();
        assert!(!state.expanded);

        state.toggle_expand();
        assert!(state.expanded);

        state.toggle_expand();
        assert!(!state.expanded);
    }

    #[test]
    fn forensic_lens_empty_events() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let events: Vec<CommittedEvent> = vec![];
        let state = ForensicState::new();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 120, 30);
                render_forensic_lens(frame, area, &events, &state);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 120, 30));
        assert!(text.contains("no events"), "Missing empty state message");
    }

    #[test]
    fn forensic_lens_shows_blob_ref() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut ev = test_event(
            0,
            EventPayload::ToolCall {
                tool: "write_file".into(),
                args: None,
            },
            false,
        );
        ev.payload_ref = Some("abc123def456".into());
        let events = vec![ev];
        let state = ForensicState::new();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 120, 30);
                render_forensic_lens(frame, area, &events, &state);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 120, 30));
        assert!(text.contains("blob_ref"), "Missing blob_ref label");
        assert!(text.contains("abc123def456"), "Missing blob ref value");
    }

    #[test]
    fn forensic_lens_policy_decision_details() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let events = vec![test_event(
            0,
            EventPayload::PolicyDecision {
                from_level: "L0".into(),
                to_level: "L1".into(),
                trigger: "queue_pressure".into(),
                queue_pressure: 0.85,
            },
            false,
        )];
        let state = ForensicState::new();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 120, 30);
                render_forensic_lens(frame, area, &events, &state);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 120, 30));
        assert!(text.contains("L0"), "Missing from_level");
        assert!(text.contains("L1"), "Missing to_level");
        assert!(text.contains("queue_pressure"), "Missing trigger");
        assert!(text.contains("85.0%"), "Missing pressure percentage");
    }

    #[test]
    fn forensic_lens_clock_skew_details() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let events = vec![test_event(
            0,
            EventPayload::ClockSkewDetected {
                expected_ns: 2_000_000_000,
                actual_ns: 1_500_000_000,
                delta_ns: 500_000_000,
            },
            false,
        )];
        let state = ForensicState::new();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 120, 30);
                render_forensic_lens(frame, area, &events, &state);
            })
            .unwrap();

        let text = buffer_text(&terminal, Rect::new(0, 0, 120, 30));
        assert!(text.contains("500ms"), "Missing delta display");
    }

    #[test]
    fn visible_window_basic() {
        assert_eq!(visible_window(0, 10, 7), (0, 5));
        assert_eq!(visible_window(4, 10, 7), (2, 7));
        assert_eq!(visible_window(9, 10, 7), (5, 10));
    }

    #[test]
    fn visible_window_empty() {
        assert_eq!(visible_window(0, 0, 10), (0, 0));
        assert_eq!(visible_window(0, 5, 0), (0, 0));
    }

    #[test]
    fn truncate_ascii_short_unchanged() {
        assert_eq!(truncate_or_full("hello", false), "hello");
    }

    #[test]
    fn truncate_ascii_long_truncated() {
        let long = "a".repeat(100);
        let result = truncate_or_full(&long, false);
        assert!(result.ends_with('…'));
        assert!(result.len() < 100);
    }

    #[test]
    fn truncate_expanded_returns_full() {
        let long = "a".repeat(100);
        assert_eq!(truncate_or_full(&long, true), long);
    }

    #[test]
    fn truncate_utf8_emoji_no_panic() {
        // Each emoji is 4 bytes. 15 emojis = 60 bytes, so 16 = 64 bytes triggers truncation.
        let emojis = "🦀".repeat(16);
        let result = truncate_or_full(&emojis, false);
        // Must not panic, must end with ellipsis, must be valid UTF-8
        assert!(result.ends_with('…'));
        // The truncated portion must contain only whole emoji characters
        let without_ellipsis = &result[..result.len() - '…'.len_utf8()];
        assert!(without_ellipsis.chars().all(|c| c == '🦀'));
    }

    #[test]
    fn truncate_utf8_cjk_no_panic() {
        // CJK characters are 3 bytes each. 21 chars = 63 bytes triggers truncation.
        let cjk = "漢".repeat(21);
        let result = truncate_or_full(&cjk, false);
        assert!(result.ends_with('…'));
        let without_ellipsis = &result[..result.len() - '…'.len_utf8()];
        assert!(without_ellipsis.chars().all(|c| c == '漢'));
    }

    #[test]
    fn truncate_utf8_mixed_no_panic() {
        // Mix of ASCII and multi-byte to stress byte boundary logic
        let mixed = format!("{}{}", "abc", "é".repeat(30));
        let result = truncate_or_full(&mixed, false);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn floor_char_boundary_basics() {
        let s = "hello";
        assert_eq!(floor_char_boundary(s, 3), 3);
        assert_eq!(floor_char_boundary(s, 100), 5);

        // 🦀 = 4 bytes at positions 0..4
        let crab = "🦀x";
        assert_eq!(floor_char_boundary(crab, 0), 0);
        assert_eq!(floor_char_boundary(crab, 1), 0); // mid-char, backs up to 0
        assert_eq!(floor_char_boundary(crab, 2), 0);
        assert_eq!(floor_char_boundary(crab, 3), 0);
        assert_eq!(floor_char_boundary(crab, 4), 4); // start of 'x'
    }
}
