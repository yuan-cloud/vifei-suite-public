use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn run_panopticon(args: &[&str]) -> (i32, String, String) {
    let bin = env!("CARGO_BIN_EXE_panopticon");
    let output = Command::new(bin)
        .args(args)
        .output()
        .expect("run panopticon binary");
    let code = output.status.code().unwrap_or(255);
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    (code, stdout, stderr)
}

fn parse_json(stdout: &str) -> Value {
    serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON")
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root must exist")
        .to_path_buf()
}

#[test]
fn no_args_auto_json_envelope_in_non_tty_mode() {
    let (code, stdout, _stderr) = run_panopticon(&[]);
    assert_eq!(code, 0, "no-args should succeed in robot mode");
    let value = parse_json(&stdout);
    assert_eq!(value["schema_version"], "panopticon-cli-robot-v1.1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["code"], "OK");
    assert_eq!(value["exit_code"], 0);
    assert!(value["data"]["quick_help"].is_string());
}

#[test]
fn invalid_args_emit_structured_error_envelope() {
    let (code, stdout, _stderr) = run_panopticon(&["bogus-subcommand"]);
    assert_eq!(code, 2, "parse failures must map to invalid-args code");
    let value = parse_json(&stdout);
    assert_eq!(value["schema_version"], "panopticon-cli-robot-v1.1");
    assert_eq!(value["ok"], false);
    assert_eq!(value["code"], "INVALID_ARGS");
    assert_eq!(value["exit_code"], 2);
    assert_eq!(value["message"], "Unknown subcommand.");
    assert!(value["suggestions"].is_array());
    assert!(
        value["suggestions"]
            .as_array()
            .expect("suggestions")
            .iter()
            .any(|v| v.as_str().is_some_and(|s| s.contains("panopticon view"))),
        "unknown-subcommand guidance should contain concrete command suggestions"
    );
}

#[test]
fn missing_required_args_emit_specific_guidance() {
    let (code, stdout, _stderr) = run_panopticon(&["--json", "export"]);
    assert_eq!(code, 2, "parse failures must map to invalid-args code");
    let value = parse_json(&stdout);
    assert_eq!(value["ok"], false);
    assert_eq!(value["code"], "INVALID_ARGS");
    assert_eq!(value["message"], "Missing required argument.");
    assert!(
        value["suggestions"]
            .as_array()
            .expect("suggestions")
            .iter()
            .any(|v| v
                .as_str()
                .is_some_and(|s| s.contains("--share-safe --output"))),
        "missing-required guidance should include export example"
    );
}

#[test]
fn conflicting_flags_emit_specific_guidance() {
    let (code, stdout, _stderr) = run_panopticon(&["--json", "--human", "view", "x.jsonl"]);
    assert_eq!(
        code, 2,
        "conflicting parse args should map to invalid-args code"
    );
    let value = parse_json(&stdout);
    assert_eq!(value["ok"], false);
    assert_eq!(value["code"], "INVALID_ARGS");
    assert_eq!(value["message"], "Conflicting flags or arguments.");
    assert!(
        value["suggestions"]
            .as_array()
            .expect("suggestions")
            .iter()
            .any(|v| v
                .as_str()
                .is_some_and(|s| s.contains("--json") && s.contains("--human"))),
        "argument-conflict guidance should mention mutually-exclusive flags"
    );
}

#[test]
fn missing_export_input_maps_not_found_contract() {
    let (code, stdout, _stderr) = run_panopticon(&[
        "--json",
        "export",
        "does-not-exist.jsonl",
        "--share-safe",
        "--output",
        "out.tar.zst",
    ]);
    assert_eq!(code, 1, "missing files must map to not-found code");
    let value = parse_json(&stdout);
    assert_eq!(value["schema_version"], "panopticon-cli-robot-v1.1");
    assert_eq!(value["ok"], false);
    assert_eq!(value["code"], "NOT_FOUND");
    assert_eq!(value["exit_code"], 1);
    assert!(value["message"].is_string());
    assert!(value["suggestions"].is_array());
}

#[test]
fn export_success_emits_structured_json_contract() {
    let dir = tempdir().expect("tempdir");
    let output = dir.path().join("bundle.tar.zst");
    let refusal_report = dir.path().join("refusal-report.json");
    let eventlog = workspace_root()
        .join("docs")
        .join("assets")
        .join("readme")
        .join("sample-export-clean-eventlog.jsonl");

    let (code, stdout, _stderr) = run_panopticon(&[
        "--json",
        "export",
        &eventlog.display().to_string(),
        "--share-safe",
        "--output",
        &output.display().to_string(),
        "--refusal-report",
        &refusal_report.display().to_string(),
    ]);
    assert_eq!(code, 0, "clean export fixture should succeed");

    let value = parse_json(&stdout);
    assert_eq!(value["schema_version"], "panopticon-cli-robot-v1.1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["code"], "OK");
    assert_eq!(value["command"], "export");
    assert_eq!(value["exit_code"], 0);
    assert!(value["data"]["bundle_path"].is_string());
    assert!(value["data"]["bundle_hash"].is_string());
    assert!(value["data"]["event_count"].is_number());
    assert!(value["data"]["blob_count"].is_number());
}

#[test]
fn tour_success_emits_structured_json_contract() {
    let dir = tempdir().expect("tempdir");
    let output_dir = dir.path().join("tour-output");
    let fixture = workspace_root()
        .join("fixtures")
        .join("small-session.jsonl");

    let (code, stdout, _stderr) = run_panopticon(&[
        "--json",
        "tour",
        &fixture.display().to_string(),
        "--stress",
        "--output-dir",
        &output_dir.display().to_string(),
    ]);
    assert_eq!(code, 0, "tour should succeed with stress fixture");

    let value = parse_json(&stdout);
    assert_eq!(value["schema_version"], "panopticon-cli-robot-v1.1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["code"], "OK");
    assert_eq!(value["command"], "tour");
    assert_eq!(value["exit_code"], 0);
    assert!(value["data"]["output_dir"].is_string());
    assert!(value["data"]["event_count"].is_number());
    assert!(value["data"]["tier_a_drops"].is_number());
    assert!(value["data"]["degradation_level"].is_string());
    assert!(value["data"]["viewmodel_hash"].is_string());
    assert!(value["data"]["artifacts"].is_array());
}

#[test]
fn alias_viewer_matches_view_contract_for_missing_file() {
    let (code, stdout, _stderr) = run_panopticon(&["--json", "viewer", "does-not-exist.jsonl"]);
    assert_eq!(code, 1, "viewer alias should route through view handler");
    let value = parse_json(&stdout);
    assert_eq!(value["ok"], false);
    assert_eq!(value["code"], "NOT_FOUND");
    assert_eq!(value["exit_code"], 1);
}

#[test]
fn normalization_repairs_flag_spelling_and_reports_note() {
    let dir = tempdir().expect("tempdir");
    let output_dir = dir.path().join("tour-output");
    let fixture = workspace_root()
        .join("fixtures")
        .join("small-session.jsonl");

    let (code, stdout, _stderr) = run_panopticon(&[
        "--json",
        "tour",
        &fixture.display().to_string(),
        "--stress",
        "--output_dir",
        &output_dir.display().to_string(),
    ]);
    assert_eq!(
        code, 0,
        "flag-shape repair should preserve successful execution"
    );
    let value = parse_json(&stdout);
    assert_eq!(value["ok"], true);
    assert_eq!(value["code"], "OK");
    let notes = value["notes"].as_array().expect("notes array");
    assert!(
        notes
            .iter()
            .any(|v| v.as_str() == Some("normalized `--output_dir` -> `--output-dir`")),
        "expected normalization note in response"
    );
}

#[test]
fn normalization_never_mutates_positionals_after_double_dash() {
    let (code, stdout, _stderr) = run_panopticon(&["--json", "view", "--", "--output_dir"]);
    assert_eq!(
        code, 1,
        "path after -- should remain positional and fail as not found"
    );
    let value = parse_json(&stdout);
    assert_eq!(value["code"], "NOT_FOUND");
    assert!(
        value["message"]
            .as_str()
            .map(|m| m.contains("--output_dir"))
            .unwrap_or(false),
        "error message should preserve original positional value"
    );
}

#[test]
fn human_flag_overrides_auto_json_when_stdout_is_not_tty() {
    let (code, stdout, _stderr) = run_panopticon(&["--human", "view", "does-not-exist.jsonl"]);
    assert_eq!(
        code, 1,
        "--human view on missing file should return not-found exit code"
    );
    assert!(
        !stdout.trim_start().starts_with('{'),
        "--human should force text output in non-tty mode"
    );
    assert!(
        stdout.trim().is_empty(),
        "human errors should be emitted on stderr, not stdout"
    );
}

#[test]
fn alias_and_repaired_flag_work_together_with_global_flag_prefix() {
    let dir = tempdir().expect("tempdir");
    let output_dir = dir.path().join("tour-output");
    let fixture = workspace_root()
        .join("fixtures")
        .join("small-session.jsonl");

    let (code, stdout, _stderr) = run_panopticon(&[
        "--json",
        "tours",
        &fixture.display().to_string(),
        "--stress",
        "--output_dir",
        &output_dir.display().to_string(),
    ]);
    assert_eq!(code, 0, "alias + repair path should succeed");
    let value = parse_json(&stdout);
    assert_eq!(value["ok"], true);
    assert_eq!(value["code"], "OK");
    assert_eq!(value["command"], "tour");
    assert!(value["notes"]
        .as_array()
        .expect("notes")
        .iter()
        .any(|v| v.as_str() == Some("normalized `--output_dir` -> `--output-dir`")));
}

#[test]
fn global_json_flag_ordering_before_or_after_subcommand_is_equivalent() {
    let (code_a, stdout_a, _stderr_a) = run_panopticon(&["--json", "view", "does-not-exist.jsonl"]);
    let (code_b, stdout_b, _stderr_b) = run_panopticon(&["view", "does-not-exist.jsonl", "--json"]);
    assert_eq!(code_a, 1);
    assert_eq!(code_b, 1);

    let a = parse_json(&stdout_a);
    let b = parse_json(&stdout_b);
    assert_eq!(a["code"], "NOT_FOUND");
    assert_eq!(b["code"], "NOT_FOUND");
    assert_eq!(a["message"], b["message"]);
    assert_eq!(a["exit_code"], b["exit_code"]);
}
