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
    assert!(value["message"].is_string());
    assert!(value["suggestions"].is_array());
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
