//! Panopticon CLI entry point.
//!
//! Provides the `panopticon` binary with subcommands for viewing, exporting,
//! and stress-testing EventLogs.

mod cli_contract;
mod cli_handlers;
mod cli_normalize;

use clap::Parser;
use cli_contract::{AppExit, Cli, OutputMode, QUICK_HELP};
use cli_handlers::{emit_json_error, emit_json_success, handle_command};
use cli_normalize::{
    looks_like_human_requested, looks_like_json_requested, normalize_args, select_output_mode,
};
use serde_json::json;
use std::env;
use std::io::{self, IsTerminal};
use std::process::ExitCode;

#[cfg(test)]
use cli_normalize::format_cli_failure;

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
    handle_command(cli, mode, &repair_notes).code()
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
