//! Panopticon CLI entry point.
//!
//! Provides the `panopticon` binary with subcommands for viewing EventLogs.

use clap::{Parser, Subcommand};
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
    }

    ExitCode::SUCCESS
}
