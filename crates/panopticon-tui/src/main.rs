//! Panopticon CLI entry point.
//!
//! Provides the `panopticon` binary with subcommands for viewing,
//! exporting, and stress-testing EventLogs.

use clap::{Parser, Subcommand};
use panopticon_export::{ExportConfig, ExportResult};
use panopticon_tour::TourConfig;
use panopticon_tui::run_viewer;
use serde_json::{json, Value};
use std::env;
use std::fmt::Write as _;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Panopticon Suite — deterministic flight recorder for AI agent runs.
#[derive(Parser)]
#[command(name = "panopticon")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Emit machine-readable JSON output.
    #[arg(long, global = true, conflicts_with = "human")]
    json: bool,

    /// Force human-readable output (overrides auto JSON in piped mode).
    #[arg(long, global = true)]
    human: bool,

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputMode {
    Human,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppExit {
    Success = 0,
    NotFound = 1,
    InvalidArgs = 2,
    ExportRefused = 3,
    RuntimeError = 4,
}

impl AppExit {
    fn code(self) -> ExitCode {
        ExitCode::from(self as u8)
    }
}

const QUICK_HELP: &str = "\
panopticon — deterministic AI run recorder
Usage: panopticon [--json|--human] <command> [args]
Commands:
  view <eventlog.jsonl>
  export <eventlog.jsonl> --share-safe --output <bundle.tar.zst> [--refusal-report <path>]
  tour <fixture.jsonl> --stress [--output-dir <dir>]
Tips:
  panopticon --help
  panopticon <command> --help";
const ROBOT_SCHEMA_VERSION: &str = "panopticon-cli-robot-v1.1";

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

fn looks_like_json_requested(args: &[String]) -> bool {
    args.iter().any(|a| a == "--json")
}

fn looks_like_human_requested(args: &[String]) -> bool {
    args.iter().any(|a| a == "--human")
}

fn select_output_mode(
    explicit_json: bool,
    explicit_human: bool,
    stdout_is_tty: bool,
) -> OutputMode {
    if explicit_json {
        return OutputMode::Json;
    }
    if explicit_human {
        return OutputMode::Human;
    }
    if stdout_is_tty {
        OutputMode::Human
    } else {
        OutputMode::Json
    }
}

fn normalize_args(args: Vec<String>) -> (Vec<String>, Vec<String>) {
    let mut repaired = args;
    let mut notes = Vec::new();

    for arg in &mut repaired {
        let replacement = match arg.as_str() {
            "--share_safe" => Some("--share-safe"),
            "--refusal_report" => Some("--refusal-report"),
            "--output_dir" => Some("--output-dir"),
            "viewer" => Some("view"),
            "exports" => Some("export"),
            "tours" => Some("tour"),
            _ => None,
        };

        if let Some(new) = replacement {
            notes.push(format!("normalized `{}` -> `{}`", arg, new));
            *arg = new.to_string();
        }
    }

    (repaired, notes)
}

fn emit_json(value: Value) {
    println!(
        "{}",
        serde_json::to_string(&value).expect("json output serialization must succeed")
    );
}

fn emit_json_success(
    code: &str,
    message: &str,
    command: Option<&str>,
    exit_code: u8,
    notes: &[String],
    mut data: Value,
) {
    if data.is_null() {
        data = json!({});
    }
    let mut obj = json!({
        "schema_version": ROBOT_SCHEMA_VERSION,
        "ok": true,
        "code": code,
        "message": message,
        "suggestions": [],
        "exit_code": exit_code,
        "data": data,
    });
    if let Some(command) = command {
        obj["command"] = json!(command);
    }
    if !notes.is_empty() {
        obj["notes"] = json!(notes);
    }
    emit_json(obj);
}

fn emit_json_error(
    code: &str,
    message: &str,
    suggestions: &[String],
    notes: &[String],
    exit_code: u8,
) {
    let mut obj = json!({
        "schema_version": ROBOT_SCHEMA_VERSION,
        "ok": false,
        "code": code,
        "message": message,
        "suggestions": suggestions,
        "exit_code": exit_code,
    });
    if !notes.is_empty() {
        obj["notes"] = json!(notes);
    }
    emit_json(obj);
}

fn ensure_file_exists(path: &Path, label: &str) -> Result<(), String> {
    if path.exists() {
        Ok(())
    } else {
        Err(format!("{} not found: {}", label, path.display()))
    }
}

fn main() -> ExitCode {
    let raw_args: Vec<String> = env::args().collect();
    let mode = select_output_mode(
        looks_like_json_requested(&raw_args),
        looks_like_human_requested(&raw_args),
        io::stdout().is_terminal(),
    );
    if raw_args.len() == 1 {
        if mode == OutputMode::Json {
            emit_json_success(
                "OK",
                "Quick help emitted.",
                Some("help"),
                AppExit::Success as u8,
                &[],
                json!({
                    "quick_help": QUICK_HELP,
                }),
            );
        } else {
            println!("{QUICK_HELP}");
        }
        return AppExit::Success.code();
    }

    let (args, repair_notes) = normalize_args(raw_args);

    let cli = match Cli::try_parse_from(&args) {
        Ok(cli) => cli,
        Err(err) => {
            let suggestions = vec![
                "Run `panopticon --help` for command syntax.".to_string(),
                "Run `panopticon <command> --help` for command-specific args.".to_string(),
            ];
            if mode == OutputMode::Json {
                emit_json_error(
                    "INVALID_ARGS",
                    "Invalid command syntax.",
                    &suggestions,
                    &repair_notes,
                    AppExit::InvalidArgs as u8,
                );
            } else {
                if !repair_notes.is_empty() {
                    for note in &repair_notes {
                        eprintln!("Note: {note}");
                    }
                }
                eprintln!("{err}");
            }
            return AppExit::InvalidArgs.code();
        }
    };

    let mode = select_output_mode(cli.json, cli.human, io::stdout().is_terminal());

    match cli.command {
        Commands::View { eventlog } => {
            if let Err(msg) = ensure_file_exists(&eventlog, "eventlog file") {
                let suggestions = vec![
                    format!(
                        "Check that `{}` exists and is readable.",
                        eventlog.display()
                    ),
                    format!("panopticon view {}", eventlog.display()),
                ];
                if mode == OutputMode::Json {
                    emit_json_error(
                        "NOT_FOUND",
                        &msg,
                        &suggestions,
                        &repair_notes,
                        AppExit::NotFound as u8,
                    );
                } else {
                    eprintln!(
                        "{}",
                        format_cli_failure(
                            &format!("view failed: {msg}"),
                            "Input path does not exist.",
                            &suggestions,
                            &[eventlog.display().to_string()],
                        )
                    );
                }
                return AppExit::NotFound.code();
            }
            if let Err(e) = run_viewer(&eventlog) {
                let suggestions = vec![
                    format!("panopticon view {}", eventlog.display()),
                    "panopticon --help".to_string(),
                ];
                if mode == OutputMode::Json {
                    emit_json_error(
                        "RUNTIME_ERROR",
                        &format!("view failed: {e}"),
                        &suggestions,
                        &repair_notes,
                        AppExit::RuntimeError as u8,
                    );
                } else {
                    let msg = format_cli_failure(
                        &format!("view failed: {e}"),
                        "EventLog path is invalid or input is not canonical EventLog JSONL.",
                        &suggestions,
                        &[eventlog.display().to_string()],
                    );
                    eprintln!("{msg}");
                }
                return AppExit::RuntimeError.code();
            }
        }

        Commands::Export {
            eventlog,
            output,
            share_safe,
            refusal_report,
        } => {
            if let Err(msg) = ensure_file_exists(&eventlog, "eventlog file") {
                let suggestions = vec![
                    format!(
                        "Check that `{}` exists and is readable.",
                        eventlog.display()
                    ),
                    format!(
                        "panopticon export {} --share-safe --output {}",
                        eventlog.display(),
                        output.display()
                    ),
                ];
                if mode == OutputMode::Json {
                    emit_json_error(
                        "NOT_FOUND",
                        &msg,
                        &suggestions,
                        &repair_notes,
                        AppExit::NotFound as u8,
                    );
                } else {
                    eprintln!(
                        "{}",
                        format_cli_failure(
                            &format!("export failed: {msg}"),
                            "Input path does not exist.",
                            &suggestions,
                            &[eventlog.display().to_string()],
                        )
                    );
                }
                return AppExit::NotFound.code();
            }
            // Require --share-safe flag
            if !share_safe {
                let suggestions = vec![
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
                ];
                if mode == OutputMode::Json {
                    emit_json_error(
                        "INVALID_ARGS",
                        "--share-safe flag is required in v0.1.",
                        &suggestions,
                        &repair_notes,
                        AppExit::InvalidArgs as u8,
                    );
                } else {
                    let msg = format_cli_failure(
                        "--share-safe flag is required in v0.1.",
                        "Export without secret scanning is disabled for share-safe posture.",
                        &suggestions,
                        &[eventlog.display().to_string()],
                    );
                    eprintln!("{msg}");
                }
                return AppExit::InvalidArgs.code();
            }

            let mut config = ExportConfig::new(&eventlog, &output);
            config.share_safe = share_safe;
            if let Some(report_path) = refusal_report {
                config = config.with_refusal_report(report_path);
            }

            match panopticon_export::run_export(&config) {
                Ok(ExportResult::Success(success)) => {
                    if mode == OutputMode::Json {
                        emit_json_success(
                            "OK",
                            "Export completed successfully.",
                            Some("export"),
                            AppExit::Success as u8,
                            &repair_notes,
                            json!({
                                "bundle_path": success.bundle_path,
                                "bundle_hash": success.bundle_hash,
                                "event_count": success.event_count,
                                "blob_count": success.blob_count,
                            }),
                        );
                    } else {
                        println!("Export successful!");
                        println!("  Bundle: {}", success.bundle_path.display());
                        println!("  Hash:   {}", success.bundle_hash);
                        println!("  Events: {}", success.event_count);
                        println!("  Blobs:  {}", success.blob_count);
                    }
                }
                Ok(ExportResult::Refused(report)) => {
                    let mut evidence = vec![eventlog.display().to_string()];
                    if let Some(ref report_path) = config.refusal_report_path {
                        evidence.push(report_path.display().to_string());
                    }
                    let suggestions = vec![
                        "Inspect refusal-report.json for exact blocked fields.".to_string(),
                        format!(
                            "panopticon export {} --share-safe --output {} --refusal-report out/refusal-report.json",
                            eventlog.display(),
                            output.display()
                        ),
                    ];
                    if mode == OutputMode::Json {
                        let mut resp = json!({
                            "schema_version": ROBOT_SCHEMA_VERSION,
                            "ok": false,
                            "code": "EXPORT_REFUSED",
                            "message": report.summary,
                            "suggestions": suggestions,
                            "blocked_items": report.blocked_items,
                            "evidence": evidence,
                            "exit_code": AppExit::ExportRefused as u8,
                        });
                        if !repair_notes.is_empty() {
                            resp["notes"] = json!(repair_notes);
                        }
                        emit_json(resp);
                    } else {
                        eprintln!(
                            "{}",
                            format_cli_failure(
                                &format!("export refused: {}", report.summary),
                                "Secret scanner found sensitive content and blocked bundle creation.",
                                &suggestions,
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
                    }
                    return AppExit::ExportRefused.code();
                }
                Err(e) => {
                    let suggestions = vec![
                        format!(
                            "panopticon export {} --share-safe --output {} --refusal-report out/refusal-report.json",
                            eventlog.display(),
                            output.display()
                        ),
                        "panopticon --help".to_string(),
                    ];
                    if mode == OutputMode::Json {
                        emit_json_error(
                            "RUNTIME_ERROR",
                            &format!("export failed: {e}"),
                            &suggestions,
                            &repair_notes,
                            AppExit::RuntimeError as u8,
                        );
                    } else {
                        eprintln!(
                            "{}",
                            format_cli_failure(
                                &format!("export failed: {e}"),
                                "File path, permissions, or bundle write step failed.",
                                &suggestions,
                                &[eventlog.display().to_string(), output.display().to_string()],
                            )
                        );
                    }
                    return AppExit::RuntimeError.code();
                }
            }
        }

        Commands::Tour {
            fixture,
            stress,
            output_dir,
        } => {
            if let Err(msg) = ensure_file_exists(&fixture, "fixture file") {
                let suggestions = vec![
                    format!("Check that `{}` exists and is readable.", fixture.display()),
                    format!(
                        "panopticon tour {} --stress --output-dir {}",
                        fixture.display(),
                        output_dir.display()
                    ),
                ];
                if mode == OutputMode::Json {
                    emit_json_error(
                        "NOT_FOUND",
                        &msg,
                        &suggestions,
                        &repair_notes,
                        AppExit::NotFound as u8,
                    );
                } else {
                    eprintln!(
                        "{}",
                        format_cli_failure(
                            &format!("tour failed: {msg}"),
                            "Fixture path does not exist.",
                            &suggestions,
                            &[fixture.display().to_string()],
                        )
                    );
                }
                return AppExit::NotFound.code();
            }
            // Require --stress flag
            if !stress {
                let suggestions = vec![format!(
                    "panopticon tour {} --stress --output-dir {}",
                    fixture.display(),
                    output_dir.display()
                )];
                if mode == OutputMode::Json {
                    emit_json_error(
                        "INVALID_ARGS",
                        "--stress flag is required in v0.1.",
                        &suggestions,
                        &repair_notes,
                        AppExit::InvalidArgs as u8,
                    );
                } else {
                    let msg = format_cli_failure(
                        "--stress flag is required in v0.1.",
                        "Tour is a stress harness and must run with explicit stress intent.",
                        &suggestions,
                        &[fixture.display().to_string()],
                    );
                    eprintln!("{msg}");
                }
                return AppExit::InvalidArgs.code();
            }

            let config = TourConfig::new(&fixture).with_output_dir(&output_dir);

            match panopticon_tour::run_tour(&config) {
                Ok(result) => {
                    if mode == OutputMode::Json {
                        emit_json_success(
                            "OK",
                            "Tour completed successfully.",
                            Some("tour"),
                            AppExit::Success as u8,
                            &repair_notes,
                            json!({
                                "output_dir": result.output_dir,
                                "event_count": result.metrics.event_count_total,
                                "tier_a_drops": result.metrics.tier_a_drops,
                                "degradation_level": result.metrics.degradation_level_final,
                                "viewmodel_hash": result.viewmodel_hash,
                                "artifacts": [
                                    "metrics.json",
                                    "viewmodel.hash",
                                    "ansi.capture",
                                    "timetravel.capture"
                                ],
                            }),
                        );
                    } else {
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
                }
                Err(e) => {
                    let suggestions = vec![format!(
                        "panopticon tour {} --stress --output-dir {}",
                        fixture.display(),
                        output_dir.display()
                    )];
                    if mode == OutputMode::Json {
                        emit_json_error(
                            "RUNTIME_ERROR",
                            &format!("tour failed: {e}"),
                            &suggestions,
                            &repair_notes,
                            AppExit::RuntimeError as u8,
                        );
                    } else {
                        eprintln!(
                            "{}",
                            format_cli_failure(
                                &format!("tour failed: {e}"),
                                "Fixture path is invalid or tour artifact generation failed.",
                                &suggestions,
                                &[
                                    fixture.display().to_string(),
                                    output_dir.display().to_string()
                                ],
                            )
                        );
                    }
                    return AppExit::RuntimeError.code();
                }
            }
        }
    }

    AppExit::Success.code()
}

#[cfg(test)]
mod tests {
    use super::{format_cli_failure, normalize_args, select_output_mode, OutputMode, QUICK_HELP};

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

    #[test]
    fn quick_help_is_compact() {
        let tokens = QUICK_HELP.split_whitespace().count();
        assert!(
            tokens <= 100,
            "quick help should stay compact, got {tokens}"
        );
    }

    #[test]
    fn output_mode_auto_json_when_not_tty() {
        assert_eq!(
            select_output_mode(false, false, false),
            OutputMode::Json,
            "piped stdout should auto-select json"
        );
    }

    #[test]
    fn output_mode_human_override_beats_auto_json() {
        assert_eq!(
            select_output_mode(false, true, false),
            OutputMode::Human,
            "--human should force human output even when piped"
        );
    }

    #[test]
    fn normalize_args_repairs_common_variants() {
        let (repaired, notes) = normalize_args(vec![
            "panopticon".to_string(),
            "viewer".to_string(),
            "--share_safe".to_string(),
            "--output_dir".to_string(),
            "out".to_string(),
        ]);
        assert_eq!(repaired[1], "view");
        assert!(repaired.contains(&"--share-safe".to_string()));
        assert!(repaired.contains(&"--output-dir".to_string()));
        assert_eq!(notes.len(), 3);
    }
}
