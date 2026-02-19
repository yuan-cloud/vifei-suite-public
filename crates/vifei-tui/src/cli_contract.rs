use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::process::ExitCode;

/// Vifei Suite — deterministic flight recorder for AI agent runs.
#[derive(Parser)]
#[command(name = "vifei")]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum UiProfileArg {
    Standard,
    Showcase,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum CompareInputFormat {
    Eventlog,
    Cassette,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// View an EventLog in the TUI.
    #[command(alias = "viewer")]
    View {
        /// Path to the EventLog JSONL file.
        eventlog: PathBuf,

        /// Presentation profile (style/layout only; does not alter truth semantics).
        #[arg(long, value_enum, default_value = "standard")]
        profile: UiProfileArg,
    },

    /// Export an EventLog as a share-safe bundle.
    #[command(alias = "exports")]
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
    #[command(alias = "tours")]
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

    /// Deterministically compare two run inputs and report causal divergences.
    Compare {
        /// Left input path (EventLog JSONL or cassette JSONL).
        left: PathBuf,

        /// Right input path (EventLog JSONL or cassette JSONL).
        right: PathBuf,

        /// Input format for the left side.
        #[arg(long, value_enum, default_value = "eventlog")]
        left_format: CompareInputFormat,

        /// Input format for the right side.
        #[arg(long, value_enum, default_value = "eventlog")]
        right_format: CompareInputFormat,
    },

    /// Build a local-first deterministic incident evidence pack from two inputs.
    #[command(alias = "incident")]
    IncidentPack {
        /// Left input path (EventLog JSONL or cassette JSONL).
        left: PathBuf,

        /// Right input path (EventLog JSONL or cassette JSONL).
        right: PathBuf,

        /// Input format for the left side.
        #[arg(long, value_enum, default_value = "eventlog")]
        left_format: CompareInputFormat,

        /// Input format for the right side.
        #[arg(long, value_enum, default_value = "eventlog")]
        right_format: CompareInputFormat,

        /// Output directory for the generated evidence pack.
        #[arg(long, default_value = "incident-pack")]
        output_dir: PathBuf,
    },

    /// Run strict trust verification checks and emit an auditable summary.
    Verify {
        /// Enable strict mode (fails non-zero if any required check fails).
        #[arg(long)]
        strict: bool,

        /// Use full fixture/profile verification lane.
        #[arg(long)]
        full: bool,

        /// Optional fixture override for determinism replay checks.
        #[arg(long)]
        fixture: Option<PathBuf>,

        /// Output directory for verification artifacts.
        #[arg(long, default_value = "verify-output")]
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
    DiffFound = 5,
}

impl AppExit {
    pub(crate) fn code(self) -> ExitCode {
        ExitCode::from(self as u8)
    }
}

pub(crate) const QUICK_HELP: &str = "\
vifei — deterministic AI run recorder
Usage: vifei [--json|--human] <command> [args]
Commands:
  view <eventlog.jsonl> [--profile standard|showcase]
  export <eventlog.jsonl> --share-safe --output <bundle.tar.zst> [--refusal-report <path>]
  tour <fixture.jsonl> --stress [--output-dir <dir>]
  compare <left.jsonl> <right.jsonl> [--left-format eventlog|cassette] [--right-format eventlog|cassette]
  incident-pack <left.jsonl> <right.jsonl> [--left-format eventlog|cassette] [--right-format eventlog|cassette] [--output-dir <dir>]
  verify --strict [--full] [--fixture <fixture.jsonl>] [--output-dir <dir>]
Tips:
  vifei --help
  vifei <command> --help";

pub(crate) const ROBOT_SCHEMA_VERSION: &str = "vifei-cli-robot-v1.1";

#[cfg(test)]
mod tests {
    use super::{Cli, Commands, CompareInputFormat, UiProfileArg};
    use clap::Parser;

    #[test]
    fn clap_alias_viewer_maps_to_view() {
        let cli = Cli::try_parse_from(["vifei", "viewer", "e.jsonl"]).expect("parse");
        assert!(matches!(cli.command, Commands::View { .. }));
    }

    #[test]
    fn clap_alias_exports_maps_to_export() {
        let cli = Cli::try_parse_from([
            "vifei",
            "exports",
            "e.jsonl",
            "--share-safe",
            "--output",
            "bundle.tar.zst",
        ])
        .expect("parse");
        assert!(matches!(cli.command, Commands::Export { .. }));
    }

    #[test]
    fn clap_alias_tours_maps_to_tour() {
        let cli = Cli::try_parse_from(["vifei", "tours", "f.jsonl", "--stress"]).expect("parse");
        assert!(matches!(cli.command, Commands::Tour { .. }));
    }

    #[test]
    fn view_profile_parses_showcase() {
        let cli = Cli::try_parse_from(["vifei", "view", "e.jsonl", "--profile", "showcase"])
            .expect("parse");
        assert!(matches!(
            cli.command,
            Commands::View {
                profile: UiProfileArg::Showcase,
                ..
            }
        ));
    }

    #[test]
    fn compare_formats_parse_from_flags() {
        let cli = Cli::try_parse_from([
            "vifei",
            "compare",
            "left.jsonl",
            "right.jsonl",
            "--left-format",
            "cassette",
            "--right-format",
            "eventlog",
        ])
        .expect("parse");
        assert!(matches!(
            cli.command,
            Commands::Compare {
                left_format: CompareInputFormat::Cassette,
                right_format: CompareInputFormat::Eventlog,
                ..
            }
        ));
    }

    #[test]
    fn incident_pack_alias_parses() {
        let cli = Cli::try_parse_from([
            "vifei",
            "incident",
            "left.jsonl",
            "right.jsonl",
            "--output-dir",
            "pack",
        ])
        .expect("parse");
        assert!(matches!(cli.command, Commands::IncidentPack { .. }));
    }

    #[test]
    fn verify_parses_strict_and_full_flags() {
        let cli = Cli::try_parse_from([
            "vifei",
            "verify",
            "--strict",
            "--full",
            "--output-dir",
            "verify-dir",
        ])
        .expect("parse");
        assert!(matches!(
            cli.command,
            Commands::Verify {
                strict: true,
                full: true,
                ..
            }
        ));
    }
}
