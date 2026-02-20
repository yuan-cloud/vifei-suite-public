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

    // Normalize option spellings in option parsing mode only.
    // Stop normalization after `--` so forced positional values are preserved.
    let mut passthrough_positionals = false;
    for arg in repaired.iter_mut().skip(1) {
        if arg == "--" {
            passthrough_positionals = true;
            continue;
        }
        if passthrough_positionals {
            continue;
        }
        let replacement = match arg.as_str() {
            "--share_safe" => Some("--share-safe"),
            "--refusal_report" => Some("--refusal-report"),
            "--output_dir" => Some("--output-dir"),
            _ => None,
        };

        if let Some(new) = replacement {
            notes.push(format!("normalized `{}` -> `{}`", arg, new));
            *arg = new.to_string();
        }
    }

    (repaired, notes)
}

#[cfg(test)]
mod tests {
    use super::normalize_args;

    #[test]
    fn normalize_does_not_rewrite_positional_subcommand_aliases() {
        let (repaired, notes) = normalize_args(vec!["vifei".to_string(), "viewer".to_string()]);
        assert_eq!(repaired[1], "viewer");
        assert!(notes.is_empty());
    }

    #[test]
    fn normalize_does_not_mutate_positionals_after_double_dash() {
        let (repaired, notes) = normalize_args(vec![
            "vifei".to_string(),
            "view".to_string(),
            "--".to_string(),
            "--output_dir".to_string(),
        ]);
        assert_eq!(repaired[1], "view");
        assert_eq!(repaired[3], "--output_dir");
        assert!(notes.is_empty());
    }

    #[test]
    fn normalize_is_idempotent_for_known_repairs() {
        let (repaired, notes) = normalize_args(vec![
            "vifei".to_string(),
            "export".to_string(),
            "in.jsonl".to_string(),
            "--share_safe".to_string(),
            "--output_dir".to_string(),
        ]);
        let (repaired_2, notes_2) = normalize_args(repaired.clone());
        assert_eq!(repaired, repaired_2);
        assert!(notes.len() >= 2);
        assert!(notes_2.is_empty());
    }
}
