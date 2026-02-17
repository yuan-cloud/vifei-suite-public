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

/// Bundle discovered content into a deterministic tar+zstd archive.
///
/// Determinism requirements (CAPACITY_ENVELOPE Export determinism targets):
/// - Tar format: POSIX (UStar/PAX-compatible)
/// - Zstd compression level: 3 (pinned, not library default)
/// - Tar mtime: 0 (Unix epoch, all entries normalized)
/// - Tar uid/gid: 0 (normalized to prevent machine-specific values)
/// - Tar username/groupname: empty
/// - Entries sorted alphabetically by path
/// - bundle_hash = BLAKE3 of final .tar.zst bytes
pub(crate) fn create_bundle(
    content: &DiscoveredContent,
    blob_store: Option<&BlobStore>,
    output_path: &Path,
) -> io::Result<ExportSuccess> {
    // Collect all entries as (archive_path, data) for deterministic sorting
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();

    // Add EventLog
    let eventlog_bytes = std::fs::read(&content.eventlog_path)?;
    entries.push(("eventlog.jsonl".to_string(), eventlog_bytes));

    // Add blobs (sorted by ref for deterministic ordering)
    let mut blob_count = 0usize;
    if let Some(store) = blob_store {
        let mut sorted_refs: Vec<&str> = content.blob_refs.iter().map(|s| s.as_str()).collect();
        sorted_refs.sort();
        for blob_ref in sorted_refs {
            if let Some(data) = store.read_blob(blob_ref)? {
                entries.push((format!("blobs/{}", blob_ref), data));
                blob_count += 1;
            }
        }
    }

    // Sort all entries alphabetically by path (deterministic archive order)
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    // Build tar+zstd into a memory buffer so we can BLAKE3-hash the result
    let mut compressed_bytes: Vec<u8> = Vec::new();
    {
        // Zstd level 3 (pinned per CAPACITY_ENVELOPE)
        let encoder = zstd::stream::write::Encoder::new(&mut compressed_bytes, 3)
            .map_err(|e| io::Error::other(format!("zstd init: {e}")))?;
        let mut tar_builder = tar::Builder::new(encoder);

        for (path, data) in &entries {
            let mut header = tar::Header::new_ustar();
            header.set_size(data.len() as u64);
            header.set_mtime(0);
            header.set_uid(0);
            header.set_gid(0);
            header.set_mode(0o644);
            // Empty username/groupname to prevent machine-specific values
            header
                .set_username("")
                .map_err(|e| io::Error::other(format!("set_username: {e}")))?;
            header
                .set_groupname("")
                .map_err(|e| io::Error::other(format!("set_groupname: {e}")))?;
            header.set_entry_type(tar::EntryType::Regular);
            header.set_cksum();

            tar_builder.append_data(&mut header, path, data.as_slice())?;
        }

        // Finish tar (writes final blocks), then finish zstd (flushes frame)
        let encoder = tar_builder.into_inner()?;
        encoder.finish()?;
    }

    // bundle_hash = BLAKE3 of final .tar.zst bytes
    let bundle_hash = blake3::hash(&compressed_bytes).to_hex().to_string();

    // Write the completed bundle to disk
    std::fs::write(output_path, &compressed_bytes)?;

    Ok(ExportSuccess {
        bundle_path: output_path.to_path_buf(),
        bundle_hash,
        event_count: content.event_count(),
        blob_count,
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
        assert_eq!(entries.len(), 1);
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
        let blob_store = panopticon_core::blob_store::BlobStore::open(&blobs_dir).unwrap();
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
        assert_eq!(paths.len(), 3); // eventlog + 2 blobs
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
}
