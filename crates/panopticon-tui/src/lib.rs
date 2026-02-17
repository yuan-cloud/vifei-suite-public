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
//!
//! ```text
//! EventLog → reduce → State → project → ViewModel
//!                       │                    │
//!            Incident Lens (domain)    Truth HUD (honesty)
//!            Forensic Lens (events)
//! ```
//!
//! - **Truth HUD** renders honesty metrics from `ViewModel` (projection output).
//! - **Incident Lens** renders domain data from `State` (reducer output).
//! - **Forensic Lens** renders event details from `Vec<CommittedEvent>`.
//!
//! All three data sources are deterministic (same EventLog → same output).
//!
//! # Invariants
//!
//! - **I2 (Deterministic projection):** ViewModel is deterministic.
//! - Truth HUD is always visible and confesses system state.

mod forensic_lens;
mod incident_lens;
mod truth_hud;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use panopticon_core::{
    event::CommittedEvent,
    eventlog::read_eventlog,
    projection::{project, LadderLevel, ProjectionInvariants, ViewModel},
    reducer::{reduce, State},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Frame, Terminal,
};
use std::io::{self, stdout};
use std::path::Path;
use std::time::Duration;

/// Which lens is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ActiveLens {
    #[default]
    Incident,
    Forensic,
}

impl ActiveLens {
    /// Toggle between Incident and Forensic lens.
    fn toggle(&self) -> Self {
        match self {
            ActiveLens::Incident => ActiveLens::Forensic,
            ActiveLens::Forensic => ActiveLens::Incident,
        }
    }

    /// Display name for the lens.
    #[allow(dead_code)] // Will be used when rendering lens name in UI
    fn name(&self) -> &'static str {
        match self {
            ActiveLens::Incident => "Incident Lens",
            ActiveLens::Forensic => "Forensic Lens",
        }
    }
}

/// Application state for the TUI.
struct App {
    /// The ViewModel derived from the EventLog.
    viewmodel: ViewModel,
    /// Reducer state — used by Incident Lens and re-projection.
    state: State,
    /// Projection invariants.
    #[allow(dead_code)] // Used by set_degradation_level
    invariants: ProjectionInvariants,
    /// Currently active lens.
    active_lens: ActiveLens,
    /// Whether the application should quit.
    should_quit: bool,
    /// Path to the EventLog file.
    eventlog_path: String,
    /// Total events in the EventLog.
    total_events: usize,
    /// Committed events for the Forensic Lens.
    events: Vec<CommittedEvent>,
    /// Forensic Lens navigation state.
    forensic_state: forensic_lens::ForensicState,
}

impl App {
    /// Create a new App by loading an EventLog and reducing it.
    fn new(eventlog_path: &Path) -> io::Result<Self> {
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
            events,
            forensic_state: forensic_lens::ForensicState::new(),
        })
    }

    /// Handle a key event. Accepts the full KeyEvent to support modifier keys (Ctrl-C).
    fn handle_key(&mut self, key: KeyEvent) {
        // Ctrl-C: clean exit (raw mode captures Ctrl-C as key event, not SIGINT)
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Tab => {
                self.active_lens = self.active_lens.toggle();
            }
            // Forensic Lens navigation (only active in Forensic mode)
            KeyCode::Char('j') | KeyCode::Down if self.active_lens == ActiveLens::Forensic => {
                self.forensic_state.move_down(self.events.len());
            }
            KeyCode::Char('k') | KeyCode::Up if self.active_lens == ActiveLens::Forensic => {
                self.forensic_state.move_up();
            }
            KeyCode::Enter if self.active_lens == ActiveLens::Forensic => {
                self.forensic_state.toggle_expand();
            }
            _ => {}
        }
    }

    /// Set degradation level and re-project.
    #[allow(dead_code)] // Will be used when user triggers level change via keybind
    fn set_degradation_level(&mut self, level: LadderLevel) {
        self.invariants.degradation_level = level;
        self.viewmodel = project(&self.state, &self.invariants);
    }
}

/// Render an EventLog to a buffer string for snapshot testing.
///
/// Exercises the full pipeline: read → reduce → project → render.
/// Used by integration tests to validate Truth HUD presence and wiring.
#[doc(hidden)]
pub fn render_to_buffer(eventlog_path: &Path, width: u16, height: u16) -> io::Result<String> {
    let app = App::new(eventlog_path)?;
    let backend = ratatui::backend::TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| render(frame, &app))?;

    let buf = terminal.backend().buffer();
    let mut text = String::new();
    for y in 0..height {
        for x in 0..width {
            text.push(buf[(x, y)].symbol().chars().next().unwrap_or(' '));
        }
    }
    Ok(text)
}

/// Render an EventLog in Incident Lens mode with line breaks for docs assets.
#[doc(hidden)]
pub fn render_incident_multiline(
    eventlog_path: &Path,
    width: u16,
    height: u16,
) -> io::Result<String> {
    let app = App::new(eventlog_path)?;
    render_multiline(&app, width, height)
}

/// Render an EventLog in Forensic Lens mode with line breaks for docs assets.
#[doc(hidden)]
pub fn render_forensic_multiline(
    eventlog_path: &Path,
    width: u16,
    height: u16,
) -> io::Result<String> {
    let mut app = App::new(eventlog_path)?;
    app.active_lens = ActiveLens::Forensic;
    render_multiline(&app, width, height)
}

/// Render an EventLog in Incident Lens mode with a forced degradation level.
#[doc(hidden)]
pub fn render_degraded_incident_multiline(
    eventlog_path: &Path,
    width: u16,
    height: u16,
    level: LadderLevel,
) -> io::Result<String> {
    let mut app = App::new(eventlog_path)?;
    app.set_degradation_level(level);
    render_multiline(&app, width, height)
}

fn render_multiline(app: &App, width: u16, height: u16) -> io::Result<String> {
    let backend = ratatui::backend::TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| render(frame, app))?;

    let buf = terminal.backend().buffer();
    let mut text = String::new();
    for y in 0..height {
        for x in 0..width {
            text.push(buf[(x, y)].symbol().chars().next().unwrap_or(' '));
        }
        if y + 1 < height {
            text.push('\n');
        }
    }
    Ok(text)
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
                    app.handle_key(key);
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

    // Layout: Truth HUD at bottom (4 lines: 2 borders + status line + version line)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(4)])
        .split(area);

    let main_area = chunks[0];
    let hud_area = chunks[1];

    // Render main content based on active lens
    match app.active_lens {
        ActiveLens::Incident => incident_lens::render_incident_lens(
            frame,
            main_area,
            &app.state,
            &app.eventlog_path,
            app.total_events,
        ),
        ActiveLens::Forensic => {
            forensic_lens::render_forensic_lens(frame, main_area, &app.events, &app.forensic_state)
        }
    }

    // Render Truth HUD (always visible, in both lenses)
    truth_hud::render_truth_hud(frame, hud_area, &app.viewmodel);
}

#[cfg(test)]
mod tests {
    use super::*;
    use panopticon_core::event::{EventPayload, ImportEvent, Tier};
    use panopticon_core::eventlog::EventLogWriter;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;

    fn make_test_event(id: &str, ts: u64) -> ImportEvent {
        ImportEvent {
            run_id: "run-1".into(),
            event_id: id.into(),
            source_id: "test".into(),
            source_seq: Some(0),
            timestamp_ns: ts,
            tier: Tier::A,
            payload: EventPayload::RunStart {
                agent: "test-agent".into(),
                args: None,
            },
            payload_ref: None,
            synthesized: false,
        }
    }

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

    /// Create a simple key press event (no modifiers).
    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    /// Create a key press event with Ctrl modifier.
    fn ctrl_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    /// Create a test App from a temporary eventlog with multiple events.
    fn test_app() -> (App, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jsonl");
        let mut writer = EventLogWriter::open(&path).unwrap();
        writer.append(make_test_event("e1", 1_000_000_000)).unwrap();
        writer.append(make_test_event("e2", 2_000_000_000)).unwrap();
        drop(writer);
        let app = App::new(&path).unwrap();
        (app, dir)
    }

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

    // --- Key handling tests ---

    #[test]
    fn handle_key_q_quits() {
        let (mut app, _dir) = test_app();
        assert!(!app.should_quit);
        app.handle_key(key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn handle_key_esc_quits() {
        let (mut app, _dir) = test_app();
        app.handle_key(key(KeyCode::Esc));
        assert!(app.should_quit);
    }

    #[test]
    fn handle_key_ctrl_c_quits() {
        let (mut app, _dir) = test_app();
        app.handle_key(ctrl_key('c'));
        assert!(app.should_quit);
    }

    #[test]
    fn handle_key_tab_toggles_lens() {
        let (mut app, _dir) = test_app();
        assert_eq!(app.active_lens, ActiveLens::Incident);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.active_lens, ActiveLens::Forensic);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.active_lens, ActiveLens::Incident);
    }

    #[test]
    fn tab_preserves_forensic_state() {
        let (mut app, _dir) = test_app();
        // Switch to Forensic, move cursor
        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.forensic_state.cursor, 1);

        // Toggle away and back
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.active_lens, ActiveLens::Incident);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.active_lens, ActiveLens::Forensic);

        // Cursor position preserved
        assert_eq!(app.forensic_state.cursor, 1);
    }

    #[test]
    fn forensic_nav_only_in_forensic_mode() {
        let (mut app, _dir) = test_app();
        // In Incident mode, j/k should not affect forensic state
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.forensic_state.cursor, 0);

        // Switch to Forensic, j moves cursor
        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.forensic_state.cursor, 1);
    }

    // --- Render tests ---

    #[test]
    fn truth_hud_visible_in_forensic_lens() {
        let (mut app, _dir) = test_app();
        app.active_lens = ActiveLens::Forensic;

        let backend = TestBackend::new(120, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();

        let hud_text = buffer_text(&terminal, Rect::new(0, 16, 120, 4));
        assert!(
            hud_text.contains("Level:"),
            "HUD must be visible in Forensic Lens"
        );
        assert!(
            hud_text.contains("Version:"),
            "HUD version must be visible in Forensic Lens"
        );
    }

    #[test]
    fn truth_hud_both_lines_visible() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jsonl");
        let mut writer = EventLogWriter::open(&path).unwrap();
        writer.append(make_test_event("e1", 1_000_000_000)).unwrap();
        drop(writer);

        let app = App::new(&path).unwrap();

        let backend = TestBackend::new(120, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|frame| render(frame, &app)).unwrap();

        // The Truth HUD occupies the bottom 4 rows (index 16..20)
        let hud_text = buffer_text(&terminal, Rect::new(0, 16, 120, 4));
        assert!(
            hud_text.contains("Level:"),
            "HUD status line must be visible, got: {}",
            hud_text
        );
        assert!(
            hud_text.contains("Version:"),
            "HUD version line must be visible (was clipped at Length(3)), got: {}",
            hud_text
        );
    }
}
