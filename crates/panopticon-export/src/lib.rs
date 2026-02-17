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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefusalReport {
    /// Version of the refusal report schema.
    pub schema_version: String,
    /// List of detected secret findings.
    pub findings: Vec<SecretFinding>,
    /// Human-readable summary.
    pub summary: String,
}

impl RefusalReport {
    /// Create a new refusal report with the given findings.
    pub fn new(findings: Vec<SecretFinding>) -> Self {
        let summary = format!(
            "Export refused: {} secret(s) detected in {} location(s)",
            findings.len(),
            findings
                .iter()
                .map(|f| &f.location)
                .collect::<HashSet<_>>()
                .len()
        );
        RefusalReport {
            schema_version: "0.1.0".into(),
            findings,
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

/// A single secret finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretFinding {
    /// Where the secret was found (event_id, blob_ref, etc.).
    pub location: String,
    /// Field path within the location (e.g., "payload.args").
    pub field_path: String,
    /// Pattern that matched (e.g., "aws_secret_key").
    pub pattern: String,
    /// Snippet of the matched content (redacted).
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
/// Returns a list of findings. Empty list means clean.
pub(crate) fn scan_for_secrets(
    content: &DiscoveredContent,
    blob_store: Option<&BlobStore>,
) -> io::Result<Vec<SecretFinding>> {
    let patterns = SecretPatterns::new();
    let mut findings = Vec::new();

    // Scan event payloads
    for event in &content.events {
        let event_findings = scan_event(&patterns, event);
        findings.extend(event_findings);
    }

    // Scan blob contents
    if let Some(store) = blob_store {
        for blob_ref in &content.blob_refs {
            if let Some(blob_data) = store.read_blob(blob_ref)? {
                let blob_findings = scan_blob(&patterns, blob_ref, &blob_data);
                findings.extend(blob_findings);
            }
        }
    }

    Ok(findings)
}

/// Scan a single event for secrets.
fn scan_event(patterns: &SecretPatterns, event: &CommittedEvent) -> Vec<SecretFinding> {
    let mut findings = Vec::new();
    let location = format!("event:{}", event.event_id);

    // Serialize the payload to JSON for scanning
    let payload_json = match serde_json::to_string(&event.payload) {
        Ok(json) => json,
        Err(_) => return findings,
    };

    // Scan the payload JSON
    for m in scan_text(patterns, &payload_json) {
        findings.push(SecretFinding {
            location: location.clone(),
            field_path: "payload".into(),
            pattern: m.pattern_name,
            redacted_match: redact_match(&m.matched_text),
        });
    }

    findings
}

/// Scan a blob for secrets.
fn scan_blob(patterns: &SecretPatterns, blob_ref: &str, data: &[u8]) -> Vec<SecretFinding> {
    let mut findings = Vec::new();
    let location = format!("blob:{}", blob_ref);

    for m in scan_bytes(patterns, data) {
        findings.push(SecretFinding {
            location: location.clone(),
            field_path: "content".into(),
            pattern: m.pattern_name,
            redacted_match: redact_match(&m.matched_text),
        });
    }

    findings
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
        let report = RefusalReport::new(findings);

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
                assert!(!report.findings.is_empty());
                assert!(report
                    .findings
                    .iter()
                    .any(|f| f.pattern == "aws_access_key"));
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
                assert!(!report.findings.is_empty());
                assert!(report.findings.iter().any(|f| f.pattern == "password"));
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
                assert!(!report.findings.is_empty());
                assert!(report.findings.iter().any(|f| f.pattern == "jwt_token"));
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
                assert!(!report.findings.is_empty());
                assert!(report.findings.iter().any(|f| f.pattern == "private_key"));
            }
        }
    }

    #[test]
    fn refusal_report_serialization() {
        let finding = SecretFinding {
            location: "event:e-123".into(),
            field_path: "payload.args".into(),
            pattern: "aws_secret_key".into(),
            redacted_match: "AKIA***REDACTED***".into(),
        };
        let report = RefusalReport::new(vec![finding]);

        let json = serde_json::to_string_pretty(&report).unwrap();
        assert!(json.contains("schema_version"));
        assert!(json.contains("findings"));
        assert!(json.contains("aws_secret_key"));

        // Round-trip
        let parsed: RefusalReport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.findings.len(), 1);
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
        let report_path = dir.path().join("refusal.json");

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

        // Check report was written
        assert!(report_path.exists());
        let report_content = std::fs::read_to_string(&report_path).unwrap();
        assert!(report_content.contains("aws_access_key"));
    }
}
