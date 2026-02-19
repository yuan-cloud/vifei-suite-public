# TUI Ratatui Guide

Patterns for building the Vifei TUI with Ratatui v0.30.0.

**Crate versions:** ratatui 0.30.0, crossterm 0.28/0.29

**MSRV:** Rust 1.86.0 (ratatui requirement)

---

## v0.30.0 key changes

- **Modular workspace:** Split into `ratatui`, `ratatui-core`, `ratatui-widgets`.
  Depend on the main `ratatui` crate (re-exports everything).
- **`Alignment` renamed to `HorizontalAlignment`** (type alias kept for compat).
- **`Frame::size()` deprecated** — use `Frame::area()` instead.
- **`ratatui::run()`** — new convenience method for simple apps.
- **Crossterm feature flags:** `crossterm_0_28` and `crossterm_0_29`.

---

## Architecture: Flux / unidirectional data flow

Vifei's data flow maps directly to Ratatui's recommended Flux pattern:

```text
EventLog → Reducer → State → Projection → ViewModel → TUI render
                                                         │
                                                    user input
                                                         │
                                                         ▼
                                                    Action dispatch
```

The TUI is a **pure renderer of ViewModel**. It does not access the
EventLog directly. It does not modify State. All truth flows one direction.

---

## Lens architecture

Two lenses, toggled by `Tab`:

### Incident Lens (default)

Run summary, top anomalies, high-level triage view. Entry point for
investigation. Shows aggregated data from ViewModel.

### Forensic Lens

Timeline scrubber plus event inspector. Shows individual events by
`commit_index` order. ToolCall, ToolResult, PolicyDecision, and
RedactionApplied events are inspectable. Synthesized events are visually
distinguished (see projection invariants in `docs/BACKPRESSURE_POLICY.md`).

---

## Truth HUD (always visible)

A small status strip visible in both lenses. Renders confession state
from ViewModel. Required fields are listed in `docs/BACKPRESSURE_POLICY.md`
§ "Projection invariants v0.1" — link there, do not duplicate here.

```rust
fn render_truth_hud(f: &mut Frame, area: Rect, vm: &ViewModel) {
    let spans = vec![
        Span::styled(format!(" {} ", vm.ladder_level), style_for_level(vm.ladder_level)),
        Span::raw(format!(" Agg:{} ", vm.aggregation_mode)),
        Span::raw(format!(" Q:{:.0}% ", vm.queue_pressure * 100.0)),
        Span::styled(
            format!(" Drops:{} ", vm.tier_a_drops),
            if vm.tier_a_drops > 0 { Style::default().fg(Color::Red) }
            else { Style::default().fg(Color::Green) }
        ),
        Span::raw(format!(" Export:{} ", vm.export_safety)),
        Span::raw(format!(" {} ", vm.projection_invariants_version)),
    ];
    let line = Line::from(spans);
    f.render_widget(Paragraph::new(line).block(Block::default()), area);
}
```

---

## Layout structure

```rust
use ratatui::layout::{Constraint, Direction, Layout};

fn layout(area: Rect) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Truth HUD (top)
            Constraint::Min(5),    // Main content (lens)
            Constraint::Length(1), // Status bar (bottom)
        ])
        .split(area);
    (chunks[0], chunks[1], chunks[2])
}
```

---

## Key widgets for Vifei

| Widget | Use |
|--------|-----|
| `Table` | Event list in Forensic Lens |
| `Paragraph` | Event detail inspector, run summary |
| `Block` | Lens framing with titles |
| `Gauge` / `LineGauge` | Queue pressure indicator in Truth HUD |
| `Tabs` | Incident / Forensic lens toggle indicator |
| `Scrollbar` | Timeline scrubbing in Forensic Lens |

---

## Simple app with ratatui::run()

```rust
use ratatui::{self, Frame};

fn main() -> std::io::Result<()> {
    ratatui::run(|frame: &mut Frame| {
        // Use frame.area() not frame.size()
        let area = frame.area();
        // render widgets...
    })
}
```

For the full Vifei TUI, use the manual terminal setup with event loop
for input handling, but `ratatui::run()` is useful for prototyping.

---

## Testing with TestBackend

```rust
use ratatui::backend::TestBackend;
use ratatui::Terminal;

#[test]
fn truth_hud_renders_correctly() {
    let backend = TestBackend::new(80, 1);
    let mut terminal = Terminal::new(backend).unwrap();

    let vm = /* create test ViewModel */;
    terminal.draw(|f| {
        render_truth_hud(f, f.area(), &vm);
    }).unwrap();

    let buffer = terminal.backend().buffer().clone();
    // Assert specific cells or snapshot the buffer
    assert!(buffer_contains(&buffer, "L0"));
    assert!(buffer_contains(&buffer, "Drops:0"));
}
```

### Snapshot testing pattern

Serialize the buffer to a string and compare against a golden file.
Changes to golden files must be reviewed — they represent intentional
UI changes.

---

## Input handling

```rust
use crossterm::event::{self, Event, KeyCode, KeyEvent};

fn handle_input(key: KeyEvent, app: &mut App) -> bool {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return false, // exit
        KeyCode::Tab => app.toggle_lens(),
        KeyCode::Up => app.scroll_up(),
        KeyCode::Down => app.scroll_down(),
        _ => {}
    }
    true // continue
}
```

---

## What NOT to include in ViewModel

The ViewModel is hashed for determinism. Exclude:
- Terminal size / dimensions
- Focus state / cursor position
- Cursor blink state
- Wall clock time
- Any randomness

These are rendering concerns handled at draw time, not ViewModel state.
