//! Integration tests for the export pipeline (M8.6).
//!
//! Required tests per bead bd-d7c.6:
//! 1. Clean fixture export → verify bundle_hash stability (run twice, compare)
//! 2. Secret-seeded fixture → export refused → verify refusal-report.json
//! 3. Re-export same clean fixture → bundle_hash matches first export
//! 4. Archive contents: unpack, verify EventLog and blobs intact
//! 5. Integrity manifest: unpack, verify manifest hashes match actual files
//! 6. Deterministic ordering: archive entries in expected order

use panopticon_core::blob_store::BlobStore;
use panopticon_core::event::{EventPayload, ImportEvent, Tier};
use panopticon_core::eventlog::EventLogWriter;
use panopticon_export::{BundleManifest, ExportConfig, ExportResult, ExportSuccess, RefusalReport};
use std::collections::HashMap;
use tempfile::tempdir;

/// Create a clean event (no secrets).
fn clean_event(id: &str, ts: u64, args: &str) -> ImportEvent {
    ImportEvent {
        run_id: "test-run".into(),
        event_id: id.into(),
        source_id: "test".into(),
        source_seq: Some(0),
        timestamp_ns: ts,
        tier: Tier::A,
        payload: EventPayload::ToolCall {
            tool: "test_tool".into(),
            args: Some(args.into()),
        },
        payload_ref: None,
        synthesized: false,
    }
}

/// Create an event with a known secret pattern.
fn secret_event(id: &str, ts: u64, secret: &str) -> ImportEvent {
    ImportEvent {
        run_id: "test-run".into(),
        event_id: id.into(),
        source_id: "test".into(),
        source_seq: Some(0),
        timestamp_ns: ts,
        tier: Tier::A,
        payload: EventPayload::ToolCall {
            tool: "leaked".into(),
            args: Some(secret.into()),
        },
        payload_ref: None,
        synthesized: false,
    }
}

/// Write a clean EventLog fixture with multiple events.
fn write_clean_fixture(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("eventlog.jsonl");
    let mut writer = EventLogWriter::open(&path).unwrap();
    writer
        .append(clean_event("e1", 1_000_000_000, "hello world"))
        .unwrap();
    writer
        .append(clean_event("e2", 2_000_000_000, "testing export"))
        .unwrap();
    writer
        .append(clean_event("e3", 3_000_000_000, "determinism check"))
        .unwrap();
    drop(writer);
    path
}

/// Write a clean EventLog with blob references.
fn write_clean_fixture_with_blobs(dir: &std::path::Path) -> (std::path::PathBuf, BlobStore) {
    let blobs_dir = dir.join("blobs");
    let store = BlobStore::open(&blobs_dir).unwrap();

    let blob_ref1 = store.write_blob(b"blob content alpha").unwrap();
    let blob_ref2 = store.write_blob(b"blob content beta").unwrap();

    let path = dir.join("eventlog.jsonl");
    let mut writer = EventLogWriter::open(&path).unwrap();

    let mut ev1 = clean_event("e1", 1_000_000_000, "with blob");
    ev1.payload_ref = Some(blob_ref1);
    writer.append(ev1).unwrap();

    let mut ev2 = clean_event("e2", 2_000_000_000, "with blob 2");
    ev2.payload_ref = Some(blob_ref2);
    writer.append(ev2).unwrap();

    writer
        .append(clean_event("e3", 3_000_000_000, "inline only"))
        .unwrap();

    drop(writer);
    (path, store)
}

/// Extract all entries from a .tar.zst bundle as (path, bytes).
fn extract_bundle(bundle_path: &std::path::Path) -> HashMap<String, Vec<u8>> {
    let compressed = std::fs::read(bundle_path).unwrap();
    let decompressed = zstd::decode_all(compressed.as_slice()).unwrap();
    let mut archive = tar::Archive::new(decompressed.as_slice());
    let mut entries = HashMap::new();

    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().to_string();
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut data).unwrap();
        entries.insert(path, data);
    }
    entries
}

/// Extract the manifest from a bundle.
fn extract_manifest(bundle_path: &std::path::Path) -> BundleManifest {
    let entries = extract_bundle(bundle_path);
    let manifest_bytes = entries.get("manifest.json").expect("manifest.json missing");
    serde_json::from_slice(manifest_bytes).expect("manifest.json parse failed")
}

/// Extract ordered entry paths from a bundle.
fn extract_entry_paths(bundle_path: &std::path::Path) -> Vec<String> {
    let compressed = std::fs::read(bundle_path).unwrap();
    let decompressed = zstd::decode_all(compressed.as_slice()).unwrap();
    let mut archive = tar::Archive::new(decompressed.as_slice());
    archive
        .entries()
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            e.path().unwrap().to_string_lossy().to_string()
        })
        .collect()
}

// ---- Test 1: Clean fixture export → bundle_hash stability ----

#[test]
fn clean_export_hash_stable_across_runs() {
    let dir = tempdir().unwrap();
    let eventlog_path = write_clean_fixture(dir.path());

    let bundle1 = dir.path().join("bundle1.tar.zst");
    let bundle2 = dir.path().join("bundle2.tar.zst");

    let config1 = ExportConfig::new(&eventlog_path, &bundle1);
    let config2 = ExportConfig::new(&eventlog_path, &bundle2);

    let result1 = run_export_success(&config1);
    let result2 = run_export_success(&config2);

    // Same inputs → same hash
    assert_eq!(result1.bundle_hash, result2.bundle_hash);

    // Same bytes
    let bytes1 = std::fs::read(&bundle1).unwrap();
    let bytes2 = std::fs::read(&bundle2).unwrap();
    assert_eq!(bytes1, bytes2, "Bundle bytes must be identical");
}

// ---- Test 2: Secret-seeded fixture → refused with report ----

#[test]
fn secret_seeded_export_refused_with_report() {
    let dir = tempdir().unwrap();
    let eventlog_path = dir.path().join("eventlog.jsonl");
    let report_path = dir.path().join("refusal-report.json");

    let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
    writer
        .append(clean_event("e1", 1_000_000_000, "clean content"))
        .unwrap();
    writer
        .append(secret_event(
            "e2",
            2_000_000_000,
            "my key is AKIAIOSFODNN7EXAMPLE",
        ))
        .unwrap();
    writer
        .append(secret_event("e3", 3_000_000_000, "password=hunter2secret"))
        .unwrap();
    drop(writer);

    let bundle_path = dir.path().join("bundle.tar.zst");
    let config = ExportConfig::new(&eventlog_path, &bundle_path).with_refusal_report(&report_path);

    let result = panopticon_export::run_export(&config).unwrap();

    match result {
        ExportResult::Refused(report) => {
            verify_refusal_report(&report);
        }
        ExportResult::Success(_) => panic!("expected refused, got success"),
    }

    // Verify report was written to disk
    assert!(report_path.exists());
    let disk_report: RefusalReport =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    verify_refusal_report(&disk_report);

    // Bundle should NOT exist
    assert!(!bundle_path.exists());
}

fn verify_refusal_report(report: &RefusalReport) {
    assert_eq!(report.report_version, "refusal-v0.1");
    assert!(!report.blocked_items.is_empty());

    // Verify specific blocked items
    assert!(
        report
            .blocked_items
            .iter()
            .any(|i| i.matched_pattern == "aws_access_key"),
        "Should detect AWS key"
    );
    assert!(
        report
            .blocked_items
            .iter()
            .any(|i| i.matched_pattern == "password"),
        "Should detect password"
    );

    // Verify event_id and field_path are populated
    for item in &report.blocked_items {
        assert!(!item.field_path.is_empty(), "field_path must be populated");
        assert!(
            !item.redacted_match.is_empty(),
            "redacted_match must be populated"
        );
    }

    // Verify deterministic ordering (sorted by event_id, field_path, pattern)
    for window in report.blocked_items.windows(2) {
        let ordering = window[0]
            .event_id
            .cmp(&window[1].event_id)
            .then_with(|| window[0].field_path.cmp(&window[1].field_path))
            .then_with(|| window[0].matched_pattern.cmp(&window[1].matched_pattern));
        assert!(ordering.is_le(), "blocked_items must be stably sorted");
    }
}

// ---- Test 3: Re-export same clean fixture → hash matches ----

#[test]
fn re_export_clean_fixture_hash_matches() {
    let dir = tempdir().unwrap();
    let eventlog_path = write_clean_fixture(dir.path());

    // First export
    let bundle1 = dir.path().join("export1.tar.zst");
    let config1 = ExportConfig::new(&eventlog_path, &bundle1);
    let result1 = run_export_success(&config1);

    // Delete first bundle to prove re-export is independent
    std::fs::remove_file(&bundle1).unwrap();

    // Re-export
    let bundle2 = dir.path().join("export2.tar.zst");
    let config2 = ExportConfig::new(&eventlog_path, &bundle2);
    let result2 = run_export_success(&config2);

    assert_eq!(
        result1.bundle_hash, result2.bundle_hash,
        "Re-export must produce same hash"
    );
}

// ---- Test 4: Archive contents — EventLog and blobs intact ----

#[test]
fn archive_contains_eventlog_and_blobs_intact() {
    let dir = tempdir().unwrap();
    let (eventlog_path, _store) = write_clean_fixture_with_blobs(dir.path());

    let bundle_path = dir.path().join("bundle.tar.zst");
    let config = ExportConfig::new(&eventlog_path, &bundle_path);
    let result = run_export_success(&config);

    assert_eq!(result.event_count, 3);
    assert_eq!(result.blob_count, 2);

    // Extract and verify contents
    let entries = extract_bundle(&bundle_path);

    // EventLog present and matches source
    let original_eventlog = std::fs::read(&eventlog_path).unwrap();
    let bundled_eventlog = entries.get("eventlog.jsonl").expect("eventlog missing");
    assert_eq!(
        &original_eventlog, bundled_eventlog,
        "EventLog bytes must match original"
    );

    // Blobs present (2 blobs)
    let blob_entries: Vec<_> = entries.keys().filter(|k| k.starts_with("blobs/")).collect();
    assert_eq!(blob_entries.len(), 2, "Should have 2 blob entries");

    // Manifest present
    assert!(entries.contains_key("manifest.json"));
}

// ---- Test 5: Integrity manifest — hashes match actual files ----

#[test]
fn manifest_hashes_match_actual_files() {
    let dir = tempdir().unwrap();
    let (eventlog_path, _store) = write_clean_fixture_with_blobs(dir.path());

    let bundle_path = dir.path().join("bundle.tar.zst");
    let config = ExportConfig::new(&eventlog_path, &bundle_path);
    run_export_success(&config);

    let entries = extract_bundle(&bundle_path);
    let manifest = extract_manifest(&bundle_path);

    assert_eq!(manifest.manifest_version, "manifest-v0.1");

    // Verify each manifest entry's hash matches actual content
    for file_entry in &manifest.files {
        let actual_data = entries.get(&file_entry.path).unwrap_or_else(|| {
            panic!(
                "File {} listed in manifest but not in archive",
                file_entry.path
            )
        });

        let actual_hash = blake3::hash(actual_data).to_hex().to_string();
        assert_eq!(
            file_entry.blake3, actual_hash,
            "BLAKE3 mismatch for {}",
            file_entry.path
        );
        assert_eq!(
            file_entry.size,
            actual_data.len() as u64,
            "Size mismatch for {}",
            file_entry.path
        );
    }

    // Verify commit_index_range
    let range = manifest
        .commit_index_range
        .expect("commit_index_range should be present");
    assert_eq!(range[0], 0, "first commit_index");
    assert_eq!(range[1], 2, "last commit_index (3 events: 0,1,2)");

    // Verify projection_invariants_version is populated
    assert!(
        !manifest.projection_invariants_version.is_empty(),
        "projection_invariants_version must be set"
    );
}

// ---- Test 6: Deterministic ordering ----

#[test]
fn archive_entries_in_deterministic_order() {
    let dir = tempdir().unwrap();
    let (eventlog_path, _store) = write_clean_fixture_with_blobs(dir.path());

    let bundle_path = dir.path().join("bundle.tar.zst");
    let config = ExportConfig::new(&eventlog_path, &bundle_path);
    run_export_success(&config);

    let paths = extract_entry_paths(&bundle_path);

    // Entries must be alphabetically sorted
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(
        paths, sorted,
        "Archive entries must be alphabetically sorted"
    );

    // Verify specific ordering: blobs/* < eventlog.jsonl < manifest.json
    let eventlog_idx = paths
        .iter()
        .position(|p| p == "eventlog.jsonl")
        .expect("eventlog.jsonl missing");
    let manifest_idx = paths
        .iter()
        .position(|p| p == "manifest.json")
        .expect("manifest.json missing");
    assert!(
        eventlog_idx < manifest_idx,
        "eventlog.jsonl should come before manifest.json"
    );

    // Any blob entries should come before eventlog.jsonl
    for (i, path) in paths.iter().enumerate() {
        if path.starts_with("blobs/") {
            assert!(
                i < eventlog_idx,
                "blobs/ entries should come before eventlog.jsonl"
            );
        }
    }
}

// ---- Additional edge case tests ----

#[test]
fn export_single_event_no_blobs() {
    let dir = tempdir().unwrap();
    let eventlog_path = dir.path().join("eventlog.jsonl");

    let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
    writer
        .append(clean_event("e1", 1_000_000_000, "single event"))
        .unwrap();
    drop(writer);

    let bundle_path = dir.path().join("bundle.tar.zst");
    let config = ExportConfig::new(&eventlog_path, &bundle_path);
    let result = run_export_success(&config);

    assert_eq!(result.event_count, 1);
    assert_eq!(result.blob_count, 0);

    let entries = extract_bundle(&bundle_path);
    assert_eq!(entries.len(), 2); // eventlog + manifest
    assert!(entries.contains_key("eventlog.jsonl"));
    assert!(entries.contains_key("manifest.json"));

    let manifest = extract_manifest(&bundle_path);
    let range = manifest.commit_index_range.unwrap();
    assert_eq!(range[0], 0);
    assert_eq!(range[1], 0); // single event: first == last
}

#[test]
fn bundle_hash_matches_file_bytes() {
    let dir = tempdir().unwrap();
    let eventlog_path = write_clean_fixture(dir.path());
    let bundle_path = dir.path().join("bundle.tar.zst");

    let config = ExportConfig::new(&eventlog_path, &bundle_path);
    let result = run_export_success(&config);

    // Independently hash file bytes
    let file_bytes = std::fs::read(&bundle_path).unwrap();
    let computed_hash = blake3::hash(&file_bytes).to_hex().to_string();
    assert_eq!(result.bundle_hash, computed_hash);
}

// ---- Empty EventLog export tests (bd-d7c.7) ----

/// Empty EventLog export must succeed (no secrets to find, clean by definition).
#[test]
fn export_empty_eventlog_succeeds() {
    let dir = tempdir().unwrap();
    let eventlog_path = dir.path().join("eventlog.jsonl");

    // Create an empty EventLog (open + close, zero events)
    let writer = EventLogWriter::open(&eventlog_path).unwrap();
    drop(writer);

    let bundle_path = dir.path().join("bundle.tar.zst");
    let config = ExportConfig::new(&eventlog_path, &bundle_path);
    let result = run_export_success(&config);

    assert_eq!(result.event_count, 0, "empty eventlog has 0 events");
    assert_eq!(result.blob_count, 0, "empty eventlog has 0 blobs");
    assert!(bundle_path.exists(), "bundle file must be created");
    assert_eq!(result.bundle_hash.len(), 64, "bundle_hash is BLAKE3 hex");
}

/// Empty EventLog manifest: commit_index_range must be absent (None).
#[test]
fn export_empty_eventlog_manifest_shape() {
    let dir = tempdir().unwrap();
    let eventlog_path = dir.path().join("eventlog.jsonl");
    let writer = EventLogWriter::open(&eventlog_path).unwrap();
    drop(writer);

    let bundle_path = dir.path().join("bundle.tar.zst");
    let config = ExportConfig::new(&eventlog_path, &bundle_path);
    run_export_success(&config);

    // Extract and verify manifest
    let manifest = extract_manifest(&bundle_path);
    assert_eq!(manifest.manifest_version, "manifest-v0.1");

    // commit_index_range must be None for empty EventLog
    assert!(
        manifest.commit_index_range.is_none(),
        "commit_index_range must be None for empty EventLog"
    );

    // Verify it's actually absent from JSON (skip_serializing_if)
    let entries = extract_bundle(&bundle_path);
    let manifest_json = String::from_utf8(entries.get("manifest.json").unwrap().clone()).unwrap();
    assert!(
        !manifest_json.contains("commit_index_range"),
        "commit_index_range must not appear in JSON for empty EventLog"
    );

    // projection_invariants_version must still be present
    assert!(
        !manifest.projection_invariants_version.is_empty(),
        "projection_invariants_version must be set even for empty EventLog"
    );

    // Files list: only eventlog.jsonl (no blobs)
    assert_eq!(manifest.files.len(), 1, "only eventlog.jsonl in manifest");
    assert_eq!(manifest.files[0].path, "eventlog.jsonl");
}

/// Empty EventLog bundle contents: eventlog.jsonl + manifest.json only.
#[test]
fn export_empty_eventlog_bundle_contents() {
    let dir = tempdir().unwrap();
    let eventlog_path = dir.path().join("eventlog.jsonl");
    let writer = EventLogWriter::open(&eventlog_path).unwrap();
    drop(writer);

    let bundle_path = dir.path().join("bundle.tar.zst");
    let config = ExportConfig::new(&eventlog_path, &bundle_path);
    run_export_success(&config);

    let entries = extract_bundle(&bundle_path);
    assert_eq!(entries.len(), 2, "bundle has eventlog + manifest");
    assert!(entries.contains_key("eventlog.jsonl"));
    assert!(entries.contains_key("manifest.json"));

    // eventlog.jsonl should be empty (0 bytes of event data)
    let eventlog_bytes = entries.get("eventlog.jsonl").unwrap();
    assert!(
        eventlog_bytes.is_empty(),
        "empty EventLog file should have 0 bytes"
    );

    // Entries in deterministic order
    let paths = extract_entry_paths(&bundle_path);
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted, "entries must be alphabetically sorted");
}

/// Empty EventLog export must be deterministic across reruns.
#[test]
fn export_empty_eventlog_deterministic() {
    let dir = tempdir().unwrap();
    let eventlog_path = dir.path().join("eventlog.jsonl");
    let writer = EventLogWriter::open(&eventlog_path).unwrap();
    drop(writer);

    let bundle1 = dir.path().join("bundle1.tar.zst");
    let bundle2 = dir.path().join("bundle2.tar.zst");

    let config1 = ExportConfig::new(&eventlog_path, &bundle1);
    let config2 = ExportConfig::new(&eventlog_path, &bundle2);

    let result1 = run_export_success(&config1);
    let result2 = run_export_success(&config2);

    // Same hash
    assert_eq!(
        result1.bundle_hash, result2.bundle_hash,
        "empty EventLog exports must produce identical hashes"
    );

    // Same bytes
    let bytes1 = std::fs::read(&bundle1).unwrap();
    let bytes2 = std::fs::read(&bundle2).unwrap();
    assert_eq!(
        bytes1, bytes2,
        "empty EventLog bundle bytes must be identical across reruns"
    );
}

/// Helper: run export and unwrap Success variant.
fn run_export_success(config: &ExportConfig) -> ExportSuccess {
    match panopticon_export::run_export(config).unwrap() {
        ExportResult::Success(s) => s,
        ExportResult::Refused(r) => panic!("expected success, got refused: {}", r.summary),
    }
}
