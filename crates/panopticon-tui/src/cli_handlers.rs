use crate::cli_contract::{
    AppExit, Cli, Commands, CompareInputFormat, OutputMode, UiProfileArg, ROBOT_SCHEMA_VERSION,
};
use crate::cli_normalize::format_cli_failure;
use panopticon_core::delta::diff_runs;
use panopticon_core::event::CommittedEvent;
use panopticon_core::eventlog::{read_eventlog, EventLogWriter};
use panopticon_core::projection::{project, viewmodel_hash, ProjectionInvariants};
use panopticon_core::reducer::{replay, state_hash};
use panopticon_export::{ExportConfig, ExportResult};
use panopticon_import::cassette;
use panopticon_tour::TourConfig;
use panopticon_tui::{run_viewer, UiProfile};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static CASSETTE_APPEND_TEMP_ID: AtomicU64 = AtomicU64::new(0);

fn emit_json(value: Value) {
    match serde_json::to_string(&value) {
        Ok(line) => println!("{line}"),
        Err(err) => {
            // Last-resort envelope to avoid panicking in robot mode.
            let fallback = json!({
                "schema_version": ROBOT_SCHEMA_VERSION,
                "ok": false,
                "code": "RUNTIME_ERROR",
                "message": format!("failed to serialize JSON response: {err}"),
                "suggestions": [],
                "exit_code": AppExit::RuntimeError as u8,
            });
            println!("{fallback}");
        }
    }
}

pub(crate) fn emit_json_success(
    code: &str,
    message: &str,
    command: Option<&str>,
    exit_code: u8,
    notes: &[String],
    mut data: Value,
) {
    if data.is_null() {
        data = json!({});
    }
    let mut obj = json!({
        "schema_version": ROBOT_SCHEMA_VERSION,
        "ok": true,
        "code": code,
        "message": message,
        "suggestions": [],
        "exit_code": exit_code,
        "data": data,
    });
    if let Some(command) = command {
        obj["command"] = json!(command);
    }
    if !notes.is_empty() {
        obj["notes"] = json!(notes);
    }
    emit_json(obj);
}

pub(crate) fn emit_json_error(
    code: &str,
    message: &str,
    suggestions: &[String],
    notes: &[String],
    exit_code: u8,
) {
    let mut obj = json!({
        "schema_version": ROBOT_SCHEMA_VERSION,
        "ok": false,
        "code": code,
        "message": message,
        "suggestions": suggestions,
        "exit_code": exit_code,
    });
    if !notes.is_empty() {
        obj["notes"] = json!(notes);
    }
    emit_json(obj);
}

fn ensure_file_exists(path: &Path, label: &str) -> Result<(), String> {
    if path.exists() {
        Ok(())
    } else {
        Err(format!("{} not found: {}", label, path.display()))
    }
}

fn load_committed_events(
    path: &Path,
    format: CompareInputFormat,
) -> Result<Vec<CommittedEvent>, String> {
    match format {
        CompareInputFormat::Eventlog => read_eventlog(path)
            .map_err(|e| format!("failed to read eventlog {}: {e}", path.display())),
        CompareInputFormat::Cassette => {
            let file = File::open(path)
                .map_err(|e| format!("failed to open cassette {}: {e}", path.display()))?;
            let reader = BufReader::new(file);
            let imported = cassette::parse_cassette(reader);
            let temp_id = CASSETTE_APPEND_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let eventlog_path = std::env::temp_dir().join(format!(
                "panopticon-cassette-canonical-{}-{temp_id}.jsonl",
                std::process::id()
            ));
            let mut writer = EventLogWriter::open(&eventlog_path).map_err(|e| {
                format!(
                    "failed to initialize append writer for {}: {e}",
                    path.display()
                )
            })?;
            let mut committed = Vec::with_capacity(imported.len() * 2);
            for import in imported {
                let result = writer.append(import).map_err(|e| {
                    format!(
                        "failed to append cassette event for {}: {e}",
                        path.display()
                    )
                })?;
                committed.extend(result.detection_events().iter().cloned());
                committed.push(result.committed_event().clone());
            }
            drop(writer);
            let _ = fs::remove_file(&eventlog_path);
            Ok(committed)
        }
    }
}

fn compare_replay_suggestions(
    left: &Path,
    right: &Path,
    left_format: CompareInputFormat,
    right_format: CompareInputFormat,
) -> Vec<String> {
    let left_view = match left_format {
        CompareInputFormat::Eventlog => format!("panopticon view {}", left.display()),
        CompareInputFormat::Cassette => format!(
            "panopticon tour {} --stress --output-dir left-tour-output",
            left.display()
        ),
    };
    let right_view = match right_format {
        CompareInputFormat::Eventlog => format!("panopticon view {}", right.display()),
        CompareInputFormat::Cassette => format!(
            "panopticon tour {} --stress --output-dir right-tour-output",
            right.display()
        ),
    };
    vec![left_view, right_view]
}

fn format_name(format: CompareInputFormat) -> &'static str {
    match format {
        CompareInputFormat::Eventlog => "eventlog",
        CompareInputFormat::Cassette => "cassette",
    }
}

fn share_safe_input_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "input".to_string())
}

fn write_committed_eventlog(path: &Path, events: &[CommittedEvent]) -> Result<(), String> {
    let mut lines = String::new();
    for event in events {
        let line = serde_json::to_string(event).map_err(|e| {
            format!(
                "failed to serialize committed event for {}: {e}",
                path.display()
            )
        })?;
        lines.push_str(&line);
        lines.push('\n');
    }
    fs::write(path, lines).map_err(|e| format!("failed to write {}: {e}", path.display()))
}

fn hash_file_blake3(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn write_json_pretty(path: &Path, value: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|e| format!("failed to serialize JSON for {}: {e}", path.display()))?;
    fs::write(path, bytes).map_err(|e| format!("failed to write {}: {e}", path.display()))
}

fn replay_summary(events: &[CommittedEvent]) -> Value {
    let (state, _checkpoints) = replay(events);
    let state_hash_hex = state_hash(&state);
    let invariants = ProjectionInvariants::default();
    let vm = project(&state, &invariants);
    let vm_hash_hex = viewmodel_hash(&vm);
    let first_commit_index = events.first().map(|e| e.commit_index);
    let last_commit_index = events.last().map(|e| e.commit_index);
    json!({
        "event_count": events.len(),
        "first_commit_index": first_commit_index,
        "last_commit_index": last_commit_index,
        "state_hash": state_hash_hex,
        "viewmodel_hash": vm_hash_hex,
        "projection_invariants_version": vm.projection_invariants_version,
        "degradation_level": vm.degradation_level,
        "tier_a_drops": vm.tier_a_drops,
        "queue_pressure": vm.queue_pressure(),
    })
}

pub(crate) fn handle_command(cli: Cli, mode: OutputMode, repair_notes: &[String]) -> AppExit {
    let map_profile = |profile: UiProfileArg| match profile {
        UiProfileArg::Standard => UiProfile::Standard,
        UiProfileArg::Showcase => UiProfile::Showcase,
    };

    match cli.command {
        Commands::View { eventlog, profile } => {
            if let Err(msg) = ensure_file_exists(&eventlog, "eventlog file") {
                let suggestions = vec![
                    format!(
                        "Check that `{}` exists and is readable.",
                        eventlog.display()
                    ),
                    format!("panopticon view {}", eventlog.display()),
                ];
                if mode == OutputMode::Json {
                    emit_json_error(
                        "NOT_FOUND",
                        &msg,
                        &suggestions,
                        repair_notes,
                        AppExit::NotFound as u8,
                    );
                } else {
                    eprintln!(
                        "{}",
                        format_cli_failure(
                            &format!("view failed: {msg}"),
                            "Input path does not exist.",
                            &suggestions,
                            &[eventlog.display().to_string()],
                        )
                    );
                }
                return AppExit::NotFound;
            }
            if let Err(e) = run_viewer(&eventlog, map_profile(profile)) {
                let suggestions = vec![
                    format!("panopticon view {}", eventlog.display()),
                    "panopticon --help".to_string(),
                ];
                if mode == OutputMode::Json {
                    emit_json_error(
                        "RUNTIME_ERROR",
                        &format!("view failed: {e}"),
                        &suggestions,
                        repair_notes,
                        AppExit::RuntimeError as u8,
                    );
                } else {
                    let msg = format_cli_failure(
                        &format!("view failed: {e}"),
                        "EventLog path is invalid or input is not canonical EventLog JSONL.",
                        &suggestions,
                        &[eventlog.display().to_string()],
                    );
                    eprintln!("{msg}");
                }
                return AppExit::RuntimeError;
            }
        }
        Commands::Export {
            eventlog,
            output,
            share_safe,
            refusal_report,
        } => {
            if let Err(msg) = ensure_file_exists(&eventlog, "eventlog file") {
                let suggestions = vec![
                    format!(
                        "Check that `{}` exists and is readable.",
                        eventlog.display()
                    ),
                    format!(
                        "panopticon export {} --share-safe --output {}",
                        eventlog.display(),
                        output.display()
                    ),
                ];
                if mode == OutputMode::Json {
                    emit_json_error(
                        "NOT_FOUND",
                        &msg,
                        &suggestions,
                        repair_notes,
                        AppExit::NotFound as u8,
                    );
                } else {
                    eprintln!(
                        "{}",
                        format_cli_failure(
                            &format!("export failed: {msg}"),
                            "Input path does not exist.",
                            &suggestions,
                            &[eventlog.display().to_string()],
                        )
                    );
                }
                return AppExit::NotFound;
            }
            if !share_safe {
                let suggestions = vec![
                    format!(
                        "panopticon export {} --share-safe --output {}",
                        eventlog.display(),
                        output.display()
                    ),
                    format!(
                        "panopticon export {} --share-safe --output {} --refusal-report out/refusal-report.json",
                        eventlog.display(),
                        output.display()
                    ),
                ];
                if mode == OutputMode::Json {
                    emit_json_error(
                        "INVALID_ARGS",
                        "--share-safe flag is required in v0.1.",
                        &suggestions,
                        repair_notes,
                        AppExit::InvalidArgs as u8,
                    );
                } else {
                    let msg = format_cli_failure(
                        "--share-safe flag is required in v0.1.",
                        "Export without secret scanning is disabled for share-safe posture.",
                        &suggestions,
                        &[eventlog.display().to_string()],
                    );
                    eprintln!("{msg}");
                }
                return AppExit::InvalidArgs;
            }

            let mut config = ExportConfig::new(&eventlog, &output);
            config.share_safe = share_safe;
            if let Some(report_path) = refusal_report {
                config = config.with_refusal_report(report_path);
            }

            match panopticon_export::run_export(&config) {
                Ok(ExportResult::Success(success)) => {
                    if mode == OutputMode::Json {
                        emit_json_success(
                            "OK",
                            "Export completed successfully.",
                            Some("export"),
                            AppExit::Success as u8,
                            repair_notes,
                            json!({
                                "bundle_path": success.bundle_path,
                                "bundle_hash": success.bundle_hash,
                                "event_count": success.event_count,
                                "blob_count": success.blob_count,
                            }),
                        );
                    } else {
                        println!("Export successful!");
                        println!("  Bundle: {}", success.bundle_path.display());
                        println!("  Hash:   {}", success.bundle_hash);
                        println!("  Events: {}", success.event_count);
                        println!("  Blobs:  {}", success.blob_count);
                    }
                }
                Ok(ExportResult::Refused(report)) => {
                    let mut evidence = vec![eventlog.display().to_string()];
                    if let Some(ref report_path) = config.refusal_report_path {
                        evidence.push(report_path.display().to_string());
                    }
                    let suggestions = vec![
                        "Inspect refusal-report.json for exact blocked fields.".to_string(),
                        format!(
                            "panopticon export {} --share-safe --output {} --refusal-report out/refusal-report.json",
                            eventlog.display(),
                            output.display()
                        ),
                    ];
                    if mode == OutputMode::Json {
                        let mut resp = json!({
                            "schema_version": ROBOT_SCHEMA_VERSION,
                            "ok": false,
                            "code": "EXPORT_REFUSED",
                            "message": report.summary,
                            "suggestions": suggestions,
                            "blocked_items": report.blocked_items,
                            "evidence": evidence,
                            "exit_code": AppExit::ExportRefused as u8,
                        });
                        if !repair_notes.is_empty() {
                            resp["notes"] = json!(repair_notes);
                        }
                        emit_json(resp);
                    } else {
                        eprintln!(
                            "{}",
                            format_cli_failure(
                                &format!("export refused: {}", report.summary),
                                "Secret scanner found sensitive content and blocked bundle creation.",
                                &suggestions,
                                &evidence,
                            )
                        );
                        for item in &report.blocked_items {
                            let loc = item
                                .blob_ref
                                .as_deref()
                                .map(|b| format!("blob:{}", b))
                                .unwrap_or_else(|| format!("event:{}", item.event_id));
                            eprintln!(
                                "  - {} @ {}: {} ({})",
                                loc, item.field_path, item.matched_pattern, item.redacted_match
                            );
                        }
                    }
                    return AppExit::ExportRefused;
                }
                Err(e) => {
                    let suggestions = vec![
                        format!(
                            "panopticon export {} --share-safe --output {} --refusal-report out/refusal-report.json",
                            eventlog.display(),
                            output.display()
                        ),
                        "panopticon --help".to_string(),
                    ];
                    if mode == OutputMode::Json {
                        emit_json_error(
                            "RUNTIME_ERROR",
                            &format!("export failed: {e}"),
                            &suggestions,
                            repair_notes,
                            AppExit::RuntimeError as u8,
                        );
                    } else {
                        eprintln!(
                            "{}",
                            format_cli_failure(
                                &format!("export failed: {e}"),
                                "File path, permissions, or bundle write step failed.",
                                &suggestions,
                                &[eventlog.display().to_string(), output.display().to_string()],
                            )
                        );
                    }
                    return AppExit::RuntimeError;
                }
            }
        }
        Commands::Tour {
            fixture,
            stress,
            output_dir,
        } => {
            if let Err(msg) = ensure_file_exists(&fixture, "fixture file") {
                let suggestions = vec![
                    format!("Check that `{}` exists and is readable.", fixture.display()),
                    format!(
                        "panopticon tour {} --stress --output-dir {}",
                        fixture.display(),
                        output_dir.display()
                    ),
                ];
                if mode == OutputMode::Json {
                    emit_json_error(
                        "NOT_FOUND",
                        &msg,
                        &suggestions,
                        repair_notes,
                        AppExit::NotFound as u8,
                    );
                } else {
                    eprintln!(
                        "{}",
                        format_cli_failure(
                            &format!("tour failed: {msg}"),
                            "Fixture path does not exist.",
                            &suggestions,
                            &[fixture.display().to_string()],
                        )
                    );
                }
                return AppExit::NotFound;
            }
            if !stress {
                let suggestions = vec![format!(
                    "panopticon tour {} --stress --output-dir {}",
                    fixture.display(),
                    output_dir.display()
                )];
                if mode == OutputMode::Json {
                    emit_json_error(
                        "INVALID_ARGS",
                        "--stress flag is required in v0.1.",
                        &suggestions,
                        repair_notes,
                        AppExit::InvalidArgs as u8,
                    );
                } else {
                    let msg = format_cli_failure(
                        "--stress flag is required in v0.1.",
                        "Tour is a stress harness and must run with explicit stress intent.",
                        &suggestions,
                        &[fixture.display().to_string()],
                    );
                    eprintln!("{msg}");
                }
                return AppExit::InvalidArgs;
            }

            let config = TourConfig::new(&fixture).with_output_dir(&output_dir);

            match panopticon_tour::run_tour(&config) {
                Ok(result) => {
                    if mode == OutputMode::Json {
                        emit_json_success(
                            "OK",
                            "Tour completed successfully.",
                            Some("tour"),
                            AppExit::Success as u8,
                            repair_notes,
                            json!({
                                "output_dir": result.output_dir,
                                "event_count": result.metrics.event_count_total,
                                "tier_a_drops": result.metrics.tier_a_drops,
                                "degradation_level": result.metrics.degradation_level_final,
                                "viewmodel_hash": result.viewmodel_hash,
                                "artifacts": [
                                    "metrics.json",
                                    "viewmodel.hash",
                                    "ansi.capture",
                                    "timetravel.capture"
                                ],
                            }),
                        );
                    } else {
                        println!("Tour completed successfully!");
                        println!("  Output:   {}", result.output_dir.display());
                        println!("  Events:   {}", result.metrics.event_count_total);
                        println!("  Drops:    {}", result.metrics.tier_a_drops);
                        println!("  Level:    {}", result.metrics.degradation_level_final);
                        println!("  Hash:     {}", result.viewmodel_hash);
                        println!();
                        println!("Artifacts:");
                        println!("  - metrics.json");
                        println!("  - viewmodel.hash");
                        println!("  - ansi.capture");
                        println!("  - timetravel.capture");
                    }
                }
                Err(e) => {
                    let suggestions = vec![format!(
                        "panopticon tour {} --stress --output-dir {}",
                        fixture.display(),
                        output_dir.display()
                    )];
                    if mode == OutputMode::Json {
                        emit_json_error(
                            "RUNTIME_ERROR",
                            &format!("tour failed: {e}"),
                            &suggestions,
                            repair_notes,
                            AppExit::RuntimeError as u8,
                        );
                    } else {
                        eprintln!(
                            "{}",
                            format_cli_failure(
                                &format!("tour failed: {e}"),
                                "Fixture path is invalid or tour artifact generation failed.",
                                &suggestions,
                                &[
                                    fixture.display().to_string(),
                                    output_dir.display().to_string()
                                ],
                            )
                        );
                    }
                    return AppExit::RuntimeError;
                }
            }
        }
        Commands::Compare {
            left,
            right,
            left_format,
            right_format,
        } => {
            if let Err(msg) = ensure_file_exists(&left, "left input file") {
                let suggestions =
                    compare_replay_suggestions(&left, &right, left_format, right_format);
                if mode == OutputMode::Json {
                    emit_json_error(
                        "NOT_FOUND",
                        &msg,
                        &suggestions,
                        repair_notes,
                        AppExit::NotFound as u8,
                    );
                } else {
                    eprintln!(
                        "{}",
                        format_cli_failure(
                            &format!("compare failed: {msg}"),
                            "Left input path does not exist.",
                            &suggestions,
                            &[left.display().to_string(), right.display().to_string()],
                        )
                    );
                }
                return AppExit::NotFound;
            }
            if let Err(msg) = ensure_file_exists(&right, "right input file") {
                let suggestions =
                    compare_replay_suggestions(&left, &right, left_format, right_format);
                if mode == OutputMode::Json {
                    emit_json_error(
                        "NOT_FOUND",
                        &msg,
                        &suggestions,
                        repair_notes,
                        AppExit::NotFound as u8,
                    );
                } else {
                    eprintln!(
                        "{}",
                        format_cli_failure(
                            &format!("compare failed: {msg}"),
                            "Right input path does not exist.",
                            &suggestions,
                            &[left.display().to_string(), right.display().to_string()],
                        )
                    );
                }
                return AppExit::NotFound;
            }

            let left_events = match load_committed_events(&left, left_format) {
                Ok(events) => events,
                Err(msg) => {
                    let suggestions =
                        compare_replay_suggestions(&left, &right, left_format, right_format);
                    if mode == OutputMode::Json {
                        emit_json_error(
                            "RUNTIME_ERROR",
                            &msg,
                            &suggestions,
                            repair_notes,
                            AppExit::RuntimeError as u8,
                        );
                    } else {
                        eprintln!(
                            "{}",
                            format_cli_failure(
                                &format!("compare failed: {msg}"),
                                "Failed to parse left input using the selected format.",
                                &suggestions,
                                &[left.display().to_string()],
                            )
                        );
                    }
                    return AppExit::RuntimeError;
                }
            };
            let right_events = match load_committed_events(&right, right_format) {
                Ok(events) => events,
                Err(msg) => {
                    let suggestions =
                        compare_replay_suggestions(&left, &right, left_format, right_format);
                    if mode == OutputMode::Json {
                        emit_json_error(
                            "RUNTIME_ERROR",
                            &msg,
                            &suggestions,
                            repair_notes,
                            AppExit::RuntimeError as u8,
                        );
                    } else {
                        eprintln!(
                            "{}",
                            format_cli_failure(
                                &format!("compare failed: {msg}"),
                                "Failed to parse right input using the selected format.",
                                &suggestions,
                                &[right.display().to_string()],
                            )
                        );
                    }
                    return AppExit::RuntimeError;
                }
            };

            let delta = diff_runs(&left_events, &right_events);
            let divergence_count = delta.divergences.len();
            let replay = compare_replay_suggestions(&left, &right, left_format, right_format);
            if divergence_count == 0 {
                if mode == OutputMode::Json {
                    emit_json_success(
                        "OK",
                        "No divergence detected.",
                        Some("compare"),
                        AppExit::Success as u8,
                        repair_notes,
                        json!({
                            "status": "NO_DIFF",
                            "left_path": left,
                            "right_path": right,
                            "left_format": format!("{left_format:?}").to_lowercase(),
                            "right_format": format!("{right_format:?}").to_lowercase(),
                            "delta": delta,
                            "replay_commands": replay,
                        }),
                    );
                } else {
                    println!("Compare completed: no divergence.");
                    println!("  Left:  {}", left.display());
                    println!("  Right: {}", right.display());
                    println!("Next command(s):");
                    for (idx, cmd) in replay.iter().enumerate() {
                        println!("  {}. {}", idx + 1, cmd);
                    }
                }
                return AppExit::Success;
            }

            if mode == OutputMode::Json {
                let mut response = json!({
                    "schema_version": ROBOT_SCHEMA_VERSION,
                    "ok": false,
                    "code": "DIFF_FOUND",
                    "message": format!("Detected {} divergence(s).", divergence_count),
                    "suggestions": replay,
                    "exit_code": AppExit::DiffFound as u8,
                    "command": "compare",
                    "data": {
                        "status": "DIFF_FOUND",
                        "left_path": left,
                        "right_path": right,
                        "left_format": format!("{left_format:?}").to_lowercase(),
                        "right_format": format!("{right_format:?}").to_lowercase(),
                        "divergence_count": divergence_count,
                        "delta": delta,
                    }
                });
                if !repair_notes.is_empty() {
                    response["notes"] = json!(repair_notes);
                }
                emit_json(response);
            } else {
                println!("Compare completed: divergence detected.");
                println!("  Left:        {}", left.display());
                println!("  Right:       {}", right.display());
                println!("  Divergences: {}", divergence_count);
                println!("Top divergences:");
                for divergence in delta.divergences.iter().take(10) {
                    println!(
                        "  - commit={} path={} class={:?}",
                        divergence.commit_index, divergence.path, divergence.change_class
                    );
                }
                println!("Next command(s):");
                for (idx, cmd) in replay.iter().enumerate() {
                    println!("  {}. {}", idx + 1, cmd);
                }
            }
            return AppExit::DiffFound;
        }
        Commands::IncidentPack {
            left,
            right,
            left_format,
            right_format,
            output_dir,
        } => {
            if let Err(msg) = ensure_file_exists(&left, "left input file") {
                let suggestions =
                    compare_replay_suggestions(&left, &right, left_format, right_format);
                if mode == OutputMode::Json {
                    emit_json_error(
                        "NOT_FOUND",
                        &msg,
                        &suggestions,
                        repair_notes,
                        AppExit::NotFound as u8,
                    );
                } else {
                    eprintln!(
                        "{}",
                        format_cli_failure(
                            &format!("incident-pack failed: {msg}"),
                            "Left input path does not exist.",
                            &suggestions,
                            &[left.display().to_string(), right.display().to_string()],
                        )
                    );
                }
                return AppExit::NotFound;
            }
            if let Err(msg) = ensure_file_exists(&right, "right input file") {
                let suggestions =
                    compare_replay_suggestions(&left, &right, left_format, right_format);
                if mode == OutputMode::Json {
                    emit_json_error(
                        "NOT_FOUND",
                        &msg,
                        &suggestions,
                        repair_notes,
                        AppExit::NotFound as u8,
                    );
                } else {
                    eprintln!(
                        "{}",
                        format_cli_failure(
                            &format!("incident-pack failed: {msg}"),
                            "Right input path does not exist.",
                            &suggestions,
                            &[left.display().to_string(), right.display().to_string()],
                        )
                    );
                }
                return AppExit::NotFound;
            }

            let left_events = match load_committed_events(&left, left_format) {
                Ok(events) => events,
                Err(msg) => {
                    let suggestions =
                        compare_replay_suggestions(&left, &right, left_format, right_format);
                    if mode == OutputMode::Json {
                        emit_json_error(
                            "RUNTIME_ERROR",
                            &msg,
                            &suggestions,
                            repair_notes,
                            AppExit::RuntimeError as u8,
                        );
                    } else {
                        eprintln!(
                            "{}",
                            format_cli_failure(
                                &format!("incident-pack failed: {msg}"),
                                "Failed to parse left input using the selected format.",
                                &suggestions,
                                &[left.display().to_string()],
                            )
                        );
                    }
                    return AppExit::RuntimeError;
                }
            };

            let right_events = match load_committed_events(&right, right_format) {
                Ok(events) => events,
                Err(msg) => {
                    let suggestions =
                        compare_replay_suggestions(&left, &right, left_format, right_format);
                    if mode == OutputMode::Json {
                        emit_json_error(
                            "RUNTIME_ERROR",
                            &msg,
                            &suggestions,
                            repair_notes,
                            AppExit::RuntimeError as u8,
                        );
                    } else {
                        eprintln!(
                            "{}",
                            format_cli_failure(
                                &format!("incident-pack failed: {msg}"),
                                "Failed to parse right input using the selected format.",
                                &suggestions,
                                &[right.display().to_string()],
                            )
                        );
                    }
                    return AppExit::RuntimeError;
                }
            };

            let normalized_dir = output_dir.join("normalized");
            let replay_dir = output_dir.join("replay");
            let compare_dir = output_dir.join("compare");
            let export_dir = output_dir.join("export");
            if let Err(e) = fs::create_dir_all(&normalized_dir)
                .and_then(|_| fs::create_dir_all(&replay_dir))
                .and_then(|_| fs::create_dir_all(&compare_dir))
                .and_then(|_| fs::create_dir_all(&export_dir))
            {
                let suggestions = vec![format!(
                    "panopticon incident-pack {} {} --output-dir {}",
                    left.display(),
                    right.display(),
                    output_dir.display()
                )];
                if mode == OutputMode::Json {
                    emit_json_error(
                        "RUNTIME_ERROR",
                        &format!("failed to create output directories: {e}"),
                        &suggestions,
                        repair_notes,
                        AppExit::RuntimeError as u8,
                    );
                } else {
                    eprintln!(
                        "{}",
                        format_cli_failure(
                            &format!("incident-pack failed: {e}"),
                            "Output directory is not writable.",
                            &suggestions,
                            &[output_dir.display().to_string()],
                        )
                    );
                }
                return AppExit::RuntimeError;
            }

            let left_eventlog_path = normalized_dir.join("left.eventlog.jsonl");
            let right_eventlog_path = normalized_dir.join("right.eventlog.jsonl");
            if let Err(msg) = write_committed_eventlog(&left_eventlog_path, &left_events) {
                let suggestions = vec![format!(
                    "Check write permissions for {}",
                    normalized_dir.display()
                )];
                if mode == OutputMode::Json {
                    emit_json_error(
                        "RUNTIME_ERROR",
                        &msg,
                        &suggestions,
                        repair_notes,
                        AppExit::RuntimeError as u8,
                    );
                } else {
                    eprintln!(
                        "{}",
                        format_cli_failure(
                            &format!("incident-pack failed: {msg}"),
                            "Unable to write normalized left eventlog.",
                            &suggestions,
                            &[left_eventlog_path.display().to_string()],
                        )
                    );
                }
                return AppExit::RuntimeError;
            }
            if let Err(msg) = write_committed_eventlog(&right_eventlog_path, &right_events) {
                let suggestions = vec![format!(
                    "Check write permissions for {}",
                    normalized_dir.display()
                )];
                if mode == OutputMode::Json {
                    emit_json_error(
                        "RUNTIME_ERROR",
                        &msg,
                        &suggestions,
                        repair_notes,
                        AppExit::RuntimeError as u8,
                    );
                } else {
                    eprintln!(
                        "{}",
                        format_cli_failure(
                            &format!("incident-pack failed: {msg}"),
                            "Unable to write normalized right eventlog.",
                            &suggestions,
                            &[right_eventlog_path.display().to_string()],
                        )
                    );
                }
                return AppExit::RuntimeError;
            }

            let delta = diff_runs(&left_events, &right_events);
            let divergence_count = delta.divergences.len();
            let delta_path = compare_dir.join("delta.json");
            if let Err(e) = write_json_pretty(&delta_path, &json!(delta)) {
                let suggestions = vec![format!(
                    "Check write permissions for {}",
                    compare_dir.display()
                )];
                if mode == OutputMode::Json {
                    emit_json_error(
                        "RUNTIME_ERROR",
                        &e,
                        &suggestions,
                        repair_notes,
                        AppExit::RuntimeError as u8,
                    );
                } else {
                    eprintln!(
                        "{}",
                        format_cli_failure(
                            &format!("incident-pack failed: {e}"),
                            "Unable to persist compare delta artifact.",
                            &suggestions,
                            &[delta_path.display().to_string()],
                        )
                    );
                }
                return AppExit::RuntimeError;
            }

            let left_replay_path = replay_dir.join("left.replay.json");
            let right_replay_path = replay_dir.join("right.replay.json");
            let left_replay = replay_summary(&left_events);
            let right_replay = replay_summary(&right_events);
            if let Err(e) = write_json_pretty(&left_replay_path, &left_replay) {
                if mode == OutputMode::Json {
                    emit_json_error(
                        "RUNTIME_ERROR",
                        &e,
                        &[],
                        repair_notes,
                        AppExit::RuntimeError as u8,
                    );
                } else {
                    eprintln!("incident-pack failed: {e}");
                }
                return AppExit::RuntimeError;
            }
            if let Err(e) = write_json_pretty(&right_replay_path, &right_replay) {
                if mode == OutputMode::Json {
                    emit_json_error(
                        "RUNTIME_ERROR",
                        &e,
                        &[],
                        repair_notes,
                        AppExit::RuntimeError as u8,
                    );
                } else {
                    eprintln!("incident-pack failed: {e}");
                }
                return AppExit::RuntimeError;
            }

            let left_bundle_path = export_dir.join("left.bundle.tar.zst");
            let right_bundle_path = export_dir.join("right.bundle.tar.zst");
            let left_refusal_path = export_dir.join("left.refusal-report.json");
            let right_refusal_path = export_dir.join("right.refusal-report.json");

            let left_export_cfg = ExportConfig::new(&left_eventlog_path, &left_bundle_path)
                .with_refusal_report(&left_refusal_path);
            let right_export_cfg = ExportConfig::new(&right_eventlog_path, &right_bundle_path)
                .with_refusal_report(&right_refusal_path);

            let left_export = panopticon_export::run_export(&left_export_cfg);
            let right_export = panopticon_export::run_export(&right_export_cfg);

            let (left_bundle_hash, right_bundle_hash) = match (left_export, right_export) {
                (Ok(ExportResult::Success(left_ok)), Ok(ExportResult::Success(right_ok))) => {
                    (left_ok.bundle_hash, right_ok.bundle_hash)
                }
                (Ok(ExportResult::Refused(left_refused)), _) => {
                    let suggestions = vec![
                        "Inspect left.refusal-report.json for exact blocked fields.".to_string(),
                        format!(
                            "panopticon export {} --share-safe --output out.tar.zst --refusal-report out/refusal-report.json",
                            left_eventlog_path.display()
                        ),
                    ];
                    if mode == OutputMode::Json {
                        emit_json_error(
                            "EXPORT_REFUSED",
                            &left_refused.summary,
                            &suggestions,
                            repair_notes,
                            AppExit::ExportRefused as u8,
                        );
                    } else {
                        eprintln!(
                            "{}",
                            format_cli_failure(
                                &format!("incident-pack export refused: {}", left_refused.summary),
                                "Secret scanner found sensitive content in left input.",
                                &suggestions,
                                &[left_refusal_path.display().to_string()],
                            )
                        );
                    }
                    return AppExit::ExportRefused;
                }
                (_, Ok(ExportResult::Refused(right_refused))) => {
                    let suggestions = vec![
                        "Inspect right.refusal-report.json for exact blocked fields.".to_string(),
                        format!(
                            "panopticon export {} --share-safe --output out.tar.zst --refusal-report out/refusal-report.json",
                            right_eventlog_path.display()
                        ),
                    ];
                    if mode == OutputMode::Json {
                        emit_json_error(
                            "EXPORT_REFUSED",
                            &right_refused.summary,
                            &suggestions,
                            repair_notes,
                            AppExit::ExportRefused as u8,
                        );
                    } else {
                        eprintln!(
                            "{}",
                            format_cli_failure(
                                &format!("incident-pack export refused: {}", right_refused.summary),
                                "Secret scanner found sensitive content in right input.",
                                &suggestions,
                                &[right_refusal_path.display().to_string()],
                            )
                        );
                    }
                    return AppExit::ExportRefused;
                }
                (Err(e), _) | (_, Err(e)) => {
                    let suggestions = vec![format!(
                        "panopticon export {} --share-safe --output out.tar.zst --refusal-report out/refusal-report.json",
                        left_eventlog_path.display()
                    )];
                    if mode == OutputMode::Json {
                        emit_json_error(
                            "RUNTIME_ERROR",
                            &format!("incident-pack export failed: {e}"),
                            &suggestions,
                            repair_notes,
                            AppExit::RuntimeError as u8,
                        );
                    } else {
                        eprintln!(
                            "{}",
                            format_cli_failure(
                                &format!("incident-pack export failed: {e}"),
                                "Share-safe export stage failed while building evidence pack.",
                                &suggestions,
                                &[output_dir.display().to_string()],
                            )
                        );
                    }
                    return AppExit::RuntimeError;
                }
            };

            let mut files = BTreeMap::new();
            let tracked = [
                (
                    "normalized/left.eventlog.jsonl",
                    left_eventlog_path.as_path(),
                ),
                (
                    "normalized/right.eventlog.jsonl",
                    right_eventlog_path.as_path(),
                ),
                ("compare/delta.json", delta_path.as_path()),
                ("replay/left.replay.json", left_replay_path.as_path()),
                ("replay/right.replay.json", right_replay_path.as_path()),
                ("export/left.bundle.tar.zst", left_bundle_path.as_path()),
                ("export/right.bundle.tar.zst", right_bundle_path.as_path()),
            ];
            for (name, path) in tracked {
                match hash_file_blake3(path) {
                    Ok(hash) => {
                        files.insert(name.to_string(), hash);
                    }
                    Err(msg) => {
                        if mode == OutputMode::Json {
                            emit_json_error(
                                "RUNTIME_ERROR",
                                &msg,
                                &[],
                                repair_notes,
                                AppExit::RuntimeError as u8,
                            );
                        } else {
                            eprintln!("incident-pack failed: {msg}");
                        }
                        return AppExit::RuntimeError;
                    }
                }
            }

            let manifest_path = output_dir.join("manifest.json");
            let manifest = json!({
                "schema_version": "panopticon-incident-pack-v1",
                "command": "incident-pack",
                "left_input_path": share_safe_input_label(&left),
                "right_input_path": share_safe_input_label(&right),
                "left_format": format_name(left_format),
                "right_format": format_name(right_format),
                "divergence_count": divergence_count,
                "left_bundle_hash": left_bundle_hash,
                "right_bundle_hash": right_bundle_hash,
                "files": files,
            });
            match serde_json::to_vec_pretty(&manifest) {
                Ok(bytes) => {
                    if let Err(e) = fs::write(&manifest_path, bytes) {
                        if mode == OutputMode::Json {
                            emit_json_error(
                                "RUNTIME_ERROR",
                                &format!("failed to write manifest: {e}"),
                                &[],
                                repair_notes,
                                AppExit::RuntimeError as u8,
                            );
                        } else {
                            eprintln!("incident-pack failed: {e}");
                        }
                        return AppExit::RuntimeError;
                    }
                }
                Err(e) => {
                    if mode == OutputMode::Json {
                        emit_json_error(
                            "RUNTIME_ERROR",
                            &format!("failed to serialize manifest: {e}"),
                            &[],
                            repair_notes,
                            AppExit::RuntimeError as u8,
                        );
                    } else {
                        eprintln!("incident-pack failed: {e}");
                    }
                    return AppExit::RuntimeError;
                }
            }

            if mode == OutputMode::Json {
                emit_json_success(
                    "OK",
                    "Incident evidence pack generated.",
                    Some("incident-pack"),
                    AppExit::Success as u8,
                    repair_notes,
                    json!({
                        "output_dir": output_dir,
                        "manifest_path": manifest_path,
                        "divergence_count": divergence_count,
                        "left_bundle_hash": left_bundle_hash,
                        "right_bundle_hash": right_bundle_hash,
                    }),
                );
            } else {
                println!("Incident pack generated.");
                println!("  Output dir:      {}", output_dir.display());
                println!("  Manifest:        {}", manifest_path.display());
                println!("  Divergences:     {}", divergence_count);
                println!("  Left bundle:     {}", left_bundle_path.display());
                println!("  Right bundle:    {}", right_bundle_path.display());
            }
            return AppExit::Success;
        }
    }

    AppExit::Success
}
