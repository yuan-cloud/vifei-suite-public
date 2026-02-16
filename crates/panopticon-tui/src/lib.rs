//! Panopticon TUI — terminal UI for viewing EventLogs.
//!
//! # Overview
//!
//! The TUI provides two lenses for investigating agent runs:
//! - **Incident Lens** (default): Run summary with top anomalies.
//! - **Forensic Lens**: Timeline scrubber with event inspector.
//!
//! # Architecture
//!
//! The TUI is strictly read-only. It NEVER writes to the EventLog.
//! The rendering pipeline is pure: ViewModel → terminal output.
//!
//! ```text
//! EventLog → reduce → State → project → ViewModel → render → terminal
//! ```
//!
//! # Invariants
//!
//! - **I2 (Deterministic projection):** ViewModel is deterministic.
//! - Truth HUD is always visible and confesses system state.

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use panopticon_core::{
    eventlog::read_eventlog,
    projection::{project, LadderLevel, ProjectionInvariants, ViewModel},
    reducer::{reduce, State},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io::{self, stdout};
use std::path::Path;
use std::time::Duration;

/// Which lens is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveLens {
    #[default]
    Incident,
    Forensic,
}

impl ActiveLens {
    /// Toggle between Incident and Forensic lens.
    pub fn toggle(&self) -> Self {
        match self {
            ActiveLens::Incident => ActiveLens::Forensic,
            ActiveLens::Forensic => ActiveLens::Incident,
        }
    }

    /// Display name for the lens.
    pub fn name(&self) -> &'static str {
        match self {
            ActiveLens::Incident => "Incident Lens",
            ActiveLens::Forensic => "Forensic Lens",
        }
    }
}

/// Application state for the TUI.
pub struct App {
    /// The ViewModel derived from the EventLog.
    pub viewmodel: ViewModel,
    /// Reducer state (kept for potential re-projection).
    pub state: State,
    /// Projection invariants.
    pub invariants: ProjectionInvariants,
    /// Currently active lens.
    pub active_lens: ActiveLens,
    /// Whether the application should quit.
    pub should_quit: bool,
    /// Path to the EventLog file.
    pub eventlog_path: String,
    /// Total events in the EventLog.
    pub total_events: usize,
}

impl App {
    /// Create a new App by loading an EventLog and reducing it.
    pub fn new(eventlog_path: &Path) -> io::Result<Self> {
        let events = read_eventlog(eventlog_path)?;
        let total_events = events.len();

        // Reduce all events to state
        let mut state = State::new();
        for event in &events {
            state = reduce(&state, event);
        }

        // Project to ViewModel
        let invariants = ProjectionInvariants::new();
        let viewmodel = project(&state, &invariants);

        Ok(App {
            viewmodel,
            state,
            invariants,
            active_lens: ActiveLens::Incident,
            should_quit: false,
            eventlog_path: eventlog_path.display().to_string(),
            total_events,
        })
    }

    /// Handle a key event.
    pub fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Tab => {
                self.active_lens = self.active_lens.toggle();
            }
            _ => {}
        }
    }

    /// Set degradation level and re-project.
    pub fn set_degradation_level(&mut self, level: LadderLevel) {
        self.invariants.degradation_level = level;
        self.viewmodel = project(&self.state, &self.invariants);
    }
}

/// Run the TUI viewer for an EventLog.
pub fn run_viewer(eventlog_path: &Path) -> io::Result<()> {
    // Set up panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // Initialize terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(eventlog_path)?;

    // Main event loop
    loop {
        // Render
        terminal.draw(|frame| render(frame, &app))?;

        // Handle events
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key.code);
                }
            }
        }

        // Check for quit
        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}

/// Render the application to a frame.
fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Layout: Truth HUD at bottom (3 lines), main content above
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let main_area = chunks[0];
    let hud_area = chunks[1];

    // Render main content based on active lens
    match app.active_lens {
        ActiveLens::Incident => render_incident_lens(frame, main_area, app),
        ActiveLens::Forensic => render_forensic_lens(frame, main_area, app),
    }

    // Render Truth HUD (always visible)
    render_truth_hud(frame, hud_area, app);
}

/// Render the Incident Lens (default view).
fn render_incident_lens(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Incident Lens (Tab to toggle) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build content
    let mut lines = vec![
        Line::from(vec![
            Span::styled("EventLog: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&app.eventlog_path),
        ]),
        Line::from(vec![
            Span::styled(
                "Total Events: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{}", app.total_events)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Tier A Event Summary:",
            Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
    ];

    // Add Tier A summaries
    if app.viewmodel.tier_a_summaries.is_empty() {
        lines.push(Line::from("  (no Tier A events)"));
    } else {
        for (event_type, count) in &app.viewmodel.tier_a_summaries {
            lines.push(Line::from(format!("  {}: {}", event_type, count)));
        }
    }

    // Add some spacing and help
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Keys: Tab=toggle lens, q=quit",
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

/// Render the Forensic Lens (timeline + inspector).
fn render_forensic_lens(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Forensic Lens (Tab to toggle) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Forensic lens content (stub for M6.3)
    let lines = vec![
        Line::from(Span::styled(
            "Timeline View",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!(
            "Viewing {} events by commit_index",
            app.total_events
        )),
        Line::from(""),
        Line::from(Span::styled(
            "[Forensic Lens details: M6.3]",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Keys: Tab=toggle lens, q=quit",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

/// Render the Truth HUD strip (always visible).
///
/// The Truth HUD must confess at minimum (from BACKPRESSURE_POLICY):
/// - Current ladder level
/// - Aggregation mode and bin size
/// - Queue pressure indicator
/// - Tier A drops counter
/// - Export safety state
/// - projection_invariants_version
fn render_truth_hud(frame: &mut Frame, area: Rect, app: &App) {
    let vm = &app.viewmodel;

    // Build HUD content
    let level_style = match vm.degradation_level {
        LadderLevel::L0 => Style::default().fg(Color::Green),
        LadderLevel::L1 | LadderLevel::L2 => Style::default().fg(Color::Yellow),
        LadderLevel::L3 | LadderLevel::L4 => Style::default().fg(Color::Red),
        LadderLevel::L5 => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    };

    let drops_style = if vm.tier_a_drops > 0 {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };

    let export_style = match vm.export_safety_state {
        panopticon_core::projection::ExportSafetyState::Unknown => Style::default().fg(Color::Gray),
        panopticon_core::projection::ExportSafetyState::Clean => Style::default().fg(Color::Green),
        panopticon_core::projection::ExportSafetyState::Dirty => Style::default().fg(Color::Red),
        panopticon_core::projection::ExportSafetyState::Refused => {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        }
    };

    let aggregation = match vm.aggregation_bin_size {
        Some(bin) => format!("{} (bin={})", vm.aggregation_mode, bin),
        None => vm.aggregation_mode.clone(),
    };

    let queue_pressure_pct = (vm.queue_pressure() * 100.0) as u32;
    let pressure_style = if queue_pressure_pct >= 80 {
        Style::default().fg(Color::Red)
    } else if queue_pressure_pct >= 50 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };

    let hud_line = Line::from(vec![
        Span::styled(" Level: ", Style::default().fg(Color::White)),
        Span::styled(format!("{}", vm.degradation_level), level_style),
        Span::raw(" | "),
        Span::styled("Agg: ", Style::default().fg(Color::White)),
        Span::raw(&aggregation),
        Span::raw(" | "),
        Span::styled("Pressure: ", Style::default().fg(Color::White)),
        Span::styled(format!("{}%", queue_pressure_pct), pressure_style),
        Span::raw(" | "),
        Span::styled("Drops: ", Style::default().fg(Color::White)),
        Span::styled(format!("{}", vm.tier_a_drops), drops_style),
        Span::raw(" | "),
        Span::styled("Export: ", Style::default().fg(Color::White)),
        Span::styled(format!("{}", vm.export_safety_state), export_style),
    ]);

    let version_line = Line::from(vec![
        Span::styled(" Version: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            &vm.projection_invariants_version,
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let block = Block::default()
        .title(" Truth HUD ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let paragraph = Paragraph::new(vec![hud_line, version_line]);
    frame.render_widget(paragraph, inner);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_lens_toggle() {
        let lens = ActiveLens::Incident;
        assert_eq!(lens.toggle(), ActiveLens::Forensic);
        assert_eq!(lens.toggle().toggle(), ActiveLens::Incident);
    }

    #[test]
    fn test_active_lens_name() {
        assert_eq!(ActiveLens::Incident.name(), "Incident Lens");
        assert_eq!(ActiveLens::Forensic.name(), "Forensic Lens");
    }

    #[test]
    fn test_active_lens_default() {
        assert_eq!(ActiveLens::default(), ActiveLens::Incident);
    }
}
