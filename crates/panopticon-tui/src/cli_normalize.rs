use crate::cli_contract::OutputMode;
use std::fmt::Write as _;

pub(crate) fn format_cli_failure(
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

pub(crate) fn looks_like_json_requested(args: &[String]) -> bool {
    args.iter().any(|a| a == "--json")
}

pub(crate) fn looks_like_human_requested(args: &[String]) -> bool {
    args.iter().any(|a| a == "--human")
}

pub(crate) fn select_output_mode(
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

pub(crate) fn normalize_args(args: Vec<String>) -> (Vec<String>, Vec<String>) {
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
