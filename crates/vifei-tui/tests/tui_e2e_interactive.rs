use serde_json::json;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output, Stdio};
use tempfile::tempdir;
use vifei_core::event::{EventPayload, ImportEvent, Tier};
use vifei_core::eventlog::EventLogWriter;

const MAX_RETRIES: usize = 1;

#[derive(Debug)]
struct SessionRun {
    status: ExitStatus,
    transcript_path: PathBuf,
    transcript: String,
    stderr: String,
}

#[derive(Debug)]
struct PreflightFailure {
    reason_code: &'static str,
    message: String,
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root must be two levels above crate manifest dir")
        .to_path_buf()
}

fn resolve_out_dir(raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        workspace_root().join(path)
    }
}

fn test_out_dir() -> PathBuf {
    let raw = env::var("VIFEI_E2E_OUT").unwrap_or_else(|_| ".tmp/e2e/tui".to_string());
    let path = resolve_out_dir(&raw);
    fs::create_dir_all(&path).expect("create e2e output dir");
    path
}

#[test]
fn relative_vifei_e2e_out_resolves_from_workspace_root() {
    let out = resolve_out_dir(".tmp/e2e/tui");
    let expected = workspace_root().join(".tmp/e2e/tui");
    assert_eq!(out, expected);
}

fn script_available() -> bool {
    Command::new("script")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn preflight_pty_support() -> Result<(), PreflightFailure> {
    if !script_available() {
        return Err(PreflightFailure {
            reason_code: "PTY_SCRIPT_UNAVAILABLE",
            message: "util-linux `script` command is unavailable".to_string(),
        });
    }

    let out_dir = test_out_dir();
    let probe_path = out_dir.join("pty-preflight.typescript");
    let output = Command::new("script")
        .arg("-qefc")
        .arg("true")
        .arg(&probe_path)
        .output()
        .map_err(|e| PreflightFailure {
            reason_code: "PTY_SCRIPT_EXEC_FAILED",
            message: format!("failed to execute `script`: {e}"),
        })?;

    if output.status.success() {
        let _ = fs::remove_file(probe_path);
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Err(PreflightFailure {
        reason_code: "PTY_ALLOCATION_DENIED",
        message: format!(
            "PTY preflight failed (exit={:?}) stderr={} stdout={}",
            output.status.code(),
            stderr,
            stdout
        ),
    })
}

fn shell_escape(arg: &str) -> String {
    let escaped = arg.replace('\'', "'\"'\"'");
    format!("'{}'", escaped)
}

fn make_fixture(path: &Path) {
    let mut writer = EventLogWriter::open(path).expect("open eventlog fixture");
    let mut ts = 1_700_000_000_000_000_000u64;
    for (seq, payload) in [
        EventPayload::RunStart {
            agent: "e2e-agent".into(),
            args: Some("demo".into()),
        },
        EventPayload::ToolCall {
            tool: "cargo test".into(),
            args: Some("--workspace".into()),
        },
        EventPayload::ToolResult {
            tool: "cargo test".into(),
            result: Some("ok".into()),
            status: Some("success".into()),
        },
        EventPayload::PolicyDecision {
            from_level: "L0".into(),
            to_level: "L2".into(),
            trigger: "QueuePressure".into(),
            queue_pressure: 0.81,
        },
        EventPayload::Error {
            kind: "io".into(),
            message: "transient stall".into(),
            severity: Some("warning".into()),
        },
        EventPayload::RunEnd {
            exit_code: Some(0),
            reason: Some("done".into()),
        },
    ]
    .into_iter()
    .enumerate()
    {
        writer
            .append(ImportEvent {
                run_id: "run-e2e-tui".into(),
                event_id: format!("ev-{}", seq + 1),
                source_id: "tui-e2e".into(),
                source_seq: Some((seq + 1) as u64),
                timestamp_ns: ts,
                tier: Tier::A,
                payload,
                payload_ref: None,
                synthesized: false,
            })
            .expect("append fixture event");
        ts += 1_000_000;
    }
}

fn run_once(
    test_name: &str,
    attempt: usize,
    fixture: &Path,
    columns: u16,
    lines: u16,
    input_path: &Path,
) -> SessionRun {
    let out_dir = test_out_dir();
    let transcript_path = out_dir.join(format!("{test_name}.attempt{attempt}.typescript"));
    let bin = env!("CARGO_BIN_EXE_vifei");
    let command = format!(
        "env COLUMNS={} LINES={} TERM=xterm-256color {} view {}",
        columns,
        lines,
        shell_escape(bin),
        shell_escape(&fixture.display().to_string())
    );

    let input = fs::File::open(input_path).expect("open scripted input");
    let output: Output = Command::new("script")
        .arg("-qefc")
        .arg(command)
        .arg(&transcript_path)
        .stdin(Stdio::from(input))
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("run script PTY command");

    let transcript = fs::read_to_string(&transcript_path).unwrap_or_default();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    SessionRun {
        status: output.status,
        transcript_path,
        transcript,
        stderr,
    }
}

fn run_with_retry(
    test_name: &str,
    fixture: &Path,
    columns: u16,
    lines: u16,
    key_bytes: &[u8],
    validate: impl Fn(&SessionRun) -> Result<(), String>,
) -> SessionRun {
    let out_dir = test_out_dir();
    let input_path = out_dir.join(format!("{test_name}.keys"));
    fs::write(&input_path, key_bytes).expect("write scripted key input");
    let assertions_log = out_dir.join(format!("{test_name}.assertions.log"));

    let first = run_once(test_name, 1, fixture, columns, lines, &input_path);
    let first_validation = validate(&first);
    if first.status.success() && first_validation.is_ok() {
        write_assertions_log(
            &assertions_log,
            &json!({
                "schema_version": "vifei-tui-e2e-assert-v1",
                "test_name": test_name,
                "status": "pass",
                "attempt": 1,
                "first_failure_transcript": serde_json::Value::Null,
                "retry_transcript": serde_json::Value::Null,
                "final_transcript": first.transcript_path.display().to_string(),
                "exit_code": first.status.code(),
                "validation": "ok"
            }),
        );
        return first;
    }

    let first_validation_error = first_validation
        .as_ref()
        .err()
        .map(String::as_str)
        .unwrap_or("ok");

    if MAX_RETRIES == 0 {
        write_assertions_log(
            &assertions_log,
            &json!({
                "schema_version": "vifei-tui-e2e-assert-v1",
                "test_name": test_name,
                "status": "fail",
                "attempt": 1,
                "first_failure_transcript": first.transcript_path.display().to_string(),
                "retry_transcript": serde_json::Value::Null,
                "final_transcript": first.transcript_path.display().to_string(),
                "first_failure_exit_code": first.status.code(),
                "first_failure_validation": first_validation_error,
                "first_failure_stderr": first.stderr.trim()
            }),
        );
        return first;
    }

    let second = run_once(test_name, 2, fixture, columns, lines, &input_path);
    let second_validation = validate(&second);
    write_assertions_log(
        &assertions_log,
        &json!({
            "schema_version": "vifei-tui-e2e-assert-v1",
            "test_name": test_name,
            "status": if second.status.success() && second_validation.is_ok() {
                "pass"
            } else {
                "fail"
            },
            "attempt": 2,
            "first_failure_transcript": first.transcript_path.display().to_string(),
            "retry_transcript": second.transcript_path.display().to_string(),
            "final_transcript": second.transcript_path.display().to_string(),
            "first_failure_exit_code": first.status.code(),
            "first_failure_validation": first_validation_error,
            "first_failure_stderr": first.stderr.trim(),
            "retry_exit_code": second.status.code(),
            "retry_validation": second_validation
                .as_ref()
                .err()
                .map(String::as_str)
                .unwrap_or("ok"),
            "retry_stderr": second.stderr.trim()
        }),
    );
    second
}

fn write_assertions_log(path: &Path, payload: &serde_json::Value) {
    let body = serde_json::to_string(payload).expect("serialize assertion payload");
    fs::write(path, body).expect("write assertion log");
}

fn write_skip_assertions_log(test_name: &str, failure: &PreflightFailure) {
    let out_dir = test_out_dir();
    let log = out_dir.join(format!("{test_name}.assertions.log"));
    write_assertions_log(
        &log,
        &json!({
            "schema_version": "vifei-tui-e2e-assert-v1",
            "test_name": test_name,
            "status": "skip",
            "attempt": 0,
            "reason_code": failure.reason_code,
            "reason": failure.message
        }),
    );
}

#[test]
fn interactive_tui_flow_lens_toggle_nav_and_quit() {
    if let Err(failure) = preflight_pty_support() {
        write_skip_assertions_log("interactive_tui_flow_lens_toggle_nav_and_quit", &failure);
        eprintln!("SKIP [{}]: {}", failure.reason_code, failure.message);
        return;
    }

    let dir = tempdir().expect("tempdir");
    let fixture = dir.path().join("fixture.jsonl");
    make_fixture(&fixture);

    // Tab to Forensic, move down, expand, then quit.
    let run = run_with_retry(
        "interactive_tui_flow_lens_toggle_nav_and_quit",
        &fixture,
        120,
        30,
        b"\tj\nq",
        |run| {
            if !run.transcript.contains("Incident Lens") {
                return Err("missing marker: Incident Lens".to_string());
            }
            if !run.transcript.contains("Forensic Lens") {
                return Err("missing marker: Forensic Lens".to_string());
            }
            if !run.transcript.contains("Level:") {
                return Err("missing marker: Level:".to_string());
            }
            Ok(())
        },
    );

    assert!(
        run.status.success(),
        "PTY TUI session failed. transcript={} exit={:?}",
        run.transcript_path.display(),
        run.status.code()
    );
    assert!(
        run.transcript.contains("Incident Lens"),
        "Expected Incident Lens in transcript: {}",
        run.transcript_path.display()
    );
    assert!(
        run.transcript.contains("Forensic Lens"),
        "Expected Forensic Lens after Tab in transcript: {}",
        run.transcript_path.display()
    );
    assert!(
        run.transcript.contains("Level:"),
        "Truth HUD not visible in transcript: {}",
        run.transcript_path.display()
    );
}

#[test]
fn interactive_tui_narrow_terminal_profile_stays_healthy() {
    if let Err(failure) = preflight_pty_support() {
        write_skip_assertions_log(
            "interactive_tui_narrow_terminal_profile_stays_healthy",
            &failure,
        );
        eprintln!("SKIP [{}]: {}", failure.reason_code, failure.message);
        return;
    }

    let dir = tempdir().expect("tempdir");
    let fixture = dir.path().join("fixture-narrow.jsonl");
    make_fixture(&fixture);

    let run = run_with_retry(
        "interactive_tui_narrow_terminal_profile_stays_healthy",
        &fixture,
        72,
        22,
        b"\tq",
        |run| {
            if !run.transcript.contains("Version:") {
                return Err("missing marker: Version:".to_string());
            }
            Ok(())
        },
    );

    assert!(
        run.status.success(),
        "Narrow PTY TUI session failed. transcript={} exit={:?}",
        run.transcript_path.display(),
        run.status.code()
    );
    assert!(
        run.transcript.contains("Version:"),
        "Truth HUD version line missing in narrow-terminal transcript: {}",
        run.transcript_path.display()
    );
}
