use panopticon_core::event::{EventPayload, ImportEvent, Tier};
use panopticon_core::eventlog::EventLogWriter;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output, Stdio};
use tempfile::tempdir;

const MAX_RETRIES: usize = 1;

#[derive(Debug)]
struct SessionRun {
    status: ExitStatus,
    transcript_path: PathBuf,
    transcript: String,
    stderr: String,
}

fn test_out_dir() -> PathBuf {
    let root = env::var("PANOPTICON_E2E_OUT").unwrap_or_else(|_| ".tmp/e2e/tui".to_string());
    let path = PathBuf::from(root);
    fs::create_dir_all(&path).expect("create e2e output dir");
    path
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

fn preflight_pty_support() -> Result<(), String> {
    if !script_available() {
        return Err("util-linux `script` command is unavailable".to_string());
    }

    let out_dir = test_out_dir();
    let probe_path = out_dir.join("pty-preflight.typescript");
    let output = Command::new("script")
        .arg("-qefc")
        .arg("true")
        .arg(&probe_path)
        .output()
        .map_err(|e| format!("failed to execute `script`: {e}"))?;

    if output.status.success() {
        let _ = fs::remove_file(probe_path);
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Err(format!(
        "PTY preflight failed (exit={:?}) stderr={} stdout={}",
        output.status.code(),
        stderr,
        stdout
    ))
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
    let bin = env!("CARGO_BIN_EXE_panopticon");
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
) -> SessionRun {
    let out_dir = test_out_dir();
    let input_path = out_dir.join(format!("{test_name}.keys"));
    fs::write(&input_path, key_bytes).expect("write scripted key input");
    let assertions_log = out_dir.join(format!("{test_name}.assertions.log"));

    let first = run_once(test_name, 1, fixture, columns, lines, &input_path);
    if first.status.success() {
        fs::write(
            assertions_log,
            format!(
                "status=pass attempt=1 transcript={}\n",
                first.transcript_path.display()
            ),
        )
        .expect("write assertion log");
        return first;
    }

    fs::write(
        &assertions_log,
        format!(
            "status=fail attempt=1 transcript={}\nexit={:?}\nstderr={}\n",
            first.transcript_path.display(),
            first.status.code(),
            first.stderr.trim()
        ),
    )
    .expect("write first-failure assertion log");

    if MAX_RETRIES == 0 {
        return first;
    }

    let second = run_once(test_name, 2, fixture, columns, lines, &input_path);
    let summary = format!(
        "status={} attempt=2 transcript={}\nexit={:?}\nstderr={}\n",
        if second.status.success() {
            "pass"
        } else {
            "fail"
        },
        second.transcript_path.display(),
        second.status.code(),
        second.stderr.trim()
    );
    fs::write(assertions_log, summary).expect("write retry assertion log");
    second
}

#[test]
fn interactive_tui_flow_lens_toggle_nav_and_quit() {
    if let Err(reason) = preflight_pty_support() {
        let out_dir = test_out_dir();
        let log = out_dir.join("interactive_tui_flow_lens_toggle_nav_and_quit.assertions.log");
        let _ = fs::write(&log, format!("status=skip reason={reason}\n"));
        eprintln!("SKIP: {reason}");
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
    if let Err(reason) = preflight_pty_support() {
        let out_dir = test_out_dir();
        let log =
            out_dir.join("interactive_tui_narrow_terminal_profile_stays_healthy.assertions.log");
        let _ = fs::write(&log, format!("status=skip reason={reason}\n"));
        eprintln!("SKIP: {reason}");
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
