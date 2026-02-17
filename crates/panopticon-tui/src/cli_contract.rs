use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

/// Panopticon Suite — deterministic flight recorder for AI agent runs.
#[derive(Parser)]
#[command(name = "panopticon")]
#[command(version, about, long_about = None)]
pub(crate) struct Cli {
    /// Emit machine-readable JSON output.
    #[arg(long, global = true, conflicts_with = "human")]
    pub(crate) json: bool,

    /// Force human-readable output (overrides auto JSON in piped mode).
    #[arg(long, global = true)]
    pub(crate) human: bool,

    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OutputMode {
    Human,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AppExit {
    Success = 0,
    NotFound = 1,
    InvalidArgs = 2,
    ExportRefused = 3,
    RuntimeError = 4,
}

impl AppExit {
    pub(crate) fn code(self) -> ExitCode {
        ExitCode::from(self as u8)
    }
}

pub(crate) const QUICK_HELP: &str = "\
panopticon — deterministic AI run recorder
Usage: panopticon [--json|--human] <command> [args]
Commands:
  view <eventlog.jsonl>
  export <eventlog.jsonl> --share-safe --output <bundle.tar.zst> [--refusal-report <path>]
  tour <fixture.jsonl> --stress [--output-dir <dir>]
Tips:
  panopticon --help
  panopticon <command> --help";

pub(crate) const ROBOT_SCHEMA_VERSION: &str = "panopticon-cli-robot-v1.1";
