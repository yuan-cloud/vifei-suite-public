use anyhow::{Context as AnyhowContext, Result};
use clap::Parser;
use ftui::prelude::*;
use ftui::render::cell::PackedRgba;
use ftui::text::{Line, Span, Text};
use ftui::widgets::block::{Alignment, Block};
use ftui::widgets::borders::{BorderType, Borders};
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::Widget;
use ftui::KeyEventKind;
use std::collections::BTreeMap;
use std::path::PathBuf;
use vifei_core::event::{CommittedEvent, EventPayload, Tier};
use vifei_core::eventlog::read_eventlog;

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
    quit: bool,
}

impl Model for FrankenTuiModel {
    type Message = Message;

    fn update(&mut self, message: Self::Message) -> Cmd<Self::Message> {
        if let Message::Terminal(Event::Key(key_event)) = message {
            if key_event.kind == KeyEventKind::Press {
                match key_event.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        self.quit = true;
                        return Cmd::quit();
                    }
                    KeyCode::Char('c') if key_event.modifiers.contains(Modifiers::CTRL) => {
                        self.quit = true;
                        return Cmd::quit();
                    }
                    _ => {}
                }
            }
        }
        Cmd::none()
    }

    #[allow(clippy::vec_init_then_push)]
    fn view(&self, frame: &mut Frame) {
        let area = frame.bounds();

        // Fill background
        let bg_block = Block::new().style(Style::new().bg(BG_DEEP));
        bg_block.render(area, frame);

        // Build styled lines
        let mut lines: Vec<Line<'_>> = Vec::new();

        // Title
        lines.push(Line::new());
        lines.push(Line::from_spans([
            Span::styled("  VIFEI ", Style::new().fg(ACCENT_GREEN).bold()),
            Span::styled(" \u{00d7} ", Style::new().fg(FG_MUTED)),
            Span::styled("FrankenTUI ", Style::new().fg(ACCENT_CYAN).bold()),
            Span::styled("Cockpit", Style::new().fg(FG_PRIMARY)),
        ]));
        lines.push(Line::new());

        // Separator
        lines.push(Line::styled(
            "  \u{2500}\u{2500}\u{2500} Truth Kernel Summary \u{2500}\u{2500}\u{2500}",
            Style::new().fg(BORDER_COLOR),
        ));
        lines.push(Line::new());

        // Metrics
        let total_str = self.cockpit_viewmodel.total_events.to_string();
        let tier_a_str = self.cockpit_viewmodel.tier_a_events.to_string();
        let skew_str = self.cockpit_viewmodel.clock_skew_detected_count.to_string();
        let dropped_str = self.cockpit_viewmodel.tier_a_dropped.to_string();

        lines.push(kv_line("  total_events", &total_str, ACCENT_GREEN));
        lines.push(kv_line("  tier_a_events", &tier_a_str, ACCENT_CYAN));
        lines.push(kv_line(
            "  clock_skew",
            &skew_str,
            if self.cockpit_viewmodel.clock_skew_detected_count > 0 {
                ACCENT_YELLOW
            } else {
                ACCENT_GREEN
            },
        ));
        lines.push(kv_line(
            "  tier_a_dropped",
            &dropped_str,
            if self.cockpit_viewmodel.tier_a_dropped > 0 {
                ACCENT_RED
            } else {
                ACCENT_GREEN
            },
        ));
        lines.push(Line::new());

        // Hash
        lines.push(Line::from_spans([
            Span::styled("  viewmodel.hash ", Style::new().fg(FG_LABEL)),
            Span::styled(&self.cockpit_viewmodel_hash, Style::new().fg(HASH_COLOR)),
        ]));
        lines.push(Line::new());

        // Separator
        lines.push(Line::styled(
            "  \u{2500}\u{2500}\u{2500} Event Breakdown \u{2500}\u{2500}\u{2500}",
            Style::new().fg(BORDER_COLOR),
        ));
        lines.push(Line::new());

        // Event type counts
        for (event_type, count) in &self.cockpit_viewmodel.event_counts_by_type {
            let color = event_type_color(event_type);
            lines.push(Line::from_spans([
                Span::styled(format!("  {:<22}", event_type), Style::new().fg(color)),
                Span::styled(format!("{:>6}", count), Style::new().fg(FG_PRIMARY).bold()),
            ]));
        }

        // Error section
        if let Some((commit_index, kind, detail)) = &self.cockpit_viewmodel.latest_error {
            lines.push(Line::new());
            lines.push(Line::styled(
                "  \u{2500}\u{2500}\u{2500} Latest Error \u{2500}\u{2500}\u{2500}",
                Style::new().fg(ACCENT_RED),
            ));
            lines.push(Line::from_spans([
                Span::styled("  commit_index=", Style::new().fg(FG_LABEL)),
                Span::styled(commit_index.to_string(), Style::new().fg(ACCENT_ORANGE)),
                Span::styled("  kind=", Style::new().fg(FG_LABEL)),
                Span::styled(kind, Style::new().fg(ACCENT_RED).bold()),
            ]));
            lines.push(Line::from_spans([
                Span::styled("  detail=", Style::new().fg(FG_LABEL)),
                Span::styled(detail, Style::new().fg(FG_PRIMARY)),
            ]));
        }

        // Footer
        lines.push(Line::new());
        lines.push(Line::styled("  Press q to quit", Style::new().fg(FG_MUTED)));

        // Render in a bordered block
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Vifei Cockpit ")
            .title_alignment(Alignment::Center)
            .style(Style::new().fg(ACCENT_CYAN).bg(BG_DEEP));

        let inner = block.inner(area);
        block.render(area, frame);

        let text = Text::from_lines(lines);
        let paragraph = Paragraph::new(text);
        paragraph.render(inner, frame);
    }
}

fn kv_line<'a>(label: &'a str, value: &'a str, value_color: PackedRgba) -> Line<'a> {
    Line::from_spans([
        Span::styled(format!("{:<22}", label), Style::new().fg(FG_LABEL)),
        Span::styled(value.to_string(), Style::new().fg(value_color).bold()),
    ])
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
        quit: false,
    };

    App::new(model).run().context("run ftui app")?;
    Ok(())
}
