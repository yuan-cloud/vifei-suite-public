use panopticon_core::event::{EventPayload, ImportEvent, Tier};
use panopticon_core::eventlog::EventLogWriter;
use panopticon_core::projection::LadderLevel;
use panopticon_export::{run_export, ExportConfig, ExportResult};
use panopticon_tour::{run_tour, TourConfig};
use panopticon_tui::{
    render_degraded_incident_multiline, render_degraded_incident_multiline_with_profile,
    render_forensic_multiline, render_forensic_multiline_with_profile, render_incident_multiline,
    render_incident_multiline_with_profile, UiProfile,
};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn main() -> io::Result<()> {
    let root = std::env::current_dir()?;
    let out_dir = root.join("docs/assets/readme");
    fs::create_dir_all(&out_dir)?;

    let eventlog_path = out_dir.join("sample-eventlog.jsonl");
    write_sample_eventlog(&eventlog_path)?;
    write_sample_export_clean_eventlog(&out_dir.join("sample-export-clean-eventlog.jsonl"))?;

    let incident = render_incident_multiline(&eventlog_path, 120, 36)?;
    fs::write(out_dir.join("incident-lens.txt"), incident)?;

    let incident_narrow = render_incident_multiline(&eventlog_path, 72, 28)?;
    fs::write(out_dir.join("incident-lens-narrow-72.txt"), incident_narrow)?;

    let forensic = render_forensic_multiline(&eventlog_path, 120, 36)?;
    fs::write(out_dir.join("forensic-lens.txt"), forensic)?;

    let degraded = render_degraded_incident_multiline(&eventlog_path, 120, 36, LadderLevel::L3)?;
    fs::write(out_dir.join("truth-hud-degraded.txt"), degraded)?;

    let incident_showcase =
        render_incident_multiline_with_profile(&eventlog_path, 120, 36, UiProfile::Showcase)?;
    fs::write(
        out_dir.join("incident-lens-showcase.txt"),
        incident_showcase,
    )?;

    let forensic_showcase =
        render_forensic_multiline_with_profile(&eventlog_path, 120, 36, UiProfile::Showcase)?;
    fs::write(
        out_dir.join("forensic-lens-showcase.txt"),
        forensic_showcase,
    )?;

    let degraded_showcase = render_degraded_incident_multiline_with_profile(
        &eventlog_path,
        120,
        36,
        LadderLevel::L3,
        UiProfile::Showcase,
    )?;
    fs::write(out_dir.join("truth-hud-showcase.txt"), degraded_showcase)?;

    let refusal = generate_export_refusal(&out_dir)?;
    fs::write(out_dir.join("export-refusal.txt"), refusal)?;

    let artifacts = generate_tour_artifacts_view(&root, &out_dir)?;
    fs::write(out_dir.join("artifacts-view.txt"), artifacts)?;

    fs::write(out_dir.join("architecture.mmd"), architecture_mermaid())?;
    fs::write(out_dir.join("README.md"), asset_index_markdown())?;

    println!("Wrote README assets under {}", out_dir.display());
    Ok(())
}

fn write_sample_eventlog(path: &Path) -> io::Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    let mut writer = EventLogWriter::open(path)?;
    for event in sample_events() {
        writer.append(event)?;
    }
    Ok(())
}

fn sample_events() -> Vec<ImportEvent> {
    vec![
        ImportEvent {
            run_id: "run-readme-1".into(),
            event_id: "ev-1".into(),
            source_id: "readme-capture".into(),
            source_seq: Some(1),
            timestamp_ns: 1_700_000_000_000_000_000,
            tier: Tier::A,
            payload: EventPayload::RunStart {
                agent: "codex".into(),
                args: Some("capture-assets --deterministic".into()),
            },
            payload_ref: None,
            synthesized: false,
        },
        ImportEvent {
            run_id: "run-readme-1".into(),
            event_id: "ev-2".into(),
            source_id: "readme-capture".into(),
            source_seq: Some(2),
            timestamp_ns: 1_700_000_000_010_000_000,
            tier: Tier::A,
            payload: EventPayload::ToolCall {
                tool: "cargo test".into(),
                args: Some("--workspace".into()),
            },
            payload_ref: None,
            synthesized: false,
        },
        ImportEvent {
            run_id: "run-readme-1".into(),
            event_id: "ev-3".into(),
            source_id: "readme-capture".into(),
            source_seq: Some(3),
            timestamp_ns: 1_700_000_000_020_000_000,
            tier: Tier::A,
            payload: EventPayload::ToolResult {
                tool: "cargo test".into(),
                result: Some("all tests passed".into()),
                status: Some("success".into()),
            },
            payload_ref: None,
            synthesized: false,
        },
        ImportEvent {
            run_id: "run-readme-1".into(),
            event_id: "ev-4".into(),
            source_id: "readme-capture".into(),
            source_seq: Some(4),
            timestamp_ns: 1_700_000_000_030_000_000,
            tier: Tier::A,
            payload: EventPayload::PolicyDecision {
                from_level: "L0".into(),
                to_level: "L2".into(),
                trigger: "QueuePressure".into(),
                queue_pressure: 0.82,
            },
            payload_ref: None,
            synthesized: false,
        },
        ImportEvent {
            run_id: "run-readme-1".into(),
            event_id: "ev-5".into(),
            source_id: "readme-capture".into(),
            source_seq: Some(5),
            timestamp_ns: 1_700_000_000_040_000_000,
            tier: Tier::A,
            payload: EventPayload::Error {
                kind: "io".into(),
                message: "transient write stall".into(),
                severity: Some("warning".into()),
            },
            payload_ref: None,
            synthesized: false,
        },
        ImportEvent {
            run_id: "run-readme-1".into(),
            event_id: "ev-6".into(),
            source_id: "readme-capture".into(),
            source_seq: Some(6),
            timestamp_ns: 1_700_000_000_050_000_000,
            tier: Tier::A,
            payload: EventPayload::ClockSkewDetected {
                expected_ns: 1_700_000_000_050_000_000,
                actual_ns: 1_700_000_000_049_900_000,
                delta_ns: 100_000,
            },
            payload_ref: None,
            synthesized: false,
        },
        ImportEvent {
            run_id: "run-readme-1".into(),
            event_id: "ev-7".into(),
            source_id: "readme-capture".into(),
            source_seq: Some(7),
            timestamp_ns: 1_700_000_000_060_000_000,
            tier: Tier::A,
            payload: EventPayload::RedactionApplied {
                target_event_id: "ev-2".into(),
                field_path: "payload.args".into(),
                reason: "secret token removed".into(),
            },
            payload_ref: None,
            synthesized: false,
        },
        ImportEvent {
            run_id: "run-readme-1".into(),
            event_id: "ev-8".into(),
            source_id: "readme-capture".into(),
            source_seq: Some(8),
            timestamp_ns: 1_700_000_000_070_000_000,
            tier: Tier::A,
            payload: EventPayload::RunEnd {
                exit_code: Some(0),
                reason: Some("done".into()),
            },
            payload_ref: None,
            synthesized: false,
        },
    ]
}

fn write_sample_export_clean_eventlog(path: &Path) -> io::Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    let mut writer = EventLogWriter::open(path)?;
    let events = vec![
        ImportEvent {
            run_id: "run-export-clean".into(),
            event_id: "clean-1".into(),
            source_id: "readme-capture".into(),
            source_seq: Some(1),
            timestamp_ns: 1000,
            tier: Tier::A,
            payload: EventPayload::RunStart {
                agent: "demo".into(),
                args: Some("check".into()),
            },
            payload_ref: None,
            synthesized: false,
        },
        ImportEvent {
            run_id: "run-export-clean".into(),
            event_id: "clean-2".into(),
            source_id: "readme-capture".into(),
            source_seq: Some(2),
            timestamp_ns: 2000,
            tier: Tier::A,
            payload: EventPayload::ToolResult {
                tool: "verify".into(),
                result: Some("ok".into()),
                status: Some("success".into()),
            },
            payload_ref: None,
            synthesized: false,
        },
        ImportEvent {
            run_id: "run-export-clean".into(),
            event_id: "clean-3".into(),
            source_id: "readme-capture".into(),
            source_seq: Some(3),
            timestamp_ns: 3000,
            tier: Tier::A,
            payload: EventPayload::RunEnd {
                exit_code: Some(0),
                reason: Some("done".into()),
            },
            payload_ref: None,
            synthesized: false,
        },
    ];
    for event in events {
        writer.append(event)?;
    }
    Ok(())
}

fn generate_export_refusal(out_dir: &Path) -> io::Result<String> {
    let refused_eventlog = out_dir.join("sample-refusal-eventlog.jsonl");
    if refused_eventlog.exists() {
        fs::remove_file(&refused_eventlog)?;
    }
    let mut writer = EventLogWriter::open(&refused_eventlog)?;

    writer.append(ImportEvent {
        run_id: "run-refusal-1".into(),
        event_id: "ref-1".into(),
        source_id: "readme-capture".into(),
        source_seq: Some(1),
        timestamp_ns: 1_700_000_100_000_000_000,
        tier: Tier::A,
        payload: EventPayload::ToolCall {
            tool: "openai".into(),
            args: Some("sk-0123456789abcdef0123456789abcdef0123456789abcdef".into()),
        },
        payload_ref: None,
        synthesized: false,
    })?;

    let bundle_out = out_dir.join("refusal-bundle.tar.zst");
    let refusal_report = out_dir.join("refusal-report.json");
    let config =
        ExportConfig::new(&refused_eventlog, &bundle_out).with_refusal_report(&refusal_report);

    match run_export(&config).map_err(io::Error::other)? {
        ExportResult::Success(_) => Ok("Unexpected: export succeeded".to_string()),
        ExportResult::Refused(report) => {
            let mut out = String::new();
            out.push_str(&format!("Export REFUSED: {}\n", report.summary));
            for item in &report.blocked_items {
                let location = item.blob_ref.as_ref().map_or_else(
                    || format!("event:{}", item.event_id),
                    |b| format!("blob:{}", b),
                );
                out.push_str(&format!(
                    "- {} @ {}: {} ({})\n",
                    location, item.field_path, item.matched_pattern, item.redacted_match
                ));
            }
            Ok(out)
        }
    }
}

fn generate_tour_artifacts_view(root: &Path, out_dir: &Path) -> io::Result<String> {
    let capture_dir: PathBuf = out_dir.join("tour-artifacts");
    let config =
        TourConfig::new(root.join("fixtures/large-stress.jsonl")).with_output_dir(&capture_dir);
    let result = run_tour(&config).map_err(io::Error::other)?;

    let mut out = String::new();
    out.push_str("Artifacts:\n");
    out.push_str("- metrics.json\n");
    out.push_str("- viewmodel.hash\n");
    out.push_str("- ansi.capture\n");
    out.push_str("- timetravel.capture\n\n");
    out.push_str(&format!("Events: {}\n", result.metrics.event_count_total));
    out.push_str(&format!("Tier A drops: {}\n", result.metrics.tier_a_drops));
    out.push_str(&format!(
        "Final level: {}\n",
        result.metrics.degradation_level_final
    ));
    out.push_str(&format!("Hash: {}\n", result.viewmodel_hash));
    Ok(out)
}

fn asset_index_markdown() -> String {
    [
        "# README assets",
        "",
        "Generated with:",
        "",
        "```bash",
        "cargo run -p panopticon-tui --bin capture_readme_assets",
        "```",
        "",
        "Files:",
        "- incident-lens.txt",
        "- incident-lens-showcase.txt",
        "- incident-lens-narrow-72.txt",
        "- forensic-lens.txt",
        "- forensic-lens-showcase.txt",
        "- truth-hud-degraded.txt",
        "- truth-hud-showcase.txt",
        "- export-refusal.txt",
        "- refusal-report.json",
        "- sample-export-clean-eventlog.jsonl",
        "- artifacts-view.txt",
        "- architecture.mmd",
        "- tour-artifacts/",
    ]
    .join("\n")
}

fn architecture_mermaid() -> String {
    [
        "flowchart TD",
        "    A[Agent Cassette JSONL] --> B[Importer]",
        "    B --> C[Append Writer<br/>assigns commit_index]",
        "    C --> D[EventLog JSONL + Blob Store]",
        "    D --> E[Reducer]",
        "    E --> F[Projection]",
        "    F --> G[ViewModel]",
        "    G --> H[Incident Lens + Forensic Lens + Truth HUD]",
        "    D --> I[Tour stress harness]",
        "    I --> J[metrics.json]",
        "    I --> K[viewmodel.hash]",
        "    I --> L[ansi.capture]",
        "    I --> M[timetravel.capture]",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use panopticon_core::eventlog::read_eventlog;

    #[test]
    fn sample_events_are_source_ordered_and_tier_a() {
        let events = sample_events();
        assert_eq!(events.len(), 8);
        for (idx, ev) in events.iter().enumerate() {
            assert_eq!(ev.source_seq, Some((idx + 1) as u64));
            assert_eq!(ev.tier, Tier::A);
        }
    }

    #[test]
    fn write_sample_export_clean_eventlog_roundtrips() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("clean.jsonl");
        write_sample_export_clean_eventlog(&path).expect("write clean eventlog");

        let committed = read_eventlog(&path).expect("read committed eventlog");
        assert_eq!(committed.len(), 3);
        assert_eq!(committed[0].event_id, "clean-1");
        assert_eq!(committed[2].event_id, "clean-3");
    }

    #[test]
    fn asset_index_lists_expected_files() {
        let index = asset_index_markdown();
        assert!(index.contains("incident-lens.txt"));
        assert!(index.contains("incident-lens-showcase.txt"));
        assert!(index.contains("incident-lens-narrow-72.txt"));
        assert!(index.contains("forensic-lens.txt"));
        assert!(index.contains("forensic-lens-showcase.txt"));
        assert!(index.contains("truth-hud-degraded.txt"));
        assert!(index.contains("truth-hud-showcase.txt"));
        assert!(index.contains("tour-artifacts/"));
    }

    #[test]
    fn generate_export_refusal_is_deterministic_and_structured() {
        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path();

        let first = generate_export_refusal(out).expect("first refusal output");
        let second = generate_export_refusal(out).expect("second refusal output");

        assert_eq!(first, second, "refusal output must be deterministic");
        assert!(
            first.contains("Export REFUSED:"),
            "missing refusal header in output"
        );
        assert!(
            first.contains("event:ref-1 @ payload"),
            "missing blocked item location/field path details"
        );
        assert!(
            first.contains("openai_key"),
            "missing matched pattern detail"
        );
    }
}
