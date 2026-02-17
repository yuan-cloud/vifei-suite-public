use crate::cli_contract::{AppExit, Cli, Commands, OutputMode, ROBOT_SCHEMA_VERSION};
use crate::cli_normalize::format_cli_failure;
use panopticon_export::{ExportConfig, ExportResult};
use panopticon_tour::TourConfig;
use panopticon_tui::run_viewer;
use serde_json::{json, Value};
use std::path::Path;

fn emit_json(value: Value) {
    println!(
        "{}",
        serde_json::to_string(&value).expect("json output serialization must succeed")
    );
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

pub(crate) fn handle_command(cli: Cli, mode: OutputMode, repair_notes: &[String]) -> AppExit {
    match cli.command {
        Commands::View { eventlog } => {
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
            if let Err(e) = run_viewer(&eventlog) {
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
    }

    AppExit::Success
}
