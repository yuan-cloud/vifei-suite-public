//! Panopticon CLI entry point.
//!
//! Provides the `panopticon` binary with subcommands for viewing and
//! exporting EventLogs.

use clap::{Parser, Subcommand};
use panopticon_export::{ExportConfig, ExportResult};
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
                    for finding in &report.findings {
                        eprintln!(
                            "  - {} @ {}: {} ({})",
                            finding.location,
                            finding.field_path,
                            finding.pattern,
                            finding.redacted_match
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
    }

    ExitCode::SUCCESS
}
