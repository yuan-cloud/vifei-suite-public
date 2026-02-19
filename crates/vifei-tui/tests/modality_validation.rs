use std::fs;
use std::path::Path;
use tempfile::tempdir;
use vifei_core::event::{EventPayload, ImportEvent, Tier};
use vifei_core::eventlog::EventLogWriter;
use vifei_tui::{render_forensic_multiline, render_incident_multiline};

fn make_fixture(path: &Path) {
    let mut writer = EventLogWriter::open(path).expect("open eventlog fixture");
    let mut ts = 1_700_000_000_000_000_000u64;
    for (seq, payload) in [
        EventPayload::RunStart {
            agent: "modality-agent".into(),
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
                run_id: "run-modality".into(),
                event_id: format!("ev-{}", seq + 1),
                source_id: "modality-test".into(),
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

#[test]
fn width_buckets_preserve_required_surface_markers() {
    let dir = tempdir().expect("tempdir");
    let fixture = dir.path().join("modality.jsonl");
    make_fixture(&fixture);

    let widths = [140u16, 120, 100, 80, 72];
    for width in widths {
        let incident =
            render_incident_multiline(&fixture, width, 28).expect("render incident modality");
        assert!(
            incident.contains("Incident Lens"),
            "missing Incident Lens marker at width={width}"
        );
        assert!(
            incident.contains("Action Now"),
            "missing anomaly triage section at width={width}"
        );
        assert!(
            incident.contains("Next action:"),
            "missing next-action hint at width={width}"
        );
        assert!(
            incident.contains("Level:"),
            "missing Truth HUD level at width={width}"
        );
        assert!(
            incident.contains("Version:"),
            "missing Truth HUD version line at width={width}"
        );

        let forensic =
            render_forensic_multiline(&fixture, width, 28).expect("render forensic modality");
        assert!(
            forensic.contains("Forensic Lens"),
            "missing Forensic Lens marker at width={width}"
        );
        assert!(
            forensic.contains("Timeline"),
            "missing timeline pane at width={width}"
        );
        assert!(
            forensic.contains("Inspector"),
            "missing inspector pane at width={width}"
        );
        assert!(
            forensic.contains("Level:"),
            "missing Truth HUD level in forensic at width={width}"
        );
    }
}

#[test]
fn readme_mobile_order_and_command_width_contract() {
    let readme_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("README.md");
    let readme = fs::read_to_string(&readme_path).expect("read README");
    let required_order = [
        "## Why This Exists",
        "## 60-Second Quickstart",
        "## Trust Signals (What You Can Verify Yourself)",
        "## Architecture Snapshot",
        "## Troubleshooting",
    ];

    let mut last = 0usize;
    for heading in required_order {
        let idx = readme.find(heading);
        assert!(idx.is_some(), "missing required heading: {heading}");
        let idx = idx.unwrap_or_default();
        assert!(
            idx >= last,
            "heading order regression: {heading} appears before prior required section"
        );
        last = idx;
    }

    let mut in_bash = false;
    let mut line_no = 0usize;
    for line in readme.lines() {
        line_no += 1;
        if line.trim_start().starts_with("```bash") {
            in_bash = true;
            continue;
        }
        if in_bash && line.trim_start().starts_with("```") {
            in_bash = false;
            continue;
        }
        if in_bash {
            assert!(
                line.len() <= 120,
                "mobile readability: command line too long at {}:{line_no} (len={})",
                readme_path.display(),
                line.len()
            );
        }
    }
}
