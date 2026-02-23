use anyhow::{Context as AnyhowContext, Result};
use clap::Parser;
use ftui::prelude::*;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::Widget;
use std::collections::BTreeMap;
use std::path::PathBuf;
use vifei_core::event::{CommittedEvent, EventPayload, Tier};
use vifei_core::eventlog::read_eventlog;

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

#[allow(dead_code)]
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
}

impl Model for FrankenTuiModel {
    type Message = Message;

    fn update(&mut self, _message: Self::Message) -> Cmd<Self::Message> {
        Cmd::none()
    }

    fn view(&self, frame: &mut Frame) {
        let mut text_lines: Vec<String> = Vec::new();

        text_lines.push("Vifei x FrankenTUI (read-only cockpit bootstrap)".to_string());
        text_lines.push("Exit: Ctrl-C (q-to-quit wiring in next step).".to_string());
        text_lines.push(String::new());

        text_lines.push(format!(
            "total_events: {}",
            self.cockpit_viewmodel.total_events
        ));
        text_lines.push(format!(
            "tier_a_events: {}",
            self.cockpit_viewmodel.tier_a_events
        ));
        text_lines.push(format!(
            "clock_skew_detected_count: {}",
            self.cockpit_viewmodel.clock_skew_detected_count
        ));
        text_lines.push(format!(
            "tier_a_dropped: {}",
            self.cockpit_viewmodel.tier_a_dropped
        ));
        text_lines.push(format!("viewmodel.hash: {}", self.cockpit_viewmodel_hash));
        text_lines.push(String::new());

        text_lines.push("counts_by_type:".to_string());
        for (event_type, count) in &self.cockpit_viewmodel.event_counts_by_type {
            text_lines.push(format!("  - {}: {}", event_type, count));
        }

        if let Some((commit_index, kind, detail)) = &self.cockpit_viewmodel.latest_error {
            text_lines.push(String::new());
            text_lines.push(format!(
                "latest_error: commit_index={} kind={}",
                commit_index, kind
            ));
            text_lines.push(format!("  detail={}", detail));
        }

        let paragraph = Paragraph::new(text_lines.join("\n"));
        paragraph.render(frame.bounds(), frame);
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
    };

    App::new(model).run().context("run ftui app")?;
    Ok(())
}
