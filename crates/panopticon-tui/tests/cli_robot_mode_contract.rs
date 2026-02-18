use serde_json::Value;
use std::fs;
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

fn canonical_json(value: &Value) -> String {
    serde_json::to_string(value).expect("json serialize")
}

fn read_json_file(path: &Path) -> Value {
    let body = fs::read_to_string(path).expect("expected readable JSON file");
    serde_json::from_str(&body).expect("expected valid JSON file")
}

fn assert_robot_envelope_shape(value: &Value) {
    let obj = value.as_object().expect("root object");
    assert!(obj.contains_key("schema_version"));
    assert!(obj.contains_key("ok"));
    assert!(obj.contains_key("code"));
    assert!(obj.contains_key("message"));
    assert!(obj.contains_key("suggestions"));
    assert!(obj.contains_key("exit_code"));
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root must exist")
        .to_path_buf()
}

fn write_compare_eventlogs() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
    let dir = tempdir().expect("tempdir");
    let source = workspace_root()
        .join("docs")
        .join("assets")
        .join("readme")
        .join("sample-export-clean-eventlog.jsonl");
    let baseline = fs::read_to_string(&source).expect("read sample eventlog");

    let left = dir.path().join("left.jsonl");
    let right_same = dir.path().join("right-same.jsonl");
    let right_diff = dir.path().join("right-diff.jsonl");

    fs::write(&left, &baseline).expect("write left");
    fs::write(&right_same, &baseline).expect("write right same");

    let mutated = baseline.replace("\"result\":\"ok\"", "\"result\":\"different\"");
    assert_ne!(baseline, mutated, "fixture mutation must change eventlog");
    fs::write(&right_diff, mutated).expect("write right diff");

    (dir, left, right_same, right_diff)
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
fn compare_no_diff_emits_ok_contract() {
    let (_dir, left, right_same, _right_diff) = write_compare_eventlogs();

    let (code, stdout, _stderr) = run_panopticon(&[
        "--json",
        "compare",
        &left.display().to_string(),
        &right_same.display().to_string(),
    ]);
    assert_eq!(code, 0, "identical runs should return success");
    let value = parse_json(&stdout);
    assert_eq!(value["schema_version"], "panopticon-cli-robot-v1.1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["code"], "OK");
    assert_eq!(value["command"], "compare");
    assert_eq!(value["exit_code"], 0);
    assert_eq!(value["data"]["status"], "NO_DIFF");
    assert_eq!(value["data"]["delta"]["divergences"], serde_json::json!([]));
    assert!(value["data"]["replay_commands"].is_array());
}

#[test]
fn compare_divergence_emits_diff_found_contract() {
    let (_dir, left, _right_same, right_diff) = write_compare_eventlogs();

    let (code, stdout, _stderr) = run_panopticon(&[
        "--json",
        "compare",
        &left.display().to_string(),
        &right_diff.display().to_string(),
    ]);
    assert_eq!(code, 5, "divergence should map to DiffFound exit code");
    let value = parse_json(&stdout);
    assert_eq!(value["schema_version"], "panopticon-cli-robot-v1.1");
    assert_eq!(value["ok"], false);
    assert_eq!(value["code"], "DIFF_FOUND");
    assert_eq!(value["command"], "compare");
    assert_eq!(value["exit_code"], 5);
    assert_eq!(value["data"]["status"], "DIFF_FOUND");
    assert!(
        value["data"]["divergence_count"]
            .as_u64()
            .is_some_and(|v| v >= 1),
        "expected at least one divergence"
    );
    assert!(value["data"]["delta"]["divergences"].is_array());
}

#[test]
fn incident_pack_success_emits_manifest_and_hashes() {
    let compare_dir = tempdir().expect("tempdir");
    let (_dir, left, right_same, _right_diff) = write_compare_eventlogs();
    let output_dir = compare_dir.path().join("incident-pack");

    let (code, stdout, _stderr) = run_panopticon(&[
        "--json",
        "incident-pack",
        &left.display().to_string(),
        &right_same.display().to_string(),
        "--output-dir",
        &output_dir.display().to_string(),
    ]);
    assert_eq!(code, 0, "incident pack should succeed for clean inputs");
    let value = parse_json(&stdout);
    assert_eq!(value["ok"], true);
    assert_eq!(value["command"], "incident-pack");
    assert_eq!(value["exit_code"], 0);
    let manifest_path = value["data"]["manifest_path"]
        .as_str()
        .expect("manifest path string");
    let manifest = read_json_file(Path::new(manifest_path));
    assert_eq!(manifest["schema_version"], "panopticon-incident-pack-v1");
    let files = manifest["files"]
        .as_object()
        .expect("manifest files must be object");
    for required in [
        "normalized/left.eventlog.jsonl",
        "normalized/right.eventlog.jsonl",
        "compare/delta.json",
        "replay/left.replay.json",
        "replay/right.replay.json",
        "export/left.bundle.tar.zst",
        "export/right.bundle.tar.zst",
    ] {
        let hash_entry = files.get(required);
        assert!(hash_entry.is_some(), "missing hash entry for {required}");
        let hash = hash_entry.and_then(Value::as_str).unwrap_or("");
        assert!(
            !hash.trim().is_empty(),
            "hash for {required} must be non-empty"
        );
    }

    let delta = read_json_file(&output_dir.join("compare").join("delta.json"));
    assert!(
        delta.as_object().is_some_and(|obj| !obj.is_empty()),
        "delta.json must not be an empty placeholder object"
    );
    assert!(delta["left_run_id"].is_string());
    assert!(delta["right_run_id"].is_string());
    assert!(delta["left_event_count"].is_number());
    assert!(delta["right_event_count"].is_number());
    assert!(delta["divergences"].is_array());

    let left_replay = read_json_file(&output_dir.join("replay").join("left.replay.json"));
    let right_replay = read_json_file(&output_dir.join("replay").join("right.replay.json"));
    for (name, replay) in [
        ("left.replay.json", left_replay),
        ("right.replay.json", right_replay),
    ] {
        assert!(
            replay.as_object().is_some_and(|obj| !obj.is_empty()),
            "{name} must not be an empty placeholder object"
        );
        assert!(replay["event_count"].is_number(), "{name}: event_count");
        assert!(replay["state_hash"].is_string(), "{name}: state_hash");
        assert!(
            replay["viewmodel_hash"].is_string(),
            "{name}: viewmodel_hash"
        );
        assert!(
            replay["projection_invariants_version"].is_string(),
            "{name}: projection_invariants_version"
        );
        assert!(
            replay["degradation_level"].is_string(),
            "{name}: degradation_level"
        );
        assert!(replay["tier_a_drops"].is_number(), "{name}: tier_a_drops");
        assert!(
            replay["queue_pressure"].is_number(),
            "{name}: queue_pressure"
        );
    }
}

#[test]
fn incident_pack_refuses_when_secrets_detected() {
    let out = tempdir().expect("tempdir");
    let left_refusal_fixture = workspace_root()
        .join("docs")
        .join("assets")
        .join("readme")
        .join("sample-refusal-eventlog.jsonl");
    let right_clean = workspace_root()
        .join("docs")
        .join("assets")
        .join("readme")
        .join("sample-export-clean-eventlog.jsonl");
    let output_dir = out.path().join("incident-pack");

    let (code, stdout, _stderr) = run_panopticon(&[
        "--json",
        "incident-pack",
        &left_refusal_fixture.display().to_string(),
        &right_clean.display().to_string(),
        "--output-dir",
        &output_dir.display().to_string(),
    ]);
    assert_eq!(code, 3, "secret findings should fail closed");
    let value = parse_json(&stdout);
    assert_eq!(value["ok"], false);
    assert_eq!(value["code"], "EXPORT_REFUSED");
    assert_eq!(value["exit_code"], 3);
}

#[test]
fn incident_pack_human_reports_runtime_error_when_output_dir_invalid() {
    let tmp = tempdir().expect("tempdir");
    let (_dir, left, right_same, _right_diff) = write_compare_eventlogs();
    let blocked_path = tmp.path().join("already-a-file");
    fs::write(&blocked_path, "occupied").expect("create blocking file");

    let (code, stdout, stderr) = run_panopticon(&[
        "--human",
        "incident-pack",
        &left.display().to_string(),
        &right_same.display().to_string(),
        "--output-dir",
        &blocked_path.display().to_string(),
    ]);
    assert_eq!(
        code, 4,
        "invalid output-dir target should fail as runtime error"
    );
    assert!(
        stdout.trim().is_empty(),
        "human-mode errors should not emit JSON/text payload to stdout"
    );
    assert!(
        stderr.contains("incident-pack failed"),
        "stderr should include failure headline"
    );
    assert!(
        stderr.contains("panopticon incident-pack"),
        "stderr should include actionable suggested command"
    );
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

#[test]
fn invalid_subcommand_envelope_matches_golden_shape() {
    let (_, stdout, _) = run_panopticon(&["bogus-subcommand"]);
    let value = parse_json(&stdout);
    let expected = serde_json::json!({
        "schema_version": "panopticon-cli-robot-v1.1",
        "ok": false,
        "code": "INVALID_ARGS",
        "message": "Unknown subcommand.",
        "suggestions": [
            "Use one of: `panopticon view`, `panopticon export`, `panopticon tour`, `panopticon compare`, or `panopticon incident-pack`.",
            "Run `panopticon --help` for full command syntax."
        ],
        "exit_code": 2
    });
    assert_eq!(canonical_json(&value), canonical_json(&expected));
}

#[test]
fn missing_required_argument_envelope_matches_golden_shape() {
    let (_, stdout, _) = run_panopticon(&["--json", "export"]);
    let value = parse_json(&stdout);
    let expected = serde_json::json!({
        "schema_version": "panopticon-cli-robot-v1.1",
        "ok": false,
        "code": "INVALID_ARGS",
        "message": "Missing required argument.",
        "suggestions": [
            "Example: `panopticon view <eventlog.jsonl>`.",
            "Example: `panopticon export <eventlog.jsonl> --share-safe --output <bundle.tar.zst>`."
        ],
        "exit_code": 2
    });
    assert_eq!(canonical_json(&value), canonical_json(&expected));
}

#[test]
fn unknown_argument_envelope_is_deterministic_and_actionable() {
    let (_, stdout, _) = run_panopticon(&["--json", "--bogus-flag", "view", "x.jsonl"]);
    let value = parse_json(&stdout);
    assert_robot_envelope_shape(&value);
    assert_eq!(value["ok"], false);
    assert_eq!(value["code"], "INVALID_ARGS");
    assert_eq!(value["message"], "Unknown flag or option.");
    assert!(
        value["suggestions"]
            .as_array()
            .expect("suggestions")
            .iter()
            .any(|s| s
                .as_str()
                .is_some_and(|line| line.contains("<command> --help"))),
        "unknown-argument path should include command-specific help hint"
    );
}

#[test]
fn human_invalid_args_include_replay_hints() {
    let (code, stdout, stderr) = run_panopticon(&["--human", "bogus-subcommand"]);
    assert_eq!(code, 2);
    assert!(stdout.trim().is_empty(), "human errors should be on stderr");
    assert!(stderr.contains("Hint 1:"));
    assert!(stderr.contains("panopticon view"));
}
