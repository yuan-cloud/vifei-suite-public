//! Panopticon CLI entry point.
//!
//! Provides the `panopticon` binary with subcommands for viewing,
//! exporting, and stress-testing EventLogs.

use clap::{Parser, Subcommand};
use panopticon_export::{ExportConfig, ExportResult};
use panopticon_tour::TourConfig;
use panopticon_tui::run_viewer;
use std::fmt::Write as _;
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

fn format_cli_failure(
    what_failed: &str,
    likely_cause: &str,
    next_commands: &[String],
    evidence_paths: &[String],
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Error: {what_failed}");
    let _ = writeln!(out, "Likely cause: {likely_cause}");

    if !next_commands.is_empty() {
        let _ = writeln!(out, "Next command(s):");
        for (i, cmd) in next_commands.iter().enumerate() {
            let _ = writeln!(out, "  {}. {}", i + 1, cmd);
        }
    }

    if !evidence_paths.is_empty() {
        let _ = writeln!(out, "Evidence:");
        for path in evidence_paths {
            let _ = writeln!(out, "  - {path}");
        }
    }

    out.trim_end().to_string()
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::View { eventlog } => {
            if let Err(e) = run_viewer(&eventlog) {
                let msg = format_cli_failure(
                    &format!("view failed: {e}"),
                    "EventLog path is invalid or input is not canonical EventLog JSONL.",
                    &[
                        format!("panopticon view {}", eventlog.display()),
                        "panopticon --help".to_string(),
                    ],
                    &[eventlog.display().to_string()],
                );
                eprintln!("{msg}");
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
                let msg = format_cli_failure(
                    "--share-safe flag is required in v0.1.",
                    "Export without secret scanning is disabled for share-safe posture.",
                    &[
                        format!(
                            "panopticon export {} --share-safe --output {}",
                            eventlog.display(),
                            output.display()
                        ),
                        format!(
                            "panopticon export {} --share-safe --output {} --refusal-report out/refusal-report.json",
                            eventlog.display(),
                            output.display()
                        ),
                    ],
                    &[eventlog.display().to_string()],
                );
                eprintln!("{msg}");
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
                    let mut evidence = vec![eventlog.display().to_string()];
                    if let Some(ref report_path) = config.refusal_report_path {
                        evidence.push(report_path.display().to_string());
                    }
                    eprintln!(
                        "{}",
                        format_cli_failure(
                            &format!("export refused: {}", report.summary),
                            "Secret scanner found sensitive content and blocked bundle creation.",
                            &[
                                "Inspect refusal-report.json for exact blocked fields.".to_string(),
                                format!(
                                    "panopticon export {} --share-safe --output {} --refusal-report out/refusal-report.json",
                                    eventlog.display(),
                                    output.display()
                                ),
                            ],
                            &evidence,
                        )
                    );
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
                    return ExitCode::FAILURE;
                }
                Err(e) => {
                    eprintln!(
                        "{}",
                        format_cli_failure(
                            &format!("export failed: {e}"),
                            "File path, permissions, or bundle write step failed.",
                            &[
                                format!(
                                    "panopticon export {} --share-safe --output {} --refusal-report out/refusal-report.json",
                                    eventlog.display(),
                                    output.display()
                                ),
                                "panopticon --help".to_string(),
                            ],
                            &[eventlog.display().to_string(), output.display().to_string()],
                        )
                    );
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
                let msg = format_cli_failure(
                    "--stress flag is required in v0.1.",
                    "Tour is a stress harness and must run with explicit stress intent.",
                    &[format!(
                        "panopticon tour {} --stress --output-dir {}",
                        fixture.display(),
                        output_dir.display()
                    )],
                    &[fixture.display().to_string()],
                );
                eprintln!("{msg}");
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
                    eprintln!(
                        "{}",
                        format_cli_failure(
                            &format!("tour failed: {e}"),
                            "Fixture path is invalid or tour artifact generation failed.",
                            &[format!(
                                "panopticon tour {} --stress --output-dir {}",
                                fixture.display(),
                                output_dir.display()
                            )],
                            &[
                                fixture.display().to_string(),
                                output_dir.display().to_string()
                            ],
                        )
                    );
                    return ExitCode::FAILURE;
                }
            }
        }
    }

    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::format_cli_failure;

    #[test]
    fn cli_failure_template_has_required_sections() {
        let msg = format_cli_failure(
            "export failed: permission denied",
            "Output path is not writable.",
            &[String::from(
                "panopticon export in.jsonl --share-safe --output out.tar.zst",
            )],
            &[String::from("in.jsonl"), String::from("out.tar.zst")],
        );

        assert!(msg.contains("Error: export failed: permission denied"));
        assert!(msg.contains("Likely cause: Output path is not writable."));
        assert!(msg.contains("Next command(s):"));
        assert!(msg.contains("Evidence:"));
    }

    #[test]
    fn cli_failure_template_numbers_next_commands() {
        let msg = format_cli_failure(
            "tour failed",
            "Fixture path invalid.",
            &[
                String::from("panopticon tour fixtures/large-stress.jsonl --stress"),
                String::from("panopticon --help"),
            ],
            &[String::from("fixtures/large-stress.jsonl")],
        );

        assert!(msg.contains("  1. panopticon tour fixtures/large-stress.jsonl --stress"));
        assert!(msg.contains("  2. panopticon --help"));
    }
}
