//! Event schema v0.1 -- foundational types for the Vifei EventLog.
//!
//! # Type hierarchy
//!
//! This module uses a **two-type pattern** to enforce `commit_index` ownership
//! at compile time (locked decision D6 from `PLANS.md`):
//!
//! - [`ImportEvent`]: produced by importers. Has all event fields **except**
//!   `commit_index`. Importers cannot set `commit_index` because the field
//!   does not exist on this type.
//!
//! - [`CommittedEvent`]: produced exclusively by the append writer
//!   (`eventlog.rs`, M2). Contains all fields from `ImportEvent` plus
//!   `commit_index: u64`. This is what gets serialized to JSONL in the
//!   EventLog.
//!
//! # Byte stability strategy
//!
//! Deterministic serialization is a hard requirement (invariant I4). This
//! module enforces it via:
//!
//! - **Struct field order is canonical.** `serde` serializes fields in
//!   declaration order by default. The field order in each struct IS the
//!   canonical serialization order. Do not reorder fields without updating
//!   tests.
//!
//! - **Allowed containers:** `BTreeMap<K, V>` (sorted keys), `Vec<T>`
//!   (ordered). **Forbidden:** `HashMap` (nondeterministic iteration),
//!   `serde_json::Value` (opaque ordering).
//!
//! - **Optional fields:** `Option<T>` fields use
//!   `#[serde(skip_serializing_if = "Option::is_none")]` and
//!   `#[serde(default)]`. When `None`, the field is omitted from JSON.
//!
//! - **`synthesized` field:** Omitted when `false` via
//!   `#[serde(skip_serializing_if)]`, defaults to `false` on read via
//!   `#[serde(default)]`. Keeps common events compact.
//!
//! # Canonical JSONL field order for [`CommittedEvent`]
//!
//! ```text
//! commit_index, run_id, event_id, source_id, [source_seq], timestamp_ns,
//! tier, payload, [payload_ref], [synthesized]
//! ```
//!
//! Fields in brackets are omitted when `None` / `false`.
//!
//! # Payload variants
//!
//! [`EventPayload`] uses `#[serde(tag = "type")]` (internally tagged). The
//! `type` field appears first in the payload object, followed by
//! variant-specific fields in declaration order.
//!
//! # Event tiers
//!
//! See `docs/BACKPRESSURE_POLICY.md` for tier definitions:
//! - **Tier A:** Never dropped, never reordered. Forensic truth.
//! - **Tier B:** May be sampled under load.
//! - **Tier C:** Best-effort telemetry.
//!
//! # Inline payload threshold
//!
//! See `docs/CAPACITY_ENVELOPE.md` for the inline payload max bytes
//! threshold. Payloads above this threshold should be stored as blobs
//! referenced by `payload_ref` (BLAKE3 hex digest).
//!
//! # Invariants enforced
//!
//! - **I1 (Forensic truth):** EventLog structure with lossless Tier A
//!   ordered by `commit_index`.
//! - **I4 (Testable determinism):** Byte-stable round-trip serialization
//!   verified by tests.
//! - **D6 (Canonical ordering):** `commit_index` ownership enforced by the
//!   two-type pattern. Importers produce [`ImportEvent`] (no
//!   `commit_index`); only the append writer creates [`CommittedEvent`] via
//!   [`CommittedEvent::commit`].

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Tier enum (M1.1)
// ---------------------------------------------------------------------------

/// Event importance tier for backpressure classification.
///
/// See `docs/BACKPRESSURE_POLICY.md` for the full tier contract.
///
/// Ordering: `A > B > C` (A is highest priority).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tier {
    /// Never dropped, never reordered. Forensic truth.
    A,
    /// May be sampled, aggregated, or collapsed under load.
    B,
    /// Best-effort telemetry. Can be dropped under stress.
    C,
}

impl Tier {
    /// Returns true if this tier is never dropped (Tier A).
    pub fn is_lossless(&self) -> bool {
        matches!(self, Tier::A)
    }
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tier::A => write!(f, "A"),
            Tier::B => write!(f, "B"),
            Tier::C => write!(f, "C"),
        }
    }
}

impl FromStr for Tier {
    type Err = TierParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "A" | "a" => Ok(Tier::A),
            "B" | "b" => Ok(Tier::B),
            "C" | "c" => Ok(Tier::C),
            _ => Err(TierParseError(s.to_string())),
        }
    }
}

/// Error returned when parsing an invalid tier string.
#[derive(Debug, Clone)]
pub struct TierParseError(pub String);

impl fmt::Display for TierParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid tier: {:?} (expected A, B, or C)", self.0)
    }
}

impl std::error::Error for TierParseError {}

// Custom Ord: A > B > C (A is most important).
impl PartialOrd for Tier {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Tier {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        fn rank(t: &Tier) -> u8 {
            match t {
                Tier::A => 2,
                Tier::B => 1,
                Tier::C => 0,
            }
        }
        rank(self).cmp(&rank(other))
    }
}

// ---------------------------------------------------------------------------
// Event payload variants (M1.3)
// ---------------------------------------------------------------------------

/// Tier A event payloads plus a Generic variant for future Tier B/C events.
///
/// Uses `#[serde(tag = "type")]` for internally tagged serialization.
/// Each variant's `type` field value is the variant name (e.g., `"RunStart"`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EventPayload {
    /// Beginning of an agent run.
    RunStart {
        /// Agent or tool identifier.
        agent: String,
        /// Command or arguments that started the run.
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        args: Option<String>,
    },

    /// End of an agent run.
    RunEnd {
        /// Process exit code, if available.
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        exit_code: Option<i32>,
        /// Human-readable summary or reason for termination.
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        reason: Option<String>,
    },

    /// Agent invokes a tool.
    ///
    /// `args` may be large (e.g., full LLM prompt). When above the inline
    /// threshold (see `docs/CAPACITY_ENVELOPE.md`), the content should be
    /// stored as a blob and referenced via the event's `payload_ref` field.
    ToolCall {
        /// Tool name.
        tool: String,
        /// Tool arguments (inline). Omit if blobbed via `payload_ref`.
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        args: Option<String>,
    },

    /// Tool returns a result.
    ///
    /// `result` may be large. Same blobbing rules as `ToolCall::args`.
    ToolResult {
        /// Tool name.
        tool: String,
        /// Result content (inline). Omit if blobbed via `payload_ref`.
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        result: Option<String>,
        /// Status indicator (e.g., "success", "error").
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        status: Option<String>,
    },

    /// Backpressure or policy decision made by the system.
    ///
    /// Tour CI assertions (M7) cross-reference these events with
    /// `metrics.json.degradation_transitions`. The fields here must be
    /// sufficient for that derivation.
    PolicyDecision {
        /// Ladder level before the transition (e.g., "L0").
        from_level: String,
        /// Ladder level after the transition (e.g., "L1").
        to_level: String,
        /// What triggered the transition.
        trigger: String,
        /// Normalized queue pressure ratio `[0.0, 1.0]` at decision time.
        ///
        /// Serialized via serde_json's Ryu algorithm, which produces
        /// canonical shortest-representation output for finite f64 values.
        /// This is deterministic across platforms. Documented here per the
        /// project's floats policy (PLANS.md "Floats policy").
        queue_pressure: f64,
    },

    /// Redaction applied to an event or field.
    RedactionApplied {
        /// The `event_id` of the event that was redacted.
        target_event_id: String,
        /// Dot-delimited path within the event (e.g., `"payload.args"`).
        field_path: String,
        /// Reason for redaction.
        reason: String,
    },

    /// An error occurred during processing.
    Error {
        /// Error classification (e.g., "io", "parse", "storage").
        kind: String,
        /// Human-readable error message.
        message: String,
        /// Severity level (e.g., "warning", "critical").
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        severity: Option<String>,
    },

    /// Source timestamp moved backward beyond the clock skew tolerance.
    ///
    /// See `docs/CAPACITY_ENVELOPE.md` for the tolerance threshold.
    ClockSkewDetected {
        /// The expected minimum timestamp (nanoseconds) based on prior events.
        expected_ns: u64,
        /// The actual timestamp observed.
        actual_ns: u64,
        /// The backward delta (expected - actual), always positive.
        delta_ns: u64,
    },

    /// Generic event for future Tier B/C extension.
    ///
    /// New event types can be added here without schema-breaking changes.
    /// The `data` field uses `BTreeMap<String, String>` (not
    /// `serde_json::Value`) to guarantee deterministic serialization with
    /// sorted keys. For large or dynamic payloads, use `payload_ref` on
    /// the event wrapper.
    Generic {
        /// Event type name (e.g., "HeartBeat", "MetricSnapshot").
        event_type: String,
        /// Small structured data as sorted key-value pairs.
        #[serde(default)]
        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        data: BTreeMap<String, String>,
    },
}

impl EventPayload {
    /// Returns the event type name as it appears in the JSON `type` field.
    pub fn event_type_name(&self) -> &str {
        match self {
            EventPayload::RunStart { .. } => "RunStart",
            EventPayload::RunEnd { .. } => "RunEnd",
            EventPayload::ToolCall { .. } => "ToolCall",
            EventPayload::ToolResult { .. } => "ToolResult",
            EventPayload::PolicyDecision { .. } => "PolicyDecision",
            EventPayload::RedactionApplied { .. } => "RedactionApplied",
            EventPayload::Error { .. } => "Error",
            EventPayload::ClockSkewDetected { .. } => "ClockSkewDetected",
            EventPayload::Generic { .. } => "Generic",
        }
    }
}

// ---------------------------------------------------------------------------
// Import event -- what importers produce (M1.2)
// ---------------------------------------------------------------------------

/// An event produced by an importer, before `commit_index` assignment.
///
/// This is the importer-facing type. It deliberately lacks `commit_index`
/// so that importers cannot set it -- enforcing D6 at compile time.
/// The append writer (M2) converts this into a [`CommittedEvent`] by
/// assigning the next monotonic `commit_index`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportEvent {
    /// Identity of the run. Scopes uniqueness of `event_id`.
    pub run_id: String,
    /// Unique event identifier within `run_id`.
    /// Recommended format when source has no ID: `"{source_id}:{source_seq}"`.
    pub event_id: String,
    /// Identifies the source or importer that produced this event.
    pub source_id: String,
    /// Monotonic sequence number per `source_id` for a given run, when
    /// available. If unknown, the importer should set `synthesized: true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub source_seq: Option<u64>,
    /// Timestamp in nanoseconds. Informative metadata only -- never used
    /// for canonical ordering (D6).
    pub timestamp_ns: u64,
    /// Event importance tier for backpressure classification.
    pub tier: Tier,
    /// The structured event payload (variant-specific data).
    pub payload: EventPayload,
    /// BLAKE3 hex digest referencing a blobbed payload, if the inline
    /// content exceeds the threshold in `docs/CAPACITY_ENVELOPE.md`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub payload_ref: Option<String>,
    /// True if any field was inferred or synthesized by the importer
    /// rather than observed in the source data. See D2 rule in `PLANS.md`.
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    pub synthesized: bool,
}

// ---------------------------------------------------------------------------
// Committed event -- what goes into the EventLog (M1.2)
// ---------------------------------------------------------------------------

/// An event committed to the EventLog with a canonical `commit_index`.
///
/// Only the append writer (`eventlog.rs`) creates these via
/// [`CommittedEvent::commit`]. The `commit_index` is the canonical replay
/// order (D6). All projections iterate by `commit_index`, never by
/// `timestamp_ns`.
///
/// # Canonical JSONL field order
///
/// ```text
/// commit_index, run_id, event_id, source_id, [source_seq], timestamp_ns,
/// tier, payload, [payload_ref], [synthesized]
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommittedEvent {
    /// Canonical replay order. Assigned by the append writer only.
    /// Monotonically increasing, starting at 0, incrementing by 1.
    pub commit_index: u64,
    /// Identity of the run.
    pub run_id: String,
    /// Unique event identifier within `run_id`.
    pub event_id: String,
    /// Identifies the source or importer.
    pub source_id: String,
    /// Monotonic sequence number per source, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub source_seq: Option<u64>,
    /// Timestamp in nanoseconds. Metadata only.
    pub timestamp_ns: u64,
    /// Event importance tier.
    pub tier: Tier,
    /// The structured event payload.
    pub payload: EventPayload,
    /// BLAKE3 hex digest of blobbed payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub payload_ref: Option<String>,
    /// True if any field was synthesized rather than observed.
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    pub synthesized: bool,
}

/// Helper for `#[serde(skip_serializing_if)]` on bool fields.
fn is_false(v: &bool) -> bool {
    !v
}

impl CommittedEvent {
    /// Commit an import event by assigning a `commit_index`.
    ///
    /// This is the ONLY way to create a `CommittedEvent`. The append writer
    /// calls this to wrap an importer-produced event with canonical ordering.
    pub fn commit(event: ImportEvent, commit_index: u64) -> Self {
        CommittedEvent {
            commit_index,
            run_id: event.run_id,
            event_id: event.event_id,
            source_id: event.source_id,
            source_seq: event.source_seq,
            timestamp_ns: event.timestamp_ns,
            tier: event.tier,
            payload: event.payload,
            payload_ref: event.payload_ref,
            synthesized: event.synthesized,
        }
    }

    /// Extract the import event, discarding the `commit_index`.
    pub fn into_import_event(self) -> ImportEvent {
        ImportEvent {
            run_id: self.run_id,
            event_id: self.event_id,
            source_id: self.source_id,
            source_seq: self.source_seq,
            timestamp_ns: self.timestamp_ns,
            tier: self.tier,
            payload: self.payload,
            payload_ref: self.payload_ref,
            synthesized: self.synthesized,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests (M1.4 serde audit + M1.5 round-trip byte stability)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // M1.1: Tier tests
    // -----------------------------------------------------------------------

    #[test]
    fn tier_display() {
        assert_eq!(Tier::A.to_string(), "A");
        assert_eq!(Tier::B.to_string(), "B");
        assert_eq!(Tier::C.to_string(), "C");
    }

    #[test]
    fn tier_from_str() {
        assert_eq!("A".parse::<Tier>().unwrap(), Tier::A);
        assert_eq!("b".parse::<Tier>().unwrap(), Tier::B);
        assert_eq!("C".parse::<Tier>().unwrap(), Tier::C);
        assert_eq!("a".parse::<Tier>().unwrap(), Tier::A);
        assert!("D".parse::<Tier>().is_err());
        assert!("".parse::<Tier>().is_err());
    }

    #[test]
    fn tier_serde_roundtrip() {
        for tier in [Tier::A, Tier::B, Tier::C] {
            let json = serde_json::to_string(&tier).unwrap();
            let expected = format!("\"{tier}\"");
            assert_eq!(json, expected, "Tier::{tier} should serialize as string");
            let back: Tier = serde_json::from_str(&json).unwrap();
            assert_eq!(back, tier);
            let json2 = serde_json::to_string(&back).unwrap();
            assert_eq!(json, json2, "Tier round-trip must be byte-stable");
        }
    }

    #[test]
    fn tier_ordering() {
        assert!(Tier::A > Tier::B);
        assert!(Tier::B > Tier::C);
        assert!(Tier::A > Tier::C);
    }

    #[test]
    fn tier_is_lossless() {
        assert!(Tier::A.is_lossless());
        assert!(!Tier::B.is_lossless());
        assert!(!Tier::C.is_lossless());
    }

    // -----------------------------------------------------------------------
    // M1.5: Round-trip byte stability tests
    // -----------------------------------------------------------------------

    /// Serialize -> deserialize -> re-serialize and assert byte equality.
    fn assert_roundtrip<T: Serialize + for<'de> Deserialize<'de>>(value: &T, label: &str) {
        let json1 = serde_json::to_string(value)
            .unwrap_or_else(|e| panic!("{label}: serialize failed: {e}"));
        let back: T = serde_json::from_str(&json1)
            .unwrap_or_else(|e| panic!("{label}: deserialize failed: {e}"));
        let json2 = serde_json::to_string(&back)
            .unwrap_or_else(|e| panic!("{label}: re-serialize failed: {e}"));
        assert_eq!(
            json1, json2,
            "{label}: round-trip byte stability failed\n  first:  {json1}\n  second: {json2}"
        );
    }

    /// Helper to build a minimal ImportEvent with a given payload.
    fn make_import_event(payload: EventPayload) -> ImportEvent {
        ImportEvent {
            run_id: "run-1".into(),
            event_id: "e-1".into(),
            source_id: "test".into(),
            source_seq: Some(0),
            timestamp_ns: 1_000_000_000,
            tier: Tier::A,
            payload,
            payload_ref: None,
            synthesized: false,
        }
    }

    #[test]
    fn roundtrip_run_start() {
        let event = make_import_event(EventPayload::RunStart {
            agent: "claude-3.5".into(),
            args: Some("--mode interactive".into()),
        });
        assert_roundtrip(&event, "ImportEvent::RunStart");
        let committed = CommittedEvent::commit(event, 0);
        assert_roundtrip(&committed, "CommittedEvent::RunStart");
    }

    #[test]
    fn roundtrip_run_end() {
        let event = make_import_event(EventPayload::RunEnd {
            exit_code: Some(0),
            reason: Some("completed".into()),
        });
        assert_roundtrip(&event, "ImportEvent::RunEnd");
        let committed = CommittedEvent::commit(event, 1);
        assert_roundtrip(&committed, "CommittedEvent::RunEnd");
    }

    #[test]
    fn roundtrip_tool_call() {
        let event = make_import_event(EventPayload::ToolCall {
            tool: "bash".into(),
            args: Some("ls -la".into()),
        });
        assert_roundtrip(&event, "ImportEvent::ToolCall");
        let committed = CommittedEvent::commit(event, 2);
        assert_roundtrip(&committed, "CommittedEvent::ToolCall");
    }

    #[test]
    fn roundtrip_tool_result() {
        let event = make_import_event(EventPayload::ToolResult {
            tool: "bash".into(),
            result: Some("total 42".into()),
            status: Some("success".into()),
        });
        assert_roundtrip(&event, "ImportEvent::ToolResult");
        let committed = CommittedEvent::commit(event, 3);
        assert_roundtrip(&committed, "CommittedEvent::ToolResult");
    }

    #[test]
    fn roundtrip_policy_decision() {
        let event = make_import_event(EventPayload::PolicyDecision {
            from_level: "L0".into(),
            to_level: "L1".into(),
            trigger: "queue_pressure_exceeded".into(),
            queue_pressure: 0.85,
        });
        assert_roundtrip(&event, "ImportEvent::PolicyDecision");
        let committed = CommittedEvent::commit(event, 4);
        assert_roundtrip(&committed, "CommittedEvent::PolicyDecision");
    }

    #[test]
    fn roundtrip_redaction_applied() {
        let event = make_import_event(EventPayload::RedactionApplied {
            target_event_id: "e-5".into(),
            field_path: "payload.args".into(),
            reason: "contains API key".into(),
        });
        assert_roundtrip(&event, "ImportEvent::RedactionApplied");
        let committed = CommittedEvent::commit(event, 5);
        assert_roundtrip(&committed, "CommittedEvent::RedactionApplied");
    }

    #[test]
    fn roundtrip_error() {
        let event = make_import_event(EventPayload::Error {
            kind: "io".into(),
            message: "disk full".into(),
            severity: Some("critical".into()),
        });
        assert_roundtrip(&event, "ImportEvent::Error");
        let committed = CommittedEvent::commit(event, 6);
        assert_roundtrip(&committed, "CommittedEvent::Error");
    }

    #[test]
    fn roundtrip_clock_skew_detected() {
        let event = make_import_event(EventPayload::ClockSkewDetected {
            expected_ns: 2_000_000_000,
            actual_ns: 1_900_000_000,
            delta_ns: 100_000_000,
        });
        assert_roundtrip(&event, "ImportEvent::ClockSkewDetected");
        let committed = CommittedEvent::commit(event, 7);
        assert_roundtrip(&committed, "CommittedEvent::ClockSkewDetected");
    }

    #[test]
    fn roundtrip_generic() {
        let mut data = BTreeMap::new();
        data.insert("key1".into(), "value1".into());
        data.insert("key2".into(), "value2".into());
        let event = ImportEvent {
            run_id: "run-1".into(),
            event_id: "e-gen".into(),
            source_id: "test".into(),
            source_seq: None,
            timestamp_ns: 1_000_000_000,
            tier: Tier::B,
            payload: EventPayload::Generic {
                event_type: "HeartBeat".into(),
                data,
            },
            payload_ref: None,
            synthesized: false,
        };
        assert_roundtrip(&event, "ImportEvent::Generic(Tier B)");
        let committed = CommittedEvent::commit(event, 8);
        assert_roundtrip(&committed, "CommittedEvent::Generic(Tier B)");
    }

    #[test]
    fn roundtrip_with_payload_ref() {
        let event = ImportEvent {
            run_id: "run-1".into(),
            event_id: "e-blob".into(),
            source_id: "test".into(),
            source_seq: Some(10),
            timestamp_ns: 1_000_000_000,
            tier: Tier::A,
            payload: EventPayload::ToolCall {
                tool: "read".into(),
                args: None, // blobbed; content is in blob store
            },
            payload_ref: Some(
                "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2".into(),
            ),
            synthesized: false,
        };
        assert_roundtrip(&event, "ImportEvent with payload_ref");
        let committed = CommittedEvent::commit(event, 9);
        assert_roundtrip(&committed, "CommittedEvent with payload_ref");
    }

    #[test]
    fn roundtrip_with_synthesized() {
        let event = ImportEvent {
            run_id: "run-1".into(),
            event_id: "e-synth".into(),
            source_id: "test".into(),
            source_seq: None,
            timestamp_ns: 1_000_000_000,
            tier: Tier::A,
            payload: EventPayload::RunStart {
                agent: "unknown".into(),
                args: None,
            },
            payload_ref: None,
            synthesized: true,
        };
        assert_roundtrip(&event, "ImportEvent with synthesized: true");
        let committed = CommittedEvent::commit(event, 10);
        assert_roundtrip(&committed, "CommittedEvent with synthesized: true");
    }

    #[test]
    fn roundtrip_source_seq_absent() {
        let event = ImportEvent {
            run_id: "run-1".into(),
            event_id: "e-noseq".into(),
            source_id: "test".into(),
            source_seq: None,
            timestamp_ns: 1_000_000_000,
            tier: Tier::A,
            payload: EventPayload::ToolCall {
                tool: "bash".into(),
                args: Some("echo hello".into()),
            },
            payload_ref: None,
            synthesized: false,
        };
        assert_roundtrip(&event, "ImportEvent with source_seq absent");
        // Verify source_seq is actually omitted from JSON
        let json = serde_json::to_string(&event).unwrap();
        assert!(
            !json.contains("source_seq"),
            "source_seq should be omitted when None"
        );
    }

    // -----------------------------------------------------------------------
    // M1.4: Serde audit tests
    // -----------------------------------------------------------------------

    #[test]
    fn committed_event_field_order() {
        let event = CommittedEvent::commit(
            make_import_event(EventPayload::RunStart {
                agent: "test".into(),
                args: None,
            }),
            42,
        );
        let json = serde_json::to_string(&event).unwrap();
        // Verify canonical field order: commit_index first, then metadata,
        // then payload, then optional trailing fields.
        let ci_pos = json.find("\"commit_index\"").expect("commit_index missing");
        let ri_pos = json.find("\"run_id\"").expect("run_id missing");
        let ei_pos = json.find("\"event_id\"").expect("event_id missing");
        let si_pos = json.find("\"source_id\"").expect("source_id missing");
        let ts_pos = json.find("\"timestamp_ns\"").expect("timestamp_ns missing");
        let ti_pos = json.find("\"tier\"").expect("tier missing");
        let pl_pos = json.find("\"payload\"").expect("payload missing");

        assert!(ci_pos < ri_pos, "commit_index before run_id");
        assert!(ri_pos < ei_pos, "run_id before event_id");
        assert!(ei_pos < si_pos, "event_id before source_id");
        assert!(si_pos < ts_pos, "source_id before timestamp_ns");
        assert!(ts_pos < ti_pos, "timestamp_ns before tier");
        assert!(ti_pos < pl_pos, "tier before payload");
    }

    #[test]
    fn import_event_field_order() {
        let event = make_import_event(EventPayload::RunStart {
            agent: "test".into(),
            args: None,
        });
        let json = serde_json::to_string(&event).unwrap();
        let ri_pos = json.find("\"run_id\"").expect("run_id missing");
        let ei_pos = json.find("\"event_id\"").expect("event_id missing");
        let si_pos = json.find("\"source_id\"").expect("source_id missing");
        let ts_pos = json.find("\"timestamp_ns\"").expect("timestamp_ns missing");
        let ti_pos = json.find("\"tier\"").expect("tier missing");
        let pl_pos = json.find("\"payload\"").expect("payload missing");

        assert!(ri_pos < ei_pos, "run_id before event_id");
        assert!(ei_pos < si_pos, "event_id before source_id");
        assert!(si_pos < ts_pos, "source_id before timestamp_ns");
        assert!(ts_pos < ti_pos, "timestamp_ns before tier");
        assert!(ti_pos < pl_pos, "tier before payload");
    }

    #[test]
    fn synthesized_false_omitted() {
        let event = make_import_event(EventPayload::RunStart {
            agent: "test".into(),
            args: None,
        });
        let json = serde_json::to_string(&event).unwrap();
        assert!(
            !json.contains("synthesized"),
            "synthesized:false should be omitted"
        );
    }

    #[test]
    fn synthesized_true_present() {
        let mut event = make_import_event(EventPayload::RunStart {
            agent: "test".into(),
            args: None,
        });
        event.synthesized = true;
        let json = serde_json::to_string(&event).unwrap();
        assert!(
            json.contains("\"synthesized\":true"),
            "synthesized:true should be present"
        );
    }

    #[test]
    fn generic_btreemap_sorted_keys() {
        let mut data = BTreeMap::new();
        data.insert("zebra".into(), "z".into());
        data.insert("alpha".into(), "a".into());
        data.insert("middle".into(), "m".into());
        let payload = EventPayload::Generic {
            event_type: "Test".into(),
            data,
        };
        let json = serde_json::to_string(&payload).unwrap();
        let alpha_pos = json.find("\"alpha\"").unwrap();
        let middle_pos = json.find("\"middle\"").unwrap();
        let zebra_pos = json.find("\"zebra\"").unwrap();
        assert!(
            alpha_pos < middle_pos && middle_pos < zebra_pos,
            "BTreeMap keys must be sorted: {json}"
        );
    }

    #[test]
    fn generic_empty_data_omitted() {
        let payload = EventPayload::Generic {
            event_type: "Ping".into(),
            data: BTreeMap::new(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(
            !json.contains("\"data\""),
            "empty data BTreeMap should be omitted"
        );
    }

    #[test]
    fn payload_type_field_present() {
        // Verify internally tagged serialization produces a "type" field.
        let payload = EventPayload::RunStart {
            agent: "test".into(),
            args: None,
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(
            json.contains("\"type\":\"RunStart\""),
            "payload should have type field: {json}"
        );
    }

    #[test]
    fn event_payload_type_names() {
        assert_eq!(
            EventPayload::RunStart {
                agent: String::new(),
                args: None
            }
            .event_type_name(),
            "RunStart"
        );
        assert_eq!(
            EventPayload::RunEnd {
                exit_code: None,
                reason: None
            }
            .event_type_name(),
            "RunEnd"
        );
        assert_eq!(
            EventPayload::ToolCall {
                tool: String::new(),
                args: None
            }
            .event_type_name(),
            "ToolCall"
        );
        assert_eq!(
            EventPayload::ToolResult {
                tool: String::new(),
                result: None,
                status: None
            }
            .event_type_name(),
            "ToolResult"
        );
        assert_eq!(
            EventPayload::PolicyDecision {
                from_level: String::new(),
                to_level: String::new(),
                trigger: String::new(),
                queue_pressure: 0.0
            }
            .event_type_name(),
            "PolicyDecision"
        );
        assert_eq!(
            EventPayload::RedactionApplied {
                target_event_id: String::new(),
                field_path: String::new(),
                reason: String::new()
            }
            .event_type_name(),
            "RedactionApplied"
        );
        assert_eq!(
            EventPayload::Error {
                kind: String::new(),
                message: String::new(),
                severity: None
            }
            .event_type_name(),
            "Error"
        );
        assert_eq!(
            EventPayload::ClockSkewDetected {
                expected_ns: 0,
                actual_ns: 0,
                delta_ns: 0
            }
            .event_type_name(),
            "ClockSkewDetected"
        );
        assert_eq!(
            EventPayload::Generic {
                event_type: "X".into(),
                data: BTreeMap::new()
            }
            .event_type_name(),
            "Generic"
        );
    }

    // -----------------------------------------------------------------------
    // Edge case tests
    // -----------------------------------------------------------------------

    #[test]
    fn roundtrip_empty_strings() {
        let event = make_import_event(EventPayload::ToolCall {
            tool: "".into(),
            args: Some("".into()),
        });
        assert_roundtrip(&event, "empty strings");
    }

    #[test]
    fn roundtrip_unicode() {
        let event = make_import_event(EventPayload::ToolCall {
            tool: "\u{5DE5}\u{5177}".into(),
            args: Some("args with \u{00e9}mojis and \u{00f1}".into()),
        });
        assert_roundtrip(&event, "unicode content");
    }

    #[test]
    fn roundtrip_u64_max() {
        let event = ImportEvent {
            run_id: "run-1".into(),
            event_id: "e-max".into(),
            source_id: "test".into(),
            source_seq: Some(u64::MAX),
            timestamp_ns: u64::MAX,
            tier: Tier::A,
            payload: EventPayload::RunStart {
                agent: "test".into(),
                args: None,
            },
            payload_ref: None,
            synthesized: false,
        };
        assert_roundtrip(&event, "u64::MAX values");
    }

    #[test]
    fn commit_preserves_all_fields() {
        let import = ImportEvent {
            run_id: "run-42".into(),
            event_id: "ev-99".into(),
            source_id: "cassette".into(),
            source_seq: Some(7),
            timestamp_ns: 999_999,
            tier: Tier::B,
            payload: EventPayload::Generic {
                event_type: "Custom".into(),
                data: BTreeMap::new(),
            },
            payload_ref: Some("deadbeef".into()),
            synthesized: true,
        };
        let committed = CommittedEvent::commit(import.clone(), 100);
        assert_eq!(committed.commit_index, 100);
        assert_eq!(committed.run_id, import.run_id);
        assert_eq!(committed.event_id, import.event_id);
        assert_eq!(committed.source_id, import.source_id);
        assert_eq!(committed.source_seq, import.source_seq);
        assert_eq!(committed.timestamp_ns, import.timestamp_ns);
        assert_eq!(committed.tier, import.tier);
        assert_eq!(committed.payload, import.payload);
        assert_eq!(committed.payload_ref, import.payload_ref);
        assert_eq!(committed.synthesized, import.synthesized);
    }

    #[test]
    fn into_import_event_drops_commit_index() {
        let import = make_import_event(EventPayload::RunStart {
            agent: "test".into(),
            args: None,
        });
        let committed = CommittedEvent::commit(import.clone(), 42);
        let back = committed.into_import_event();
        assert_eq!(back, import);
    }

    #[test]
    fn policy_decision_float_determinism() {
        // Verify that f64 queue_pressure round-trips deterministically.
        // serde_json uses Ryu for canonical shortest-representation output.
        let values = [0.0, 0.5, 0.8, 0.85, 1.0, 0.123456789];
        for qp in values {
            let event = make_import_event(EventPayload::PolicyDecision {
                from_level: "L0".into(),
                to_level: "L1".into(),
                trigger: "test".into(),
                queue_pressure: qp,
            });
            assert_roundtrip(&event, &format!("PolicyDecision qp={qp}"));
        }
    }

    #[test]
    fn committed_event_jsonl_no_pretty_print() {
        // Verify serialization is compact (no newlines, no indentation).
        let event = CommittedEvent::commit(
            make_import_event(EventPayload::RunStart {
                agent: "test".into(),
                args: None,
            }),
            0,
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(!json.contains('\n'), "JSONL must not contain newlines");
        assert!(!json.contains("  "), "JSONL must not be pretty-printed");
    }
}
