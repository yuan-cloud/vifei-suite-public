//! Append-only EventLog writer — the sole assigner of `commit_index`.
//!
//! # Overview
//!
//! The `EventLogWriter` is the single code path for writing events to the
//! JSONL EventLog file. It enforces:
//!
//! - **Monotonic `commit_index`:** Starts at 0 for new files, increments by
//!   exactly 1 per appended event. Assigned here and nowhere else (D6).
//! - **JSONL format:** One JSON object per line, newline-terminated, no
//!   pretty printing, UTF-8 bytes.
//! - **Fsync per Tier A event:** See `docs/CAPACITY_ENVELOPE.md`.
//! - **Line size limit:** Rejects serialized events exceeding the max line
//!   bytes budget in `docs/CAPACITY_ENVELOPE.md`.
//!
//! # Clock skew detection
//!
//! Tracks last-seen `timestamp_ns` per `source_id`. When a source's
//! timestamp moves backward beyond the tolerance in
//! `docs/CAPACITY_ENVELOPE.md`, emits a `ClockSkewDetected` Tier A event
//! **before** the triggering event. Both events get their own
//! `commit_index`. Timestamps are metadata only (D6) — skew is surfaced,
//! never corrected.
//!
//! # Blob integration
//!
//! The writer can optionally integrate with a [`BlobStore`] to externalize
//! large payloads. When a payload field exceeds the inline threshold, the
//! caller should store it as a blob and set `payload_ref` on the event.
//! The writer itself does not perform blob decisions — that is the caller's
//! responsibility (typically the importer or ingestion pipeline).
//!
//! # Error handling
//!
//! Write failures (fsync error, oversized line) return `io::Error`. The
//! caller is responsible for entering L5 safe failure posture per
//! `FM-APPEND-FAIL` in `docs/BACKPRESSURE_POLICY.md`.
//!
//! # Invariants
//!
//! - **I1 (Forensic truth):** EventLog is append-only canonical truth.
//! - **I5 (Loud failure):** Write errors are returned, never swallowed.
//! - **D6 (Canonical ordering):** `commit_index` assigned here only.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use crate::event::{CommittedEvent, EventPayload, ImportEvent, Tier};

/// Maximum serialized event line size in bytes. Events serializing to more
/// than this are rejected to prevent unbounded memory usage.
/// Value from `docs/CAPACITY_ENVELOPE.md`.
const EVENTLOG_MAX_LINE_BYTES: usize = 1_048_576;

/// Clock skew tolerance in nanoseconds. A backward timestamp delta
/// exceeding this triggers a `ClockSkewDetected` event.
/// Value from `docs/CAPACITY_ENVELOPE.md`.
const CLOCK_SKEW_TOLERANCE_NS: u64 = 50_000_000;

/// Append-only EventLog writer.
///
/// The sole assigner of `commit_index`. Pass explicitly, not a global.
pub struct EventLogWriter {
    /// The JSONL file handle.
    file: File,
    /// Path to the EventLog file.
    #[allow(dead_code)] // Will be used for reload/recovery
    path: PathBuf,
    /// Next `commit_index` to assign.
    next_index: u64,
    /// Last-seen `timestamp_ns` per `source_id` for clock skew detection.
    source_timestamps: HashMap<String, u64>,
}

/// Result of appending an event, including any generated detection events.
#[derive(Debug)]
pub struct AppendResult {
    /// The committed event that was appended.
    #[allow(dead_code)] // Will be used by importer/TUI
    pub(crate) committed: CommittedEvent,
    /// Any detection events (e.g., `ClockSkewDetected`) appended before
    /// the main event. These have their own `commit_index` values.
    #[allow(dead_code)] // Will be used by importer/TUI
    pub(crate) detection_events: Vec<CommittedEvent>,
}

impl AppendResult {
    /// Returns detection events written before the main committed event.
    pub fn detection_events(&self) -> &[CommittedEvent] {
        &self.detection_events
    }

    /// Returns the committed event appended for the input import event.
    pub fn committed_event(&self) -> &CommittedEvent {
        &self.committed
    }
}

#[derive(Default)]
struct ScanMetadata {
    highest_commit_index: Option<u64>,
    source_timestamps: HashMap<String, u64>,
}

impl EventLogWriter {
    /// Open or create an EventLog at the given path.
    ///
    /// If the file exists, scans it to find the highest `commit_index` and
    /// resumes from there. If new, starts at `commit_index = 0`.
    pub fn open(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let metadata = if path.exists() {
            Self::scan_metadata(&path)?
        } else {
            ScanMetadata::default()
        };
        let next_index = metadata
            .highest_commit_index
            .map_or(0, |highest| highest + 1);

        let file = OpenOptions::new().create(true).append(true).open(&path)?;

        Ok(EventLogWriter {
            file,
            path,
            next_index,
            source_timestamps: metadata.source_timestamps,
        })
    }

    /// Append an import event to the EventLog.
    ///
    /// Assigns the next monotonic `commit_index`. May emit
    /// `ClockSkewDetected` events before the main event if the source's
    /// timestamp moved backward beyond tolerance.
    ///
    /// Returns an `AppendResult` containing the committed event and any
    /// detection events.
    pub fn append(&mut self, event: ImportEvent) -> io::Result<AppendResult> {
        let mut detection_events = Vec::new();

        // Clock skew detection: check before appending the main event.
        if let Some(skew_event) = self.check_clock_skew(&event) {
            let committed_skew = self.write_committed(skew_event)?;
            detection_events.push(committed_skew);
        }

        // Append the main event.
        let committed = self.write_committed(event)?;

        Ok(AppendResult {
            committed,
            detection_events,
        })
    }

    /// The `commit_index` that will be assigned to the next appended event.
    #[allow(dead_code)] // Will be used for recovery/resume
    pub(crate) fn next_index(&self) -> u64 {
        self.next_index
    }

    /// Path to the EventLog file.
    #[allow(dead_code)] // Will be used for recovery/reload
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Commit and write a single event to the JSONL file.
    fn write_committed(&mut self, event: ImportEvent) -> io::Result<CommittedEvent> {
        let committed = CommittedEvent::commit(event, self.next_index);
        let mut line = serde_json::to_string(&committed).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("event serialization failed: {e}"),
            )
        })?;

        // Line size check (before adding newline).
        if line.len() > EVENTLOG_MAX_LINE_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "serialized event exceeds max line bytes ({} > {})",
                    line.len(),
                    EVENTLOG_MAX_LINE_BYTES
                ),
            ));
        }

        line.push('\n');
        self.file.write_all(line.as_bytes())?;

        // Fsync per Tier A event.
        if committed.tier.is_lossless() {
            self.file.sync_all()?;
        }

        self.next_index += 1;
        Ok(committed)
    }

    /// Check for clock skew and return a `ClockSkewDetected` import event
    /// if the source's timestamp moved backward beyond tolerance.
    fn check_clock_skew(&mut self, event: &ImportEvent) -> Option<ImportEvent> {
        let last_ts = self
            .source_timestamps
            .get(&event.source_id)
            .copied()
            .unwrap_or(0);

        // Update last-seen timestamp (even if skewed, we track the latest
        // seen value to avoid repeated detections for the same plateau).
        if event.timestamp_ns > last_ts {
            self.source_timestamps
                .insert(event.source_id.clone(), event.timestamp_ns);
        }

        // Detect backward movement beyond tolerance.
        if last_ts > 0 && event.timestamp_ns < last_ts {
            let delta = last_ts - event.timestamp_ns;
            if delta > CLOCK_SKEW_TOLERANCE_NS {
                return Some(ImportEvent {
                    run_id: event.run_id.clone(),
                    event_id: format!("clock-skew:{}:{}", event.source_id, self.next_index),
                    source_id: event.source_id.clone(),
                    source_seq: None,
                    timestamp_ns: event.timestamp_ns,
                    tier: Tier::A,
                    payload: EventPayload::ClockSkewDetected {
                        expected_ns: last_ts,
                        actual_ns: event.timestamp_ns,
                        delta_ns: delta,
                    },
                    payload_ref: None,
                    synthesized: true,
                });
            }
        }

        None
    }

    /// Scan existing EventLog data needed to resume writer state.
    ///
    /// Includes:
    /// - highest committed index for monotonic continuation
    /// - latest timestamp per source for skew detection across restarts
    fn scan_metadata(path: &Path) -> io::Result<ScanMetadata> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut metadata = ScanMetadata::default();

        for (line_no, line) in reader.lines().enumerate() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Parse committed events from JSONL. Fail loudly on malformed lines
            // to avoid silently resuming from a corrupted truth log.
            let event = serde_json::from_str::<CommittedEvent>(trimmed).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "failed to parse EventLog line {} while resuming writer: {e}",
                        line_no + 1
                    ),
                )
            })?;
            metadata.highest_commit_index = Some(match metadata.highest_commit_index {
                Some(h) => h.max(event.commit_index),
                None => event.commit_index,
            });
            metadata
                .source_timestamps
                .entry(event.source_id)
                .and_modify(|existing| *existing = (*existing).max(event.timestamp_ns))
                .or_insert(event.timestamp_ns);
        }

        Ok(metadata)
    }
}

/// Read all committed events from an EventLog file.
///
/// Returns events in file order (which should be `commit_index` order).
pub fn read_eventlog(path: &Path) -> io::Result<Vec<CommittedEvent>> {
    let content = fs::read_to_string(path)?;
    let mut events = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let event: CommittedEvent = serde_json::from_str(trimmed).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to parse EventLog line: {e}"),
            )
        })?;
        events.push(event);
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventPayload, ImportEvent, Tier};

    /// Helper to create a minimal ImportEvent.
    fn make_event(source_id: &str, timestamp_ns: u64) -> ImportEvent {
        ImportEvent {
            run_id: "run-1".into(),
            event_id: format!("{source_id}:{timestamp_ns}"),
            source_id: source_id.into(),
            source_seq: Some(0),
            timestamp_ns,
            tier: Tier::A,
            payload: EventPayload::RunStart {
                agent: "test".into(),
                args: None,
            },
            payload_ref: None,
            synthesized: false,
        }
    }

    // -------------------------------------------------------------------
    // M2.4: Append writer monotonicity tests
    // -------------------------------------------------------------------

    #[test]
    fn append_1000_events_monotonic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eventlog.jsonl");
        let mut writer = EventLogWriter::open(&path).unwrap();

        for i in 0..1000 {
            let event = ImportEvent {
                run_id: "run-1".into(),
                event_id: format!("e-{i}"),
                source_id: "test".into(),
                source_seq: Some(i),
                timestamp_ns: 1_000_000_000 + i * 1_000_000,
                tier: Tier::A,
                payload: EventPayload::ToolCall {
                    tool: "bash".into(),
                    args: Some(format!("cmd-{i}")),
                },
                payload_ref: None,
                synthesized: false,
            };
            let result = writer.append(event).unwrap();
            assert_eq!(result.committed.commit_index, i);
        }

        // Read back and verify full sequence.
        let events = read_eventlog(&path).unwrap();
        assert_eq!(events.len(), 1000);
        for (i, event) in events.iter().enumerate() {
            assert_eq!(
                event.commit_index, i as u64,
                "event {i} has wrong commit_index"
            );
        }
    }

    #[test]
    fn resume_from_existing_eventlog() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eventlog.jsonl");

        // Write 10 events.
        {
            let mut writer = EventLogWriter::open(&path).unwrap();
            for i in 0..10 {
                let event = ImportEvent {
                    run_id: "run-1".into(),
                    event_id: format!("e-{i}"),
                    source_id: "test".into(),
                    source_seq: Some(i),
                    timestamp_ns: 1_000_000_000 + i * 1_000_000,
                    tier: Tier::A,
                    payload: EventPayload::RunStart {
                        agent: "test".into(),
                        args: None,
                    },
                    payload_ref: None,
                    synthesized: false,
                };
                writer.append(event).unwrap();
            }
        }

        // Re-open and continue.
        let mut writer = EventLogWriter::open(&path).unwrap();
        assert_eq!(writer.next_index(), 10);

        let result = writer.append(make_event("test", 2_000_000_000)).unwrap();
        assert_eq!(result.committed.commit_index, 10);
    }

    #[test]
    fn new_eventlog_starts_at_zero() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eventlog.jsonl");
        let mut writer = EventLogWriter::open(&path).unwrap();
        assert_eq!(writer.next_index(), 0);

        let result = writer.append(make_event("test", 1_000_000_000)).unwrap();
        assert_eq!(result.committed.commit_index, 0);
    }

    #[test]
    fn existing_empty_eventlog_starts_at_zero() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eventlog.jsonl");
        std::fs::File::create(&path).unwrap();

        let mut writer = EventLogWriter::open(&path).unwrap();
        assert_eq!(writer.next_index(), 0);

        let result = writer.append(make_event("test", 1_000_000_000)).unwrap();
        assert_eq!(result.committed.commit_index, 0);
    }

    #[test]
    fn commit_index_is_writer_assigned() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eventlog.jsonl");
        let mut writer = EventLogWriter::open(&path).unwrap();

        // ImportEvent has no commit_index. Writer assigns it.
        let import = make_event("test", 1_000_000_000);
        let result = writer.append(import).unwrap();
        assert_eq!(result.committed.commit_index, 0);
    }

    #[test]
    fn max_line_bytes_rejection() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eventlog.jsonl");
        let mut writer = EventLogWriter::open(&path).unwrap();

        // Create an event with a huge inline payload.
        let huge_args = "x".repeat(EVENTLOG_MAX_LINE_BYTES + 1);
        let event = ImportEvent {
            run_id: "run-1".into(),
            event_id: "e-huge".into(),
            source_id: "test".into(),
            source_seq: Some(0),
            timestamp_ns: 1_000_000_000,
            tier: Tier::A,
            payload: EventPayload::ToolCall {
                tool: "bash".into(),
                args: Some(huge_args),
            },
            payload_ref: None,
            synthesized: false,
        };

        let result = writer.append(event);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("max line bytes"));
    }

    #[test]
    fn jsonl_format_one_line_per_event() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eventlog.jsonl");
        let mut writer = EventLogWriter::open(&path).unwrap();

        for i in 0..5 {
            writer
                .append(make_event("test", 1_000_000_000 + i * 1_000_000))
                .unwrap();
        }
        drop(writer);

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 5);
        for line in &lines {
            assert!(!line.is_empty());
            // Each line must be valid JSON.
            serde_json::from_str::<CommittedEvent>(line).unwrap();
        }
    }

    // -------------------------------------------------------------------
    // M2.5: Blob store integration tests (via append writer)
    // -------------------------------------------------------------------

    #[test]
    fn blob_store_with_writer() {
        use crate::blob_store::BlobStore;

        let dir = tempfile::tempdir().unwrap();
        let eventlog_path = dir.path().join("eventlog.jsonl");
        let blob_store = BlobStore::open(dir.path().join("blobs")).unwrap();
        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();

        // Create a payload above inline threshold.
        let large_payload = vec![b'A'; crate::blob_store::INLINE_PAYLOAD_MAX_BYTES + 100];
        let payload_ref = blob_store.write_blob(&large_payload).unwrap();

        // Create event with payload_ref (inline args omitted).
        let event = ImportEvent {
            run_id: "run-1".into(),
            event_id: "e-blob".into(),
            source_id: "test".into(),
            source_seq: Some(0),
            timestamp_ns: 1_000_000_000,
            tier: Tier::A,
            payload: EventPayload::ToolCall {
                tool: "bash".into(),
                args: None,
            },
            payload_ref: Some(payload_ref.clone()),
            synthesized: false,
        };
        writer.append(event).unwrap();

        // Read back EventLog and verify payload_ref resolves.
        let events = read_eventlog(&eventlog_path).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].payload_ref.as_deref(), Some(payload_ref.as_str()));

        // Resolve blob content.
        let blob_data = blob_store.read_blob(&payload_ref).unwrap().unwrap();
        assert_eq!(blob_data, large_payload);
    }

    // -------------------------------------------------------------------
    // M2.6: Clock skew detection tests
    // -------------------------------------------------------------------

    #[test]
    fn clock_skew_detected_beyond_tolerance() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eventlog.jsonl");
        let mut writer = EventLogWriter::open(&path).unwrap();

        // First event at t=2s.
        writer.append(make_event("src-1", 2_000_000_000)).unwrap();

        // Second event at t=1s (1s backward > 50ms tolerance).
        let result = writer.append(make_event("src-1", 1_000_000_000)).unwrap();

        assert_eq!(
            result.detection_events.len(),
            1,
            "should emit ClockSkewDetected"
        );
        let skew = &result.detection_events[0];
        assert_eq!(skew.tier, Tier::A);
        assert!(skew.synthesized);
        assert!(
            matches!(&skew.payload, EventPayload::ClockSkewDetected { .. }),
            "expected ClockSkewDetected payload"
        );
        if let EventPayload::ClockSkewDetected {
            expected_ns,
            actual_ns,
            delta_ns,
        } = &skew.payload
        {
            assert_eq!(*expected_ns, 2_000_000_000);
            assert_eq!(*actual_ns, 1_000_000_000);
            assert_eq!(*delta_ns, 1_000_000_000);
        }

        // Skew event gets commit_index before the main event.
        assert_eq!(skew.commit_index, 1); // after first event (index 0)
        assert_eq!(result.committed.commit_index, 2);
    }

    #[test]
    fn clock_skew_within_tolerance_no_detection() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eventlog.jsonl");
        let mut writer = EventLogWriter::open(&path).unwrap();

        // First event at t=2s.
        writer.append(make_event("src-1", 2_000_000_000)).unwrap();

        // Second event backward by exactly 50ms (at tolerance boundary).
        let result = writer
            .append(make_event("src-1", 2_000_000_000 - CLOCK_SKEW_TOLERANCE_NS))
            .unwrap();

        assert!(
            result.detection_events.is_empty(),
            "at-tolerance should not trigger detection"
        );
    }

    #[test]
    fn clock_skew_just_beyond_tolerance() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eventlog.jsonl");
        let mut writer = EventLogWriter::open(&path).unwrap();

        // First event at t=2s.
        writer.append(make_event("src-1", 2_000_000_000)).unwrap();

        // Backward by tolerance + 1 ns.
        let result = writer
            .append(make_event(
                "src-1",
                2_000_000_000 - CLOCK_SKEW_TOLERANCE_NS - 1,
            ))
            .unwrap();

        assert_eq!(
            result.detection_events.len(),
            1,
            "just beyond tolerance should trigger"
        );
    }

    #[test]
    fn clock_skew_multiple_sources_independent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eventlog.jsonl");
        let mut writer = EventLogWriter::open(&path).unwrap();

        // Source A at t=2s, source B at t=3s.
        writer.append(make_event("src-a", 2_000_000_000)).unwrap();
        writer.append(make_event("src-b", 3_000_000_000)).unwrap();

        // Source A goes backward (triggers), source B goes forward (no trigger).
        let result_a = writer.append(make_event("src-a", 1_000_000_000)).unwrap();
        let result_b = writer.append(make_event("src-b", 4_000_000_000)).unwrap();

        assert_eq!(
            result_a.detection_events.len(),
            1,
            "source A should trigger"
        );
        assert!(
            result_b.detection_events.is_empty(),
            "source B should not trigger"
        );
    }

    #[test]
    fn clock_skew_event_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eventlog.jsonl");
        let mut writer = EventLogWriter::open(&path).unwrap();

        writer
            .append(make_event("my-source", 5_000_000_000))
            .unwrap();
        let result = writer
            .append(make_event("my-source", 3_000_000_000))
            .unwrap();

        let skew = &result.detection_events[0];
        assert_eq!(skew.source_id, "my-source");
        assert_eq!(skew.run_id, "run-1");
        assert!(skew.event_id.starts_with("clock-skew:my-source:"));
        assert_eq!(skew.tier, Tier::A);
        assert!(skew.synthesized);

        assert!(
            matches!(&skew.payload, EventPayload::ClockSkewDetected { .. }),
            "wrong payload type"
        );
        if let EventPayload::ClockSkewDetected {
            expected_ns,
            actual_ns,
            delta_ns,
        } = &skew.payload
        {
            assert_eq!(*expected_ns, 5_000_000_000);
            assert_eq!(*actual_ns, 3_000_000_000);
            assert_eq!(*delta_ns, 2_000_000_000);
        }
    }

    #[test]
    fn read_eventlog_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eventlog.jsonl");
        let mut writer = EventLogWriter::open(&path).unwrap();

        for i in 0..5 {
            writer
                .append(make_event("test", 1_000_000_000 + i * 1_000_000))
                .unwrap();
        }
        drop(writer);

        let events = read_eventlog(&path).unwrap();
        assert_eq!(events.len(), 5);
        for (i, event) in events.iter().enumerate() {
            assert_eq!(event.commit_index, i as u64);
            assert_eq!(event.source_id, "test");
        }
    }

    #[test]
    fn clock_skew_detected_after_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eventlog.jsonl");

        {
            let mut writer = EventLogWriter::open(&path).unwrap();
            writer.append(make_event("src-1", 2_000_000_000)).unwrap();
        }

        // Reopen writer and ensure historical source timestamp is restored.
        let mut writer = EventLogWriter::open(&path).unwrap();
        let result = writer.append(make_event("src-1", 1_000_000_000)).unwrap();

        assert_eq!(
            result.detection_events.len(),
            1,
            "clock skew should still be detected after writer reopen"
        );
        assert!(
            matches!(
                &result.detection_events[0].payload,
                EventPayload::ClockSkewDetected { .. }
            ),
            "expected ClockSkewDetected payload"
        );
        if let EventPayload::ClockSkewDetected {
            expected_ns,
            actual_ns,
            delta_ns,
        } = &result.detection_events[0].payload
        {
            assert_eq!(*expected_ns, 2_000_000_000);
            assert_eq!(*actual_ns, 1_000_000_000);
            assert_eq!(*delta_ns, 1_000_000_000);
        }
    }

    #[test]
    fn open_fails_loudly_on_malformed_existing_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eventlog.jsonl");
        std::fs::write(&path, "{\"not\":\"a-committed-event\"}\n").unwrap();

        let result = EventLogWriter::open(&path);
        assert!(
            result.is_err(),
            "open() should fail for malformed existing EventLog"
        );
        let err = result.err().unwrap();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("failed to parse EventLog line 1"));
    }
}
