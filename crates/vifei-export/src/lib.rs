//! Share-safe export pipeline for EventLogs.
//!
//! # Overview
//!
//! The export pipeline produces deterministic bundles from EventLogs after
//! verifying they contain no secrets. Export is the gate between internal
//! truth and external sharing — conservative by default, refusing if any
//! doubt about safety.
//!
//! # Pipeline stages
//!
//! ```text
//! EventLog → discover blobs → secret scan → bundle OR refuse
//! ```
//!
//! 1. **Discover**: Read EventLog, identify all referenced blobs
//! 2. **Scan**: Check event payloads and blob contents for secrets (M8.2)
//! 3. **Decide**: If secrets found → refuse with report; otherwise → bundle
//! 4. **Bundle**: Create deterministic tar.zstd archive (M8.4)
//!
//! # CLI
//!
//! ```text
//! vifei export --share-safe -o bundle.tar.zst ./eventlog.jsonl
//! ```
//!
//! # Invariants
//!
//! - **I3 (Share-safe export):** Never produce an unsafe bundle. Refuse is
//!   the default when any doubt exists.
//! - **I5 (Loud failure):** Errors are returned, never silently swallowed.

mod bundle;
mod discover;
mod scanner;
mod secret_scan;

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use vifei_core::blob_store::BlobStore;
use vifei_core::event::CommittedEvent;

pub(crate) use bundle::create_bundle;
pub(crate) use discover::discover_content;
pub(crate) use secret_scan::scan_for_secrets;

/// Scanner version string for refusal reports.
const SCANNER_VERSION: &str = "secret-scanner-v0.1";

/// Format current time as ISO 8601 UTC string.
///
/// Uses `SystemTime` to avoid adding chrono dependency.
/// This value is informational only (not included in any hash).
fn format_utc_now() -> String {
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    // Simple UTC formatting: seconds since epoch → ISO 8601
    // Good enough for informational timestamp (not used in deterministic surfaces)
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;

    // Days since 1970-01-01
    let (year, month, day) = days_to_ymd(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

/// Convert days since Unix epoch to (year, month, day).
fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    // Civil calendar algorithm
    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let days_in_months: [u64; 12] = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u64;
    for &dm in &days_in_months {
        if days < dm {
            break;
        }
        days -= dm;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(y: u64) -> bool {
    y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400))
}

/// Result of an export attempt.
#[derive(Debug)]
pub enum ExportResult {
    /// Export succeeded, bundle created at path.
    Success(ExportSuccess),
    /// Export refused due to detected secrets.
    Refused(RefusalReport),
}

/// Successful export result.
#[derive(Debug)]
pub struct ExportSuccess {
    /// Path to the created bundle.
    pub bundle_path: PathBuf,
    /// BLAKE3 hash of the bundle file.
    pub bundle_hash: String,
    /// Number of events in the bundle.
    pub event_count: usize,
    /// Number of blobs in the bundle.
    pub blob_count: usize,
}

/// Refusal report when export is blocked due to secrets.
///
/// Schema contract defined in PLANS.md § "Artifact schema contracts".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefusalReport {
    /// Report schema version (contract: "refusal-v0.1").
    pub report_version: String,
    /// Path to the source EventLog that was scanned.
    pub eventlog_path: String,
    /// Blocked items, stably sorted for deterministic output.
    pub blocked_items: Vec<BlockedItem>,
    /// ISO 8601 UTC timestamp of when the scan was performed (informational only).
    pub scan_timestamp_utc: String,
    /// Scanner version string.
    pub scanner_version: String,
    /// Human-readable summary (not in schema contract, kept for CLI display).
    pub summary: String,
}

impl RefusalReport {
    /// Create a new refusal report from blocked items.
    ///
    /// Items are stably sorted by
    /// (event_id, field_path, matched_pattern, blob_ref, redacted_match)
    /// for deterministic output per M8.3 requirements.
    pub fn new(eventlog_path: &str, mut items: Vec<BlockedItem>) -> Self {
        // Deterministic sort: include blob_ref and redacted_match as tie-breakers
        // to avoid nondeterministic ordering when multiple blob findings share
        // the same event_id/field_path/pattern tuple.
        items.sort_by(|a, b| {
            a.event_id
                .cmp(&b.event_id)
                .then_with(|| a.field_path.cmp(&b.field_path))
                .then_with(|| a.matched_pattern.cmp(&b.matched_pattern))
                .then_with(|| a.blob_ref.cmp(&b.blob_ref))
                .then_with(|| a.redacted_match.cmp(&b.redacted_match))
        });

        let unique_locations: HashSet<&str> = items
            .iter()
            .map(|f| f.blob_ref.as_deref().unwrap_or(f.event_id.as_str()))
            .collect();

        let summary = format!(
            "Export refused: {} secret(s) detected in {} location(s)",
            items.len(),
            unique_locations.len()
        );

        RefusalReport {
            report_version: "refusal-v0.1".into(),
            eventlog_path: eventlog_path.to_string(),
            blocked_items: items,
            scan_timestamp_utc: format_utc_now(),
            scanner_version: SCANNER_VERSION.into(),
            summary,
        }
    }

    /// Write the refusal report to a JSON file.
    pub fn write_to(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("JSON serialization failed: {e}"),
            )
        })?;
        std::fs::write(path, json)
    }
}

/// A single blocked item in a refusal report.
///
/// Schema contract: event_id, field_path, matched_pattern, blob_ref (optional).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedItem {
    /// Event ID where the secret was found.
    pub event_id: String,
    /// Field path within the event (dot-delimited, e.g., "payload.args").
    pub field_path: String,
    /// Pattern name that triggered the block (e.g., "aws_access_key").
    pub matched_pattern: String,
    /// Blob reference, if the secret was found in a blob rather than inline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_ref: Option<String>,
    /// Snippet of the matched content (redacted for safe display).
    pub redacted_match: String,
}

/// Export pipeline configuration.
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// Path to the EventLog JSONL file.
    pub eventlog_path: PathBuf,
    /// Output bundle path.
    pub output_path: PathBuf,
    /// Path to write refusal report if secrets found.
    pub refusal_report_path: Option<PathBuf>,
    /// Enable share-safe scanning (mandatory in v0.1).
    pub share_safe: bool,
}

impl ExportConfig {
    /// Create a new export configuration.
    pub fn new(eventlog_path: impl Into<PathBuf>, output_path: impl Into<PathBuf>) -> Self {
        ExportConfig {
            eventlog_path: eventlog_path.into(),
            output_path: output_path.into(),
            refusal_report_path: None,
            share_safe: true,
        }
    }

    /// Set the refusal report output path.
    pub fn with_refusal_report(mut self, path: impl Into<PathBuf>) -> Self {
        self.refusal_report_path = Some(path.into());
        self
    }
}

/// Discovered content from an EventLog ready for export.
#[derive(Debug)]
pub(crate) struct DiscoveredContent {
    /// Path to the EventLog file.
    pub eventlog_path: PathBuf,
    /// The events in the EventLog.
    pub events: Vec<CommittedEvent>,
    /// Set of blob payload_refs referenced by events.
    pub blob_refs: HashSet<String>,
}

impl DiscoveredContent {
    /// Total number of events.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

/// Integrity manifest embedded in export bundles (M8.5).
///
/// The manifest is the "receipt" for bundle consumers — tells them exactly
/// what's inside and how to verify individual file integrity.
///
/// Note: `bundle_hash` (BLAKE3 of the overall .tar.zst) is NOT in the manifest
/// because the manifest is inside the archive (circular dependency). The
/// `bundle_hash` is returned in [`ExportSuccess`] for external verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    /// Manifest schema version.
    pub manifest_version: String,
    /// Files in the bundle with BLAKE3 digests, stably sorted by path.
    pub files: Vec<ManifestEntry>,
    /// EventLog commit_index range: (first, last). None if EventLog is empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_index_range: Option<[u64; 2]>,
    /// Projection invariants version for context.
    pub projection_invariants_version: String,
}

/// A single file entry in the bundle manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// Archive path (e.g., "eventlog.jsonl", "blobs/abcd...").
    pub path: String,
    /// BLAKE3 hex digest of the file contents.
    pub blake3: String,
    /// File size in bytes.
    pub size: u64,
}

/// Run the full export pipeline.
///
/// This is the main entry point for the export CLI.
pub fn run_export(config: &ExportConfig) -> io::Result<ExportResult> {
    // Validate --share-safe is enabled (mandatory in v0.1)
    if !config.share_safe {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Export requires --share-safe flag in v0.1. \
             Unscanned exports are not supported.",
        ));
    }

    // Stage 1: Discover content
    let content = discover_content(&config.eventlog_path)?;

    // Try to open blob store (sibling to eventlog)
    let blob_store = config
        .eventlog_path
        .parent()
        .map(|p| p.join("blobs"))
        .and_then(|p| BlobStore::open(p).ok());

    // Stage 2: Scan for secrets
    let findings = scan_for_secrets(&content, blob_store.as_ref())?;

    // Stage 3: Decide
    if !findings.is_empty() {
        let eventlog_str = share_safe_path_label(&config.eventlog_path);
        let report = RefusalReport::new(&eventlog_str, findings);

        // Write refusal report if path configured
        if let Some(ref report_path) = config.refusal_report_path {
            report.write_to(report_path)?;
        }

        return Ok(ExportResult::Refused(report));
    }

    // Stage 4: Bundle (clean export)
    let success = create_bundle(&content, blob_store.as_ref(), &config.output_path)?;

    Ok(ExportResult::Success(success))
}

fn share_safe_path_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use vifei_core::event::{CommittedEvent, EventPayload, ImportEvent, Tier};
    use vifei_core::eventlog::EventLogWriter;
    use vifei_core::projection::PROJECTION_INVARIANTS_VERSION;

    fn make_event(id: &str, ts: u64, args: &str) -> ImportEvent {
        ImportEvent {
            run_id: "test-run".into(),
            event_id: id.into(),
            source_id: "test".into(),
            source_seq: Some(0),
            timestamp_ns: ts,
            tier: Tier::A,
            payload: EventPayload::ToolCall {
                tool: "test".into(),
                args: Some(args.into()),
            },
            payload_ref: None,
            synthesized: false,
        }
    }

    #[test]
    fn discover_content_basic() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");

        // Create a small EventLog
        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
        writer
            .append(make_event("e1", 1_000_000_000, "hello"))
            .unwrap();
        writer
            .append(make_event("e2", 2_000_000_000, "world"))
            .unwrap();
        drop(writer);

        let content = discover_content(&eventlog_path).unwrap();
        assert_eq!(content.event_count(), 2);
        assert!(content.blob_refs.is_empty());
    }

    #[test]
    fn discover_content_with_blobs() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");

        // Create EventLog with blob refs
        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();

        let mut event = make_event("e1", 1_000_000_000, "test");
        event.payload_ref = Some("abcd".repeat(16)); // 64 hex chars
        writer.append(event).unwrap();

        let mut event2 = make_event("e2", 2_000_000_000, "test");
        event2.payload_ref = Some("1234".repeat(16));
        writer.append(event2).unwrap();

        drop(writer);

        let content = discover_content(&eventlog_path).unwrap();
        assert_eq!(content.event_count(), 2);
        assert_eq!(content.blob_refs.len(), 2);
    }

    #[test]
    fn export_without_share_safe_fails() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");
        std::fs::write(&eventlog_path, "").unwrap();

        let mut config = ExportConfig::new(&eventlog_path, dir.path().join("out.tar.zst"));
        config.share_safe = false;

        let result = run_export(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("--share-safe"));
    }

    #[test]
    fn export_clean_eventlog_succeeds() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");

        // Create clean EventLog (no secrets)
        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
        writer
            .append(make_event("e1", 1_000_000_000, "hello world"))
            .unwrap();
        drop(writer);

        let output_path = dir.path().join("bundle.tar.zst");
        let config = ExportConfig::new(&eventlog_path, &output_path);

        let result = run_export(&config).unwrap();
        assert!(
            matches!(result, ExportResult::Success(_)),
            "expected success, got refusal"
        );
        if let ExportResult::Success(success) = result {
            assert_eq!(success.event_count, 1);
            assert_eq!(success.blob_count, 0);
            assert!(output_path.exists());
            assert_eq!(success.bundle_hash.len(), 64);
        }
    }

    #[test]
    fn export_with_aws_key_refused() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");

        // Create EventLog with an AWS key
        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
        writer
            .append(make_event(
                "e1",
                1_000_000_000,
                "my key is AKIAIOSFODNN7EXAMPLE",
            ))
            .unwrap();
        drop(writer);

        let output_path = dir.path().join("bundle.tar.zst");
        let config = ExportConfig::new(&eventlog_path, &output_path);

        let result = run_export(&config).unwrap();
        assert!(
            matches!(result, ExportResult::Refused(_)),
            "expected refusal, got success"
        );
        if let ExportResult::Refused(report) = result {
            assert!(!report.blocked_items.is_empty());
            assert!(report
                .blocked_items
                .iter()
                .any(|f| f.matched_pattern == "aws_access_key"));
        }
    }

    #[test]
    fn export_with_password_refused() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");

        // Create EventLog with a password
        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
        let key = ["pass", "word", "="].concat();
        let password_payload = format!("{key}{}", "supersecret123");
        writer
            .append(make_event("e1", 1_000_000_000, &password_payload))
            .unwrap();
        drop(writer);

        let output_path = dir.path().join("bundle.tar.zst");
        let config = ExportConfig::new(&eventlog_path, &output_path);

        let result = run_export(&config).unwrap();
        assert!(
            matches!(result, ExportResult::Refused(_)),
            "expected refusal, got success"
        );
        if let ExportResult::Refused(report) = result {
            assert!(!report.blocked_items.is_empty());
            assert!(report
                .blocked_items
                .iter()
                .any(|f| f.matched_pattern == "password"));
        }
    }

    #[test]
    fn export_with_jwt_refused() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");

        // Create EventLog with a JWT
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
        writer
            .append(make_event("e1", 1_000_000_000, &format!("token: {}", jwt)))
            .unwrap();
        drop(writer);

        let output_path = dir.path().join("bundle.tar.zst");
        let config = ExportConfig::new(&eventlog_path, &output_path);

        let result = run_export(&config).unwrap();
        assert!(
            matches!(result, ExportResult::Refused(_)),
            "expected refusal, got success"
        );
        if let ExportResult::Refused(report) = result {
            assert!(!report.blocked_items.is_empty());
            assert!(report
                .blocked_items
                .iter()
                .any(|f| f.matched_pattern == "jwt_token"));
        }
    }

    #[test]
    fn export_with_private_key_refused() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");

        // Create EventLog with a private key header
        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
        let private_key_header = ["-----BEGIN ", "RSA PRIVATE KEY", "-----"].concat();
        writer
            .append(make_event("e1", 1_000_000_000, &private_key_header))
            .unwrap();
        drop(writer);

        let output_path = dir.path().join("bundle.tar.zst");
        let config = ExportConfig::new(&eventlog_path, &output_path);

        let result = run_export(&config).unwrap();
        assert!(
            matches!(result, ExportResult::Refused(_)),
            "expected refusal, got success"
        );
        if let ExportResult::Refused(report) = result {
            assert!(!report.blocked_items.is_empty());
            assert!(report
                .blocked_items
                .iter()
                .any(|f| f.matched_pattern == "private_key"));
        }
    }

    #[test]
    fn refusal_report_schema_conformance() {
        let item = BlockedItem {
            event_id: "e-123".into(),
            field_path: "payload.args".into(),
            matched_pattern: "aws_access_key".into(),
            blob_ref: None,
            redacted_match: "AKIA***MPLE".into(),
        };
        let report = RefusalReport::new("/tmp/test.jsonl", vec![item]);

        let json = serde_json::to_string_pretty(&report).unwrap();

        // All required schema keys present (PLANS.md contract)
        assert!(json.contains("report_version"));
        assert!(json.contains("refusal-v0.1"));
        assert!(json.contains("eventlog_path"));
        assert!(json.contains("blocked_items"));
        assert!(json.contains("scan_timestamp_utc"));
        assert!(json.contains("scanner_version"));
        assert!(json.contains("event_id"));
        assert!(json.contains("field_path"));
        assert!(json.contains("matched_pattern"));
        assert!(json.contains("aws_access_key"));

        // Round-trip
        let parsed: RefusalReport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.blocked_items.len(), 1);
        assert_eq!(parsed.report_version, "refusal-v0.1");
        assert_eq!(parsed.eventlog_path, "/tmp/test.jsonl");
        assert!(!parsed.scan_timestamp_utc.is_empty());
        assert_eq!(parsed.scanner_version, "secret-scanner-v0.1");
    }

    #[test]
    fn export_config_builder() {
        let config =
            ExportConfig::new("event.jsonl", "out.tar.zst").with_refusal_report("refusal.json");

        assert_eq!(config.eventlog_path, PathBuf::from("event.jsonl"));
        assert_eq!(config.output_path, PathBuf::from("out.tar.zst"));
        assert_eq!(
            config.refusal_report_path,
            Some(PathBuf::from("refusal.json"))
        );
        assert!(config.share_safe);
    }

    #[test]
    fn refusal_report_written_to_file() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");
        let report_path = dir.path().join("refusal-report.json");

        // Create EventLog with a secret
        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
        writer
            .append(make_event("e1", 1_000_000_000, "AKIAIOSFODNN7EXAMPLE"))
            .unwrap();
        drop(writer);

        let output_path = dir.path().join("bundle.tar.zst");
        let config =
            ExportConfig::new(&eventlog_path, &output_path).with_refusal_report(&report_path);

        let result = run_export(&config).unwrap();
        assert!(matches!(result, ExportResult::Refused(_)));

        // Check report was written with schema-compliant structure
        assert!(report_path.exists());
        let report_content = std::fs::read_to_string(&report_path).unwrap();
        let parsed: RefusalReport = serde_json::from_str(&report_content).unwrap();
        assert_eq!(parsed.report_version, "refusal-v0.1");
        assert!(!parsed.blocked_items.is_empty());
        assert!(parsed
            .blocked_items
            .iter()
            .any(|i| i.matched_pattern == "aws_access_key"));
        assert!(!parsed.eventlog_path.is_empty());
        assert!(!parsed.scan_timestamp_utc.is_empty());
    }

    #[test]
    fn refusal_report_deterministic_ordering() {
        // Items should be stably sorted by (event_id, field_path, matched_pattern)
        let items = vec![
            BlockedItem {
                event_id: "e-2".into(),
                field_path: "payload".into(),
                matched_pattern: "password".into(),
                blob_ref: None,
                redacted_match: "pass***rd12".into(),
            },
            BlockedItem {
                event_id: "e-1".into(),
                field_path: "payload".into(),
                matched_pattern: "aws_access_key".into(),
                blob_ref: None,
                redacted_match: "AKIA***MPLE".into(),
            },
            BlockedItem {
                event_id: "e-1".into(),
                field_path: "payload".into(),
                matched_pattern: "bearer_token".into(),
                blob_ref: None,
                redacted_match: "Bear***en12".into(),
            },
        ];
        let report = RefusalReport::new("/tmp/test.jsonl", items);

        // Sorted: e-1/aws_access_key, e-1/bearer_token, e-2/password
        assert_eq!(report.blocked_items[0].event_id, "e-1");
        assert_eq!(report.blocked_items[0].matched_pattern, "aws_access_key");
        assert_eq!(report.blocked_items[1].event_id, "e-1");
        assert_eq!(report.blocked_items[1].matched_pattern, "bearer_token");
        assert_eq!(report.blocked_items[2].event_id, "e-2");
        assert_eq!(report.blocked_items[2].matched_pattern, "password");
    }

    #[test]
    fn refusal_report_blob_ref_present() {
        let items = vec![BlockedItem {
            event_id: String::new(),
            field_path: "content".into(),
            matched_pattern: "private_key".into(),
            blob_ref: Some("abc123".into()),
            redacted_match: "----***Y---".into(),
        }];
        let report = RefusalReport::new("/tmp/test.jsonl", items);

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"blob_ref\":\"abc123\""));
    }

    #[test]
    fn refusal_report_blob_ref_absent_not_serialized() {
        let items = vec![BlockedItem {
            event_id: "e-1".into(),
            field_path: "payload".into(),
            matched_pattern: "password".into(),
            blob_ref: None,
            redacted_match: "pass***rd12".into(),
        }];
        let report = RefusalReport::new("/tmp/test.jsonl", items);

        let json = serde_json::to_string(&report).unwrap();
        // blob_ref should be skipped when None
        assert!(!json.contains("blob_ref"));
    }

    #[test]
    fn refusal_report_sort_is_deterministic_for_blob_ties() {
        let items = vec![
            BlockedItem {
                event_id: String::new(),
                field_path: "content".into(),
                matched_pattern: "private_key".into(),
                blob_ref: Some("z-blob".into()),
                redacted_match: "----***z---".into(),
            },
            BlockedItem {
                event_id: String::new(),
                field_path: "content".into(),
                matched_pattern: "private_key".into(),
                blob_ref: Some("a-blob".into()),
                redacted_match: "----***a---".into(),
            },
        ];
        let report = RefusalReport::new("/tmp/test.jsonl", items);
        assert_eq!(report.blocked_items.len(), 2);
        assert_eq!(report.blocked_items[0].blob_ref.as_deref(), Some("a-blob"));
        assert_eq!(report.blocked_items[1].blob_ref.as_deref(), Some("z-blob"));
    }

    // ---- M8.4: Deterministic tar+zstd bundling tests ----

    #[test]
    fn bundle_is_valid_tar_zstd() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");

        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
        writer
            .append(make_event("e1", 1_000_000_000, "hello"))
            .unwrap();
        drop(writer);

        let content = discover_content(&eventlog_path).unwrap();
        let bundle_path = dir.path().join("bundle.tar.zst");
        let result = create_bundle(&content, None, &bundle_path).unwrap();

        assert!(bundle_path.exists());
        assert_eq!(result.event_count, 1);
        assert_eq!(result.blob_count, 0);
        assert_eq!(result.bundle_hash.len(), 64); // BLAKE3 hex

        // Verify it decompresses and contains the eventlog entry
        let compressed = std::fs::read(&bundle_path).unwrap();
        let decompressed = zstd::decode_all(compressed.as_slice()).unwrap();
        let mut archive = tar::Archive::new(decompressed.as_slice());
        let entries: Vec<_> = archive
            .entries()
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(entries.len(), 2); // eventlog.jsonl + manifest.json
    }

    #[test]
    fn bundle_deterministic_same_inputs_same_bytes() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");

        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
        writer
            .append(make_event("e1", 1_000_000_000, "deterministic"))
            .unwrap();
        writer
            .append(make_event("e2", 2_000_000_000, "test"))
            .unwrap();
        drop(writer);

        let content = discover_content(&eventlog_path).unwrap();

        // Create bundle twice
        let bundle1_path = dir.path().join("bundle1.tar.zst");
        let bundle2_path = dir.path().join("bundle2.tar.zst");
        let result1 = create_bundle(&content, None, &bundle1_path).unwrap();
        let result2 = create_bundle(&content, None, &bundle2_path).unwrap();

        // Same inputs must produce identical bytes
        let bytes1 = std::fs::read(&bundle1_path).unwrap();
        let bytes2 = std::fs::read(&bundle2_path).unwrap();
        assert_eq!(
            bytes1, bytes2,
            "Bundle bytes must be identical for same inputs"
        );
        assert_eq!(result1.bundle_hash, result2.bundle_hash);
    }

    #[test]
    fn bundle_metadata_normalized() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");

        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
        writer
            .append(make_event("e1", 1_000_000_000, "metadata test"))
            .unwrap();
        drop(writer);

        let content = discover_content(&eventlog_path).unwrap();
        let bundle_path = dir.path().join("bundle.tar.zst");
        create_bundle(&content, None, &bundle_path).unwrap();

        // Decompress and verify metadata
        let compressed = std::fs::read(&bundle_path).unwrap();
        let decompressed = zstd::decode_all(compressed.as_slice()).unwrap();
        let mut archive = tar::Archive::new(decompressed.as_slice());

        for entry in archive.entries().unwrap() {
            let entry = entry.unwrap();
            let header = entry.header();
            assert_eq!(header.mtime().unwrap(), 0, "mtime must be 0");
            assert_eq!(header.uid().unwrap(), 0, "uid must be 0");
            assert_eq!(header.gid().unwrap(), 0, "gid must be 0");
            assert_eq!(
                header.username().unwrap().unwrap_or(""),
                "",
                "username must be empty"
            );
            assert_eq!(
                header.groupname().unwrap().unwrap_or(""),
                "",
                "groupname must be empty"
            );
            assert_eq!(header.mode().unwrap(), 0o644, "mode must be 0644");
        }
    }

    #[test]
    fn bundle_entries_sorted_alphabetically() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");
        let blobs_dir = dir.path().join("blobs");

        // Write blobs using BlobStore (correct on-disk layout)
        let blob_store = vifei_core::blob_store::BlobStore::open(&blobs_dir).unwrap();
        let ref_a = blob_store.write_blob(b"blob-a-data").unwrap();
        let ref_f = blob_store.write_blob(b"blob-f-data").unwrap();

        // Create EventLog with blob references
        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
        let mut ev1 = make_event("e1", 1_000_000_000, "blob test");
        ev1.payload_ref = Some(ref_f.clone());
        writer.append(ev1).unwrap();
        let mut ev2 = make_event("e2", 2_000_000_000, "blob test 2");
        ev2.payload_ref = Some(ref_a.clone());
        writer.append(ev2).unwrap();
        drop(writer);

        let content = discover_content(&eventlog_path).unwrap();
        let bundle_path = dir.path().join("bundle.tar.zst");
        create_bundle(&content, Some(&blob_store), &bundle_path).unwrap();

        // Verify entry ordering
        let compressed = std::fs::read(&bundle_path).unwrap();
        let decompressed = zstd::decode_all(compressed.as_slice()).unwrap();
        let mut archive = tar::Archive::new(decompressed.as_slice());
        let paths: Vec<String> = archive
            .entries()
            .unwrap()
            .map(|e| {
                let e = e.unwrap();
                e.path().unwrap().to_string_lossy().to_string()
            })
            .collect();

        // Must be sorted alphabetically
        let mut sorted = paths.clone();
        sorted.sort();
        assert_eq!(paths, sorted, "Entries must be in alphabetical order");

        // Verify expected entries present
        assert!(paths.iter().any(|p| p == "eventlog.jsonl"));
        assert!(paths.iter().any(|p| p == "manifest.json"));
        assert_eq!(paths.len(), 4); // eventlog + 2 blobs + manifest
    }

    #[test]
    fn bundle_hash_is_blake3_of_file_bytes() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");

        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
        writer
            .append(make_event("e1", 1_000_000_000, "hash test"))
            .unwrap();
        drop(writer);

        let content = discover_content(&eventlog_path).unwrap();
        let bundle_path = dir.path().join("bundle.tar.zst");
        let result = create_bundle(&content, None, &bundle_path).unwrap();

        // Independently hash the file bytes
        let file_bytes = std::fs::read(&bundle_path).unwrap();
        let expected_hash = blake3::hash(&file_bytes).to_hex().to_string();
        assert_eq!(result.bundle_hash, expected_hash);
    }

    // ---- M8.5: Integrity manifest tests ----

    #[test]
    fn manifest_included_in_bundle() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");

        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
        writer
            .append(make_event("e1", 1_000_000_000, "manifest test"))
            .unwrap();
        drop(writer);

        let content = discover_content(&eventlog_path).unwrap();
        let bundle_path = dir.path().join("bundle.tar.zst");
        create_bundle(&content, None, &bundle_path).unwrap();

        // Extract manifest.json from the bundle
        let compressed = std::fs::read(&bundle_path).unwrap();
        let decompressed = zstd::decode_all(compressed.as_slice()).unwrap();
        let mut archive = tar::Archive::new(decompressed.as_slice());

        let mut found_manifest = false;
        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            if entry.path().unwrap().to_string_lossy() == "manifest.json" {
                found_manifest = true;
                let mut content = String::new();
                std::io::Read::read_to_string(&mut entry, &mut content).unwrap();
                let manifest: BundleManifest = serde_json::from_str(&content).unwrap();
                assert_eq!(manifest.manifest_version, "manifest-v0.1");
                break;
            }
        }
        assert!(found_manifest, "manifest.json must be in the bundle");
    }

    #[test]
    fn manifest_file_hashes_correct() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");

        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
        writer
            .append(make_event("e1", 1_000_000_000, "hash check"))
            .unwrap();
        drop(writer);

        let eventlog_bytes = std::fs::read(&eventlog_path).unwrap();
        let expected_hash = blake3::hash(&eventlog_bytes).to_hex().to_string();

        let content = discover_content(&eventlog_path).unwrap();
        let bundle_path = dir.path().join("bundle.tar.zst");
        create_bundle(&content, None, &bundle_path).unwrap();

        // Extract and verify manifest
        let compressed = std::fs::read(&bundle_path).unwrap();
        let decompressed = zstd::decode_all(compressed.as_slice()).unwrap();
        let mut archive = tar::Archive::new(decompressed.as_slice());

        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            if entry.path().unwrap().to_string_lossy() == "manifest.json" {
                let mut json = String::new();
                std::io::Read::read_to_string(&mut entry, &mut json).unwrap();
                let manifest: BundleManifest = serde_json::from_str(&json).unwrap();

                // Find eventlog entry in manifest
                let el_entry = manifest
                    .files
                    .iter()
                    .find(|f| f.path == "eventlog.jsonl")
                    .expect("eventlog.jsonl must be in manifest");
                assert_eq!(el_entry.blake3, expected_hash);
                assert_eq!(el_entry.size, eventlog_bytes.len() as u64);
                break;
            }
        }
    }

    #[test]
    fn manifest_commit_index_range() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");

        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
        writer
            .append(make_event("e1", 1_000_000_000, "first"))
            .unwrap();
        writer
            .append(make_event("e2", 2_000_000_000, "second"))
            .unwrap();
        writer
            .append(make_event("e3", 3_000_000_000, "third"))
            .unwrap();
        drop(writer);

        let content = discover_content(&eventlog_path).unwrap();
        let bundle_path = dir.path().join("bundle.tar.zst");
        create_bundle(&content, None, &bundle_path).unwrap();

        // Extract manifest and check commit_index_range
        let compressed = std::fs::read(&bundle_path).unwrap();
        let decompressed = zstd::decode_all(compressed.as_slice()).unwrap();
        let mut archive = tar::Archive::new(decompressed.as_slice());

        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            if entry.path().unwrap().to_string_lossy() == "manifest.json" {
                let mut json = String::new();
                std::io::Read::read_to_string(&mut entry, &mut json).unwrap();
                let manifest: BundleManifest = serde_json::from_str(&json).unwrap();

                let range = manifest
                    .commit_index_range
                    .expect("commit_index_range must be present");
                assert_eq!(range[0], 0, "first commit_index");
                assert_eq!(range[1], 2, "last commit_index");
                break;
            }
        }
    }

    #[test]
    fn manifest_commit_index_range_uses_min_max_for_unordered_inputs() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");
        std::fs::write(&eventlog_path, "").unwrap();

        let content = DiscoveredContent {
            eventlog_path,
            events: vec![
                CommittedEvent::commit(make_event("e-high", 3_000_000_000, "a"), 42),
                CommittedEvent::commit(make_event("e-low", 1_000_000_000, "b"), 7),
                CommittedEvent::commit(make_event("e-mid", 2_000_000_000, "c"), 15),
            ],
            blob_refs: HashSet::new(),
        };

        let bundle_path = dir.path().join("bundle.tar.zst");
        create_bundle(&content, None, &bundle_path).unwrap();

        let compressed = std::fs::read(&bundle_path).unwrap();
        let decompressed = zstd::decode_all(compressed.as_slice()).unwrap();
        let mut archive = tar::Archive::new(decompressed.as_slice());

        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            if entry.path().unwrap().to_string_lossy() == "manifest.json" {
                let mut json = String::new();
                std::io::Read::read_to_string(&mut entry, &mut json).unwrap();
                let manifest: BundleManifest = serde_json::from_str(&json).unwrap();

                let range = manifest
                    .commit_index_range
                    .expect("commit_index_range must be present");
                assert_eq!(range[0], 7, "minimum commit_index");
                assert_eq!(range[1], 42, "maximum commit_index");
                break;
            }
        }
    }

    #[test]
    fn manifest_projection_invariants_version() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");

        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
        writer
            .append(make_event("e1", 1_000_000_000, "version check"))
            .unwrap();
        drop(writer);

        let content = discover_content(&eventlog_path).unwrap();
        let bundle_path = dir.path().join("bundle.tar.zst");
        create_bundle(&content, None, &bundle_path).unwrap();

        let compressed = std::fs::read(&bundle_path).unwrap();
        let decompressed = zstd::decode_all(compressed.as_slice()).unwrap();
        let mut archive = tar::Archive::new(decompressed.as_slice());

        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            if entry.path().unwrap().to_string_lossy() == "manifest.json" {
                let mut json = String::new();
                std::io::Read::read_to_string(&mut entry, &mut json).unwrap();
                let manifest: BundleManifest = serde_json::from_str(&json).unwrap();

                assert_eq!(
                    manifest.projection_invariants_version,
                    PROJECTION_INVARIANTS_VERSION
                );
                break;
            }
        }
    }

    #[test]
    fn manifest_files_stably_sorted() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");
        let blobs_dir = dir.path().join("blobs");

        let blob_store = vifei_core::blob_store::BlobStore::open(&blobs_dir).unwrap();
        let ref_z = blob_store.write_blob(b"z-data").unwrap();
        let ref_a = blob_store.write_blob(b"a-data").unwrap();

        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
        let mut ev1 = make_event("e1", 1_000_000_000, "sort test");
        ev1.payload_ref = Some(ref_z);
        writer.append(ev1).unwrap();
        let mut ev2 = make_event("e2", 2_000_000_000, "sort test 2");
        ev2.payload_ref = Some(ref_a);
        writer.append(ev2).unwrap();
        drop(writer);

        let content = discover_content(&eventlog_path).unwrap();
        let bundle_path = dir.path().join("bundle.tar.zst");
        create_bundle(&content, Some(&blob_store), &bundle_path).unwrap();

        let compressed = std::fs::read(&bundle_path).unwrap();
        let decompressed = zstd::decode_all(compressed.as_slice()).unwrap();
        let mut archive = tar::Archive::new(decompressed.as_slice());

        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            if entry.path().unwrap().to_string_lossy() == "manifest.json" {
                let mut json = String::new();
                std::io::Read::read_to_string(&mut entry, &mut json).unwrap();
                let manifest: BundleManifest = serde_json::from_str(&json).unwrap();

                // Manifest file entries must be sorted by path
                let paths: Vec<&str> = manifest.files.iter().map(|f| f.path.as_str()).collect();
                let mut sorted = paths.clone();
                sorted.sort();
                assert_eq!(paths, sorted, "Manifest files must be sorted by path");
                break;
            }
        }
    }
}
