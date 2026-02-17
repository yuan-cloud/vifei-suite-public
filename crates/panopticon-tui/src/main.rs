//! Panopticon CLI entry point.
//!
//! Provides the `panopticon` binary with subcommands for viewing,
//! exporting, and stress-testing EventLogs.

use clap::{Parser, Subcommand};
use panopticon_export::{ExportConfig, ExportResult};
use panopticon_tour::TourConfig;
use panopticon_tui::run_viewer;
use std::path::PathBuf;
use std::process::ExitCode;

/// Panopticon Suite — deterministic flight recorder for AI agent runs.
#[derive(Parser)]
#[command(name = "panopticon")]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// View an EventLog in the TUI.
    View {
        /// Path to the EventLog JSONL file.
        eventlog: PathBuf,
    },

    /// Export an EventLog as a share-safe bundle.
    Export {
        /// Path to the EventLog JSONL file.
        eventlog: PathBuf,

        /// Output bundle path.
        #[arg(short, long)]
        output: PathBuf,

        /// Enable share-safe secret scanning (required in v0.1).
        #[arg(long)]
        share_safe: bool,

        /// Path to write refusal report if secrets are detected.
        #[arg(long)]
        refusal_report: Option<PathBuf>,
    },

    /// Run the Tour stress harness to generate proof artifacts.
    Tour {
        /// Path to the fixture file (Agent Cassette JSONL).
        fixture: PathBuf,

        /// Enable stress mode (required in v0.1).
        #[arg(long)]
        stress: bool,

        /// Output directory for proof artifacts (default: tour-output).
        #[arg(long, default_value = "tour-output")]
        output_dir: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::View { eventlog } => {
            if let Err(e) = run_viewer(&eventlog) {
                eprintln!("Error: {}", e);
                return ExitCode::FAILURE;
            }
        }

        Commands::Export {
            eventlog,
            output,
            share_safe,
            refusal_report,
        } => {
            // Require --share-safe flag
            if !share_safe {
                eprintln!(
                    "Error: --share-safe flag is required in v0.1.\n\
                     Export without secret scanning is not supported.\n\n\
                     Usage: panopticon export --share-safe -o <output> <eventlog>"
                );
                return ExitCode::FAILURE;
            }

            let mut config = ExportConfig::new(&eventlog, &output);
            config.share_safe = share_safe;
            if let Some(report_path) = refusal_report {
                config = config.with_refusal_report(report_path);
            }

            match panopticon_export::run_export(&config) {
                Ok(ExportResult::Success(success)) => {
                    println!("Export successful!");
                    println!("  Bundle: {}", success.bundle_path.display());
                    println!("  Hash:   {}", success.bundle_hash);
                    println!("  Events: {}", success.event_count);
                    println!("  Blobs:  {}", success.blob_count);
                }
                Ok(ExportResult::Refused(report)) => {
                    eprintln!("Export REFUSED: {}", report.summary);
                    for item in &report.blocked_items {
                        let loc = item
                            .blob_ref
                            .as_deref()
                            .map(|b| format!("blob:{}", b))
                            .unwrap_or_else(|| format!("event:{}", item.event_id));
                        eprintln!(
                            "  - {} @ {}: {} ({})",
                            loc, item.field_path, item.matched_pattern, item.redacted_match
                        );
                    }
                    if let Some(ref report_path) = config.refusal_report_path {
                        eprintln!("Refusal report written to: {}", report_path.display());
                    }
                    return ExitCode::FAILURE;
                }
                Err(e) => {
                    eprintln!("Export error: {}", e);
                    return ExitCode::FAILURE;
                }
            }
        }

        Commands::Tour {
            fixture,
            stress,
            output_dir,
        } => {
            // Require --stress flag
            if !stress {
                eprintln!(
                    "Error: --stress flag is required in v0.1.\n\
                     Tour without stress mode is not supported.\n\n\
                     Usage: panopticon tour --stress <fixture>"
                );
                return ExitCode::FAILURE;
            }

            let config = TourConfig::new(&fixture).with_output_dir(&output_dir);

            match panopticon_tour::run_tour(&config) {
                Ok(result) => {
                    println!("Tour completed successfully!");
                    println!("  Output:   {}", result.output_dir.display());
                    println!("  Events:   {}", result.metrics.event_count_total);
                    println!("  Drops:    {}", result.metrics.tier_a_drops);
                    println!("  Level:    {}", result.metrics.degradation_level_final);
                    println!("  Hash:     {}", result.viewmodel_hash);
                    println!();
                    println!("Artifacts:");
                    println!("  - metrics.json");
                    println!("  - viewmodel.hash");
                    println!("  - ansi.capture");
                    println!("  - timetravel.capture");
                }
                Err(e) => {
                    eprintln!("Tour error: {}", e);
                    return ExitCode::FAILURE;
                }
            }
        }
    }

    ExitCode::SUCCESS
}
