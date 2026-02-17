use serde_json::Value;
use std::process::Command;

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
