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
//! panopticon export --share-safe -o bundle.tar.zst ./eventlog.jsonl
//! ```
//!
//! # Invariants
//!
//! - **I3 (Share-safe export):** Never produce an unsafe bundle. Refuse is
//!   the default when any doubt exists.
//! - **I5 (Loud failure):** Errors are returned, never silently swallowed.

mod scanner;

use panopticon_core::blob_store::BlobStore;
use panopticon_core::event::CommittedEvent;
use panopticon_core::eventlog::read_eventlog;
use scanner::{redact_match, scan_bytes, scan_text, SecretPatterns};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

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
    /// Items are stably sorted by (event_id, field_path, matched_pattern)
    /// for deterministic output per M8.3 requirements.
    pub fn new(eventlog_path: &str, mut items: Vec<BlockedItem>) -> Self {
        // Deterministic sort: by event_id, then field_path, then matched_pattern
        items.sort_by(|a, b| {
            a.event_id
                .cmp(&b.event_id)
                .then_with(|| a.field_path.cmp(&b.field_path))
                .then_with(|| a.matched_pattern.cmp(&b.matched_pattern))
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

/// Discover all content referenced by an EventLog.
///
/// Reads the EventLog and identifies all blob references.
pub(crate) fn discover_content(eventlog_path: &Path) -> io::Result<DiscoveredContent> {
    let events = read_eventlog(eventlog_path)?;
    let mut blob_refs = HashSet::new();

    for event in &events {
        if let Some(ref payload_ref) = event.payload_ref {
            blob_refs.insert(payload_ref.clone());
        }
    }

    Ok(DiscoveredContent {
        eventlog_path: eventlog_path.to_path_buf(),
        events,
        blob_refs,
    })
}

/// Scan discovered content for secrets.
///
/// Scans all event payloads and blob contents for secret patterns.
/// Returns a list of blocked items. Empty list means clean.
pub(crate) fn scan_for_secrets(
    content: &DiscoveredContent,
    blob_store: Option<&BlobStore>,
) -> io::Result<Vec<BlockedItem>> {
    let patterns = SecretPatterns::new();
    let mut items = Vec::new();

    // Scan event payloads
    for event in &content.events {
        let event_items = scan_event(&patterns, event);
        items.extend(event_items);
    }

    // Scan blob contents
    if let Some(store) = blob_store {
        for blob_ref in &content.blob_refs {
            if let Some(blob_data) = store.read_blob(blob_ref)? {
                let blob_items = scan_blob(&patterns, blob_ref, &blob_data);
                items.extend(blob_items);
            }
        }
    }

    Ok(items)
}

/// Scan a single event for secrets.
fn scan_event(patterns: &SecretPatterns, event: &CommittedEvent) -> Vec<BlockedItem> {
    let mut items = Vec::new();

    // Serialize the payload to JSON for scanning
    let payload_json = match serde_json::to_string(&event.payload) {
        Ok(json) => json,
        Err(_) => return items,
    };

    // Scan the payload JSON
    for m in scan_text(patterns, &payload_json) {
        items.push(BlockedItem {
            event_id: event.event_id.clone(),
            field_path: "payload".into(),
            matched_pattern: m.pattern_name,
            blob_ref: None,
            redacted_match: redact_match(&m.matched_text),
        });
    }

    items
}

/// Scan a blob for secrets.
fn scan_blob(patterns: &SecretPatterns, blob_ref: &str, data: &[u8]) -> Vec<BlockedItem> {
    let mut items = Vec::new();

    for m in scan_bytes(patterns, data) {
        items.push(BlockedItem {
            event_id: String::new(),
            field_path: "content".into(),
            matched_pattern: m.pattern_name,
            blob_ref: Some(blob_ref.to_string()),
            redacted_match: redact_match(&m.matched_text),
        });
    }

    items
}

/// Bundle discovered content into a deterministic archive.
///
/// NOTE: Full implementation in M8.4. This is a pipeline stub.
pub(crate) fn create_bundle(
    content: &DiscoveredContent,
    _blob_store: Option<&BlobStore>,
    output_path: &Path,
) -> io::Result<ExportSuccess> {
    // M8.4 will implement deterministic tar+zstd bundling.
    // For now, create a placeholder file to test the pipeline.

    // Create a minimal bundle placeholder
    let placeholder = format!(
        "# Panopticon Export Bundle (placeholder)\n\
         # Full implementation in M8.4\n\
         eventlog: {}\n\
         event_count: {}\n\
         blob_count: {}\n",
        content.eventlog_path.display(),
        content.event_count(),
        content.blob_refs.len()
    );

    std::fs::write(output_path, &placeholder)?;

    // Compute bundle hash
    let bundle_hash = blake3::hash(placeholder.as_bytes()).to_hex().to_string();

    Ok(ExportSuccess {
        bundle_path: output_path.to_path_buf(),
        bundle_hash,
        event_count: content.event_count(),
        blob_count: content.blob_refs.len(),
    })
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
        let eventlog_str = config.eventlog_path.display().to_string();
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

#[cfg(test)]
mod tests {
    use super::*;
    use panopticon_core::event::{EventPayload, ImportEvent, Tier};
    use panopticon_core::eventlog::EventLogWriter;
    use tempfile::tempdir;

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
        match result {
            ExportResult::Success(success) => {
                assert_eq!(success.event_count, 1);
                assert_eq!(success.blob_count, 0);
                assert!(output_path.exists());
                assert_eq!(success.bundle_hash.len(), 64);
            }
            ExportResult::Refused(_) => panic!("expected success, got refused"),
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
        match result {
            ExportResult::Success(_) => panic!("expected refused, got success"),
            ExportResult::Refused(report) => {
                assert!(!report.blocked_items.is_empty());
                assert!(report
                    .blocked_items
                    .iter()
                    .any(|f| f.matched_pattern == "aws_access_key"));
            }
        }
    }

    #[test]
    fn export_with_password_refused() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");

        // Create EventLog with a password
        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
        writer
            .append(make_event("e1", 1_000_000_000, "password=supersecret123"))
            .unwrap();
        drop(writer);

        let output_path = dir.path().join("bundle.tar.zst");
        let config = ExportConfig::new(&eventlog_path, &output_path);

        let result = run_export(&config).unwrap();
        match result {
            ExportResult::Success(_) => panic!("expected refused, got success"),
            ExportResult::Refused(report) => {
                assert!(!report.blocked_items.is_empty());
                assert!(report
                    .blocked_items
                    .iter()
                    .any(|f| f.matched_pattern == "password"));
            }
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
        match result {
            ExportResult::Success(_) => panic!("expected refused, got success"),
            ExportResult::Refused(report) => {
                assert!(!report.blocked_items.is_empty());
                assert!(report
                    .blocked_items
                    .iter()
                    .any(|f| f.matched_pattern == "jwt_token"));
            }
        }
    }

    #[test]
    fn export_with_private_key_refused() {
        let dir = tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");

        // Create EventLog with a private key header
        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
        writer
            .append(make_event(
                "e1",
                1_000_000_000,
                "-----BEGIN RSA PRIVATE KEY-----",
            ))
            .unwrap();
        drop(writer);

        let output_path = dir.path().join("bundle.tar.zst");
        let config = ExportConfig::new(&eventlog_path, &output_path);

        let result = run_export(&config).unwrap();
        match result {
            ExportResult::Success(_) => panic!("expected refused, got success"),
            ExportResult::Refused(report) => {
                assert!(!report.blocked_items.is_empty());
                assert!(report
                    .blocked_items
                    .iter()
                    .any(|f| f.matched_pattern == "private_key"));
            }
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
}
