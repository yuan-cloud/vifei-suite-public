//! Shared adapter contract for external provider ingestion.
//!
//! Goals:
//! - Keep canonical ordering ownership in append writer only.
//! - Provide deterministic normalization helpers for adapter implementations.
//! - Expose stable contract/version constants for tests and docs.

use vifei_core::event::{EventPayload, Tier};

/// Version for adapter-level normalization contract semantics.
pub const ADAPTER_CONTRACT_VERSION: &str = "adapter-contract-v1";

/// Current Agent Cassette record schema version.
pub const AGENT_CASSETTE_SCHEMA_VERSION: &str = "agent-cassette-v1";

/// Current OpenAI Responses record schema version.
pub const OPENAI_RESPONSES_SCHEMA_VERSION: &str = "openai-responses-v1";

/// Current Anthropic messages/tool-use record schema version.
pub const ANTHROPIC_MESSAGES_SCHEMA_VERSION: &str = "anthropic-messages-v1";

/// Current Cohere Translate record schema version.
pub const COHERE_TRANSLATE_SCHEMA_VERSION: &str = "cohere-translate-v1";

/// Deterministically normalize run identity.
///
/// Returns `(run_id, synthesized)` where synthesized is true when fallback is used.
pub fn normalize_run_id(raw: Option<&str>, fallback: &str) -> (String, bool) {
    match raw {
        Some(value) if !value.trim().is_empty() => (value.to_string(), false),
        _ => (fallback.to_string(), true),
    }
}

/// Deterministically normalize event identity.
///
/// Returns `(event_id, synthesized)` where synthesized is true when fallback is used.
pub fn normalize_event_id(raw: Option<&str>, fallback: &str) -> (String, bool) {
    match raw {
        Some(value) if !value.trim().is_empty() => (value.to_string(), false),
        _ => (fallback.to_string(), true),
    }
}

/// Validate optional source schema version against adapter expectation.
///
/// Missing version is accepted in v0.1 for compatibility with legacy fixtures.
pub fn validate_schema_version(source_value: Option<&str>, expected: &str) -> Result<(), String> {
    match source_value {
        None => Ok(()),
        Some(value) if value == expected => Ok(()),
        Some(value) => Err(format!(
            "schema_version mismatch: expected {expected}, got {value}"
        )),
    }
}

/// Reject source-provided commit index; canonical ordering is append-writer-owned.
pub fn reject_source_commit_index(commit_index: Option<u64>) -> Result<(), String> {
    match commit_index {
        None => Ok(()),
        Some(value) => Err(format!(
            "source provided forbidden commit_index={value}; canonical commit_index is append-writer-assigned"
        )),
    }
}

/// Build a Tier A contract error payload.
pub fn contract_error_payload(message: String) -> (EventPayload, Tier) {
    (
        EventPayload::Error {
            kind: "contract".to_string(),
            message,
            severity: Some("error".to_string()),
        },
        Tier::A,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_constants_are_stable() {
        assert_eq!(ADAPTER_CONTRACT_VERSION, "adapter-contract-v1");
        assert_eq!(AGENT_CASSETTE_SCHEMA_VERSION, "agent-cassette-v1");
        assert_eq!(OPENAI_RESPONSES_SCHEMA_VERSION, "openai-responses-v1");
        assert_eq!(ANTHROPIC_MESSAGES_SCHEMA_VERSION, "anthropic-messages-v1");
        assert_eq!(COHERE_TRANSLATE_SCHEMA_VERSION, "cohere-translate-v1");
    }

    #[test]
    fn normalize_helpers_mark_synthesized_when_missing() {
        let (run_id, run_syn) = normalize_run_id(None, "unknown-session");
        let (event_id, event_syn) = normalize_event_id(None, "adapter:0");
        assert_eq!(run_id, "unknown-session");
        assert!(run_syn);
        assert_eq!(event_id, "adapter:0");
        assert!(event_syn);
    }

    #[test]
    fn reject_source_commit_index_is_strict() {
        assert!(reject_source_commit_index(None).is_ok());
        assert!(reject_source_commit_index(Some(42)).is_err());
    }
}
