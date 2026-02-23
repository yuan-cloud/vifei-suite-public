use anyhow::{Context as AnyhowContext, Result};
use clap::Parser;
use ftui::layout::{Constraint, Flex};
use ftui::prelude::*;
use ftui::render::cell::PackedRgba;
use ftui::text::{Line, Span, Text};
use ftui::widgets::block::{Alignment, Block};
use ftui::widgets::borders::{BorderType, Borders};
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::rule::Rule;
use ftui::widgets::table::{Row, Table};
use ftui::widgets::{Badge, Widget};
use ftui::KeyEventKind;
use ftui_extras::glowing_text::GlowingText;
use ftui_extras::text_effects::{ColorGradient, StyledText, TextEffect};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;
use vifei_core::event::{CommittedEvent, EventPayload, Tier};
use vifei_core::eventlog::read_eventlog;
use web_time::Instant;

// ── Colors ──────────────────────────────────────────────────────────────
const BG_DEEP: PackedRgba = PackedRgba::rgb(18, 18, 30);
const FG_PRIMARY: PackedRgba = PackedRgba::rgb(220, 220, 230);
const FG_MUTED: PackedRgba = PackedRgba::rgb(120, 120, 140);
const FG_LABEL: PackedRgba = PackedRgba::rgb(140, 180, 220);
const ACCENT_GREEN: PackedRgba = PackedRgba::rgb(0, 255, 136);
const ACCENT_CYAN: PackedRgba = PackedRgba::rgb(100, 220, 255);
const ACCENT_YELLOW: PackedRgba = PackedRgba::rgb(255, 200, 60);
const ACCENT_RED: PackedRgba = PackedRgba::rgb(255, 80, 80);
const ACCENT_ORANGE: PackedRgba = PackedRgba::rgb(255, 160, 40);
const BORDER_COLOR: PackedRgba = PackedRgba::rgb(60, 60, 90);
const HASH_COLOR: PackedRgba = PackedRgba::rgb(180, 140, 255);
const HASH_GLOW: PackedRgba = PackedRgba::rgb(120, 80, 200);

const SPINNER_FRAMES: &[&str] = &[
    "\u{280b}", "\u{2819}", "\u{2839}", "\u{2838}", "\u{283c}", "\u{2834}", "\u{2826}", "\u{2827}",
    "\u{2807}", "\u{280f}",
];

#[derive(Parser, Debug)]
struct CommandLineArguments {
    #[arg(long)]
    events: PathBuf,

    /// Print cockpit summary to stdout and exit (no TUI)
    #[arg(long)]
    headless: bool,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
struct CockpitViewModel {
    total_events: u64,
    event_counts_by_type: BTreeMap<String, u64>,
    tier_a_events: u64,
    tier_a_dropped: u64,
    clock_skew_detected_count: u64,
    latest_error: Option<(u64, String, String)>,
}

fn blake3_hash_of_stable_json<T: serde::Serialize>(value: &T) -> Result<String> {
    let stable_json_bytes = serde_json::to_vec(value).context("serialize viewmodel")?;
    Ok(blake3::hash(&stable_json_bytes).to_hex().to_string())
}

fn build_cockpit_viewmodel(committed_events: &[CommittedEvent]) -> CockpitViewModel {
    let mut cockpit_viewmodel = CockpitViewModel::default();

    for event in committed_events {
        cockpit_viewmodel.total_events += 1;

        let payload_type_name = event.payload.event_type_name().to_string();
        *cockpit_viewmodel
            .event_counts_by_type
            .entry(payload_type_name.clone())
            .or_insert(0) += 1;

        if event.tier == Tier::A {
            cockpit_viewmodel.tier_a_events += 1;
        }

        if matches!(event.payload, EventPayload::ClockSkewDetected { .. }) {
            cockpit_viewmodel.clock_skew_detected_count += 1;
        }

        if let EventPayload::Error { kind, message, .. } = &event.payload {
            cockpit_viewmodel.latest_error =
                Some((event.commit_index, kind.clone(), message.clone()));
        }
    }

    cockpit_viewmodel
}

#[derive(Debug, Clone)]
enum Message {
    Terminal(Event),
}

impl From<Event> for Message {
    fn from(event: Event) -> Self {
        Self::Terminal(event)
    }
}

#[derive(Debug, Clone)]
struct FrankenTuiModel {
    cockpit_viewmodel: CockpitViewModel,
    cockpit_viewmodel_hash: String,
    start_time: Instant,
    tick_count: u64,
    quit: bool,
}

impl Model for FrankenTuiModel {
    type Message = Message;

    fn init(&mut self) -> Cmd<Self::Message> {
        Cmd::tick(Duration::from_millis(50))
    }

    fn update(&mut self, message: Self::Message) -> Cmd<Self::Message> {
        let cmd = match message {
            Message::Terminal(Event::Tick) => {
                self.tick_count = self.tick_count.wrapping_add(1);
                Cmd::none()
            }
            Message::Terminal(Event::Key(key_event)) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        self.quit = true;
                        return Cmd::quit();
                    }
                    KeyCode::Char('c') if key_event.modifiers.contains(Modifiers::CTRL) => {
                        self.quit = true;
                        return Cmd::quit();
                    }
                    _ => Cmd::none(),
                }
            }
            _ => Cmd::none(),
        };
        Cmd::batch(vec![cmd, Cmd::tick(Duration::from_millis(50))])
    }

    fn view(&self, frame: &mut Frame) {
        let area = frame.bounds();
        let time = self.start_time.elapsed().as_secs_f64();

        // Fill background
        Block::new()
            .style(Style::new().bg(BG_DEEP))
            .render(area, frame);

        // Outer bordered block
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Vifei Cockpit ")
            .title_alignment(Alignment::Center)
            .style(Style::new().fg(ACCENT_CYAN).bg(BG_DEEP));

        let inner = block.inner(area);
        block.render(area, frame);

        if inner.height < 5 || inner.width < 20 {
            return;
        }

        // Calculate dynamic sizes
        let event_type_count = self.cockpit_viewmodel.event_counts_by_type.len() as u16;
        let has_error = self.cockpit_viewmodel.latest_error.is_some();
        let error_height = if has_error { 4 } else { 0 };

        // Major vertical sections
        let sections = Flex::vertical()
            .constraints([
                Constraint::Fixed(3),                    // [0] Title + badge
                Constraint::Fixed(1),                    // [1] Rule: Truth Kernel Summary
                Constraint::Fixed(6),                    // [2] Metrics + hash
                Constraint::Fixed(1),                    // [3] Rule: Event Breakdown
                Constraint::Fixed(event_type_count + 1), // [4] Event table
                Constraint::Fixed(error_height),         // [5] Error section
                Constraint::Min(1),                      // [6] Footer
            ])
            .split(inner);

        // ── [0] Title with animated gradient + health badge ──
        self.render_title(frame, sections[0], time);

        // ── [1] Rule: Truth Kernel Summary ──
        Rule::new()
            .title("Truth Kernel Summary")
            .style(Style::new().fg(BORDER_COLOR))
            .render(sections[1], frame);

        // ── [2] Metrics + hash ──
        self.render_metrics(frame, sections[2]);

        // ── [3] Rule: Event Breakdown ──
        Rule::new()
            .title("Event Breakdown")
            .style(Style::new().fg(BORDER_COLOR))
            .render(sections[3], frame);

        // ── [4] Event table ──
        self.render_event_table(frame, sections[4]);

        // ── [5] Error section ──
        if has_error {
            self.render_error(frame, sections[5]);
        }

        // ── [6] Footer ──
        let uptime_secs = self.start_time.elapsed().as_secs();
        let mins = uptime_secs / 60;
        let secs = uptime_secs % 60;
        let spinner_char = SPINNER_FRAMES[(self.tick_count as usize) % SPINNER_FRAMES.len()];
        let footer = Paragraph::new(Text::from(Line::from_spans([
            Span::styled("  q ", Style::new().fg(FG_MUTED)),
            Span::styled("quit", Style::new().fg(FG_LABEL)),
            Span::styled("  \u{2502}  ", Style::new().fg(BORDER_COLOR)),
            Span::styled(
                format!("{} 20fps", spinner_char),
                Style::new().fg(ACCENT_GREEN),
            ),
            Span::styled("  \u{2502}  ", Style::new().fg(BORDER_COLOR)),
            Span::styled(format!("{}:{:02}", mins, secs), Style::new().fg(FG_MUTED)),
        ])));
        footer.render(sections[6], frame);
    }
}

// ── View helpers ────────────────────────────────────────────────────────

impl FrankenTuiModel {
    fn render_title(&self, frame: &mut Frame, area: ftui::core::geometry::Rect, time: f64) {
        // Split title area: spinner, gradient text, badge
        let cols = Flex::horizontal()
            .constraints([
                Constraint::Fixed(3),
                Constraint::Min(17),
                Constraint::Fixed(14),
            ])
            .split(area);

        // Spinner (centered vertically in 3-line area)
        let spinner_rows = Flex::vertical()
            .constraints([
                Constraint::Fixed(1),
                Constraint::Fixed(1),
                Constraint::Fixed(1),
            ])
            .split(cols[0]);

        let spinner_char = SPINNER_FRAMES[(self.tick_count as usize) % SPINNER_FRAMES.len()];
        let spinner = Paragraph::new(Text::from(Line::styled(
            format!(" {}", spinner_char),
            Style::new().fg(ACCENT_GREEN),
        )));
        spinner.render(spinner_rows[1], frame);

        // Center the title vertically in the 3-line area
        let title_rows = Flex::vertical()
            .constraints([
                Constraint::Fixed(1),
                Constraint::Fixed(1),
                Constraint::Fixed(1),
            ])
            .split(cols[1]);

        // Animated gradient title
        let gradient = ColorGradient::new(vec![
            (0.0, ACCENT_GREEN),
            (0.3, ACCENT_CYAN),
            (0.7, HASH_COLOR),
            (1.0, ACCENT_GREEN),
        ]);
        StyledText::new("VIFEI \u{00d7} FrankenTUI Cockpit")
            .effect(TextEffect::AnimatedGradient {
                gradient,
                speed: 0.3,
            })
            .bold()
            .time(time)
            .render(title_rows[1], frame);

        // Health badge (centered vertically)
        let badge_rows = Flex::vertical()
            .constraints([
                Constraint::Fixed(1),
                Constraint::Fixed(1),
                Constraint::Fixed(1),
            ])
            .split(cols[2]);

        let (badge_label, badge_style) = self.health_badge();
        Badge::new(badge_label)
            .with_style(badge_style)
            .with_padding(1, 1)
            .render(badge_rows[1], frame);
    }

    fn health_badge(&self) -> (&str, Style) {
        if self.cockpit_viewmodel.tier_a_dropped > 0 {
            ("DROPPED", Style::new().fg(BG_DEEP).bg(ACCENT_RED).bold())
        } else if self.cockpit_viewmodel.clock_skew_detected_count > 0 {
            ("SKEW", Style::new().fg(BG_DEEP).bg(ACCENT_YELLOW).bold())
        } else if self.cockpit_viewmodel.latest_error.is_some() {
            ("ERROR", Style::new().fg(BG_DEEP).bg(ACCENT_ORANGE).bold())
        } else {
            ("HEALTHY", Style::new().fg(BG_DEEP).bg(ACCENT_GREEN).bold())
        }
    }

    fn render_metrics(&self, frame: &mut Frame, area: ftui::core::geometry::Rect) {
        let total_str = self.cockpit_viewmodel.total_events.to_string();
        let tier_a_str = self.cockpit_viewmodel.tier_a_events.to_string();
        let skew_str = self.cockpit_viewmodel.clock_skew_detected_count.to_string();
        let dropped_str = self.cockpit_viewmodel.tier_a_dropped.to_string();

        let rows = Flex::vertical()
            .constraints([
                Constraint::Fixed(1), // total_events
                Constraint::Fixed(1), // tier_a_events
                Constraint::Fixed(1), // clock_skew
                Constraint::Fixed(1), // tier_a_dropped
                Constraint::Fixed(1), // spacer
                Constraint::Fixed(1), // hash
            ])
            .split(area);

        render_metric_line(frame, rows[0], "  total_events", &total_str, ACCENT_GREEN);
        render_metric_line(frame, rows[1], "  tier_a_events", &tier_a_str, ACCENT_CYAN);
        render_metric_line(
            frame,
            rows[2],
            "  clock_skew",
            &skew_str,
            if self.cockpit_viewmodel.clock_skew_detected_count > 0 {
                ACCENT_YELLOW
            } else {
                ACCENT_GREEN
            },
        );
        render_metric_line(
            frame,
            rows[3],
            "  tier_a_dropped",
            &dropped_str,
            if self.cockpit_viewmodel.tier_a_dropped > 0 {
                ACCENT_RED
            } else {
                ACCENT_GREEN
            },
        );

        // Hash line with glowing text
        let hash_cols = Flex::horizontal()
            .constraints([Constraint::Fixed(22), Constraint::Min(1)])
            .split(rows[5]);

        let hash_label = Paragraph::new(Text::from(Line::styled(
            "  viewmodel.hash",
            Style::new().fg(FG_LABEL),
        )));
        hash_label.render(hash_cols[0], frame);

        GlowingText::new(&self.cockpit_viewmodel_hash)
            .color(HASH_COLOR)
            .glow(HASH_GLOW)
            .glow_intensity(0.6)
            .bold()
            .render(hash_cols[1], frame);
    }

    fn render_event_table(&self, frame: &mut Frame, area: ftui::core::geometry::Rect) {
        let max_count = self
            .cockpit_viewmodel
            .event_counts_by_type
            .values()
            .max()
            .copied()
            .unwrap_or(1);

        let header = Row::new([
            Text::from(Line::styled(
                "  Event Type",
                Style::new().fg(ACCENT_CYAN).bold(),
            )),
            Text::from(Line::styled(" Count", Style::new().fg(ACCENT_CYAN).bold())),
            Text::from(Line::styled(
                "Distribution",
                Style::new().fg(ACCENT_CYAN).bold(),
            )),
        ]);

        let data_rows: Vec<Row> = self
            .cockpit_viewmodel
            .event_counts_by_type
            .iter()
            .map(|(event_type, count)| {
                let color = event_type_color(event_type);
                Row::new([
                    Text::from(Line::styled(
                        format!("  {}", event_type),
                        Style::new().fg(color),
                    )),
                    Text::from(Line::styled(
                        format!("{:>6}", count),
                        Style::new().fg(FG_PRIMARY).bold(),
                    )),
                    Text::from(Line::styled(
                        bar_string(*count, max_count, 12),
                        Style::new().fg(color),
                    )),
                ])
            })
            .collect();

        let table = Table::new(
            data_rows,
            [
                Constraint::Min(20),
                Constraint::Fixed(8),
                Constraint::Fixed(14),
            ],
        )
        .header(header)
        .style(Style::new().fg(FG_PRIMARY).bg(BG_DEEP))
        .theme(TableTheme::preset(TablePresetId::Midnight));

        table.render(area, frame);
    }

    fn render_error(&self, frame: &mut Frame, area: ftui::core::geometry::Rect) {
        if let Some((commit_index, kind, detail)) = &self.cockpit_viewmodel.latest_error {
            let error_rows = Flex::vertical()
                .constraints([
                    Constraint::Fixed(1), // Rule
                    Constraint::Fixed(1), // commit_index + kind
                    Constraint::Fixed(1), // detail
                    Constraint::Min(1),   // spacer
                ])
                .split(area);

            Rule::new()
                .title("Latest Error")
                .style(Style::new().fg(ACCENT_RED))
                .render(error_rows[0], frame);

            let idx_line = Paragraph::new(Text::from(Line::from_spans([
                Span::styled("  commit_index=", Style::new().fg(FG_LABEL)),
                Span::styled(commit_index.to_string(), Style::new().fg(ACCENT_ORANGE)),
                Span::styled("  kind=", Style::new().fg(FG_LABEL)),
                Span::styled(kind, Style::new().fg(ACCENT_RED).bold()),
            ])));
            idx_line.render(error_rows[1], frame);

            let detail_line = Paragraph::new(Text::from(Line::from_spans([
                Span::styled("  detail=", Style::new().fg(FG_LABEL)),
                Span::styled(detail, Style::new().fg(FG_PRIMARY)),
            ])));
            detail_line.render(error_rows[2], frame);
        }
    }
}

// ── Free functions ──────────────────────────────────────────────────────

fn render_metric_line(
    frame: &mut Frame,
    area: ftui::core::geometry::Rect,
    label: &str,
    value: &str,
    value_color: PackedRgba,
) {
    let paragraph = Paragraph::new(Text::from(Line::from_spans([
        Span::styled(format!("{:<22}", label), Style::new().fg(FG_LABEL)),
        Span::styled(value.to_string(), Style::new().fg(value_color).bold()),
    ])));
    paragraph.render(area, frame);
}

fn bar_string(count: u64, max: u64, width: usize) -> String {
    let ratio = if max > 0 {
        count as f64 / max as f64
    } else {
        0.0
    };
    let filled = (ratio * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "\u{2593}".repeat(filled), "\u{2591}".repeat(empty))
}

fn event_type_color(event_type: &str) -> PackedRgba {
    match event_type {
        "RunStart" | "RunEnd" => ACCENT_CYAN,
        "ToolCall" | "ToolResult" => ACCENT_GREEN,
        "PolicyDecision" => ACCENT_YELLOW,
        "RedactionApplied" => ACCENT_ORANGE,
        "Error" => ACCENT_RED,
        "ClockSkewDetected" => ACCENT_YELLOW,
        _ => FG_PRIMARY,
    }
}

fn main() -> Result<()> {
    let command_line_arguments = CommandLineArguments::parse();
    let committed_events = read_eventlog(&command_line_arguments.events)
        .with_context(|| format!("read eventlog at {:?}", command_line_arguments.events))?;

    let cockpit_viewmodel = build_cockpit_viewmodel(&committed_events);
    let cockpit_viewmodel_hash = blake3_hash_of_stable_json(&cockpit_viewmodel)?;

    if command_line_arguments.headless {
        println!("Vifei x FrankenTUI (read-only cockpit bootstrap)");
        println!();
        println!("total_events: {}", cockpit_viewmodel.total_events);
        println!("tier_a_events: {}", cockpit_viewmodel.tier_a_events);
        println!(
            "clock_skew_detected_count: {}",
            cockpit_viewmodel.clock_skew_detected_count
        );
        println!("tier_a_dropped: {}", cockpit_viewmodel.tier_a_dropped);
        println!("viewmodel.hash: {}", cockpit_viewmodel_hash);
        println!();
        println!("counts_by_type:");
        for (event_type, count) in &cockpit_viewmodel.event_counts_by_type {
            println!("  - {}: {}", event_type, count);
        }
        if let Some((commit_index, kind, detail)) = &cockpit_viewmodel.latest_error {
            println!();
            println!("latest_error: commit_index={} kind={}", commit_index, kind);
            println!("  detail={}", detail);
        }
        return Ok(());
    }

    let model = FrankenTuiModel {
        cockpit_viewmodel,
        cockpit_viewmodel_hash,
        start_time: Instant::now(),
        tick_count: 0,
        quit: false,
    };

    App::new(model).run().context("run ftui app")?;
    Ok(())
}
