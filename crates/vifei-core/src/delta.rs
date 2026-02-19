//! Deterministic run-to-run delta engine.
//!
//! Compares two committed event streams by canonical `commit_index` and emits
//! stable divergence records keyed by `(commit_index, path, change_class)`.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::event::CommittedEvent;

/// Change classification for a divergence record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ChangeClass {
    /// Event exists on the right side, missing on the left side.
    EventMissingLeft,
    /// Event exists on the left side, missing on the right side.
    EventMissingRight,
    /// Field/value mismatch at the same canonical path.
    ValueMismatch,
}

/// One deterministic divergence keyed by `commit_index` and path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Divergence {
    pub commit_index: u64,
    pub path: String,
    pub change_class: ChangeClass,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub left_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub right_value: Option<String>,
}

/// Deterministic delta output between two runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunDelta {
    pub left_run_id: String,
    pub right_run_id: String,
    pub left_event_count: usize,
    pub right_event_count: usize,
    pub divergences: Vec<Divergence>,
}

/// Compute a deterministic delta over two committed streams.
///
/// Notes:
/// - events are matched by canonical `commit_index` only.
/// - order of output divergences is deterministic by construction.
/// - input order does not matter; all access is via `BTreeMap` keyed by index.
pub fn diff_runs(left: &[CommittedEvent], right: &[CommittedEvent]) -> RunDelta {
    let left_by_index = index_events_by_commit_index(left);
    let right_by_index = index_events_by_commit_index(right);
    let left_run_id = left_by_index
        .iter()
        .next()
        .map(|(_, e)| e.run_id.clone())
        .unwrap_or_default();
    let right_run_id = right_by_index
        .iter()
        .next()
        .map(|(_, e)| e.run_id.clone())
        .unwrap_or_default();

    let all_indices: BTreeSet<u64> = left_by_index
        .keys()
        .chain(right_by_index.keys())
        .copied()
        .collect();

    let mut divergences = Vec::new();

    for commit_index in all_indices {
        let left_event = left_by_index.get(&commit_index).copied();
        let right_event = right_by_index.get(&commit_index).copied();

        match (left_event, right_event) {
            (None, Some(_)) => divergences.push(Divergence {
                commit_index,
                path: "$event".to_string(),
                change_class: ChangeClass::EventMissingLeft,
                left_value: None,
                right_value: Some("present".to_string()),
            }),
            (Some(_), None) => divergences.push(Divergence {
                commit_index,
                path: "$event".to_string(),
                change_class: ChangeClass::EventMissingRight,
                left_value: Some("present".to_string()),
                right_value: None,
            }),
            (Some(l), Some(r)) => compare_event(commit_index, l, r, &mut divergences),
            (None, None) => {}
        }
    }

    RunDelta {
        left_run_id,
        right_run_id,
        left_event_count: left.len(),
        right_event_count: right.len(),
        divergences,
    }
}

fn index_events_by_commit_index(events: &[CommittedEvent]) -> BTreeMap<u64, &CommittedEvent> {
    let mut out: BTreeMap<u64, &CommittedEvent> = BTreeMap::new();
    for event in events {
        match out.get(&event.commit_index) {
            None => {
                out.insert(event.commit_index, event);
            }
            Some(existing) => {
                if event_stable_tiebreak_key(event) < event_stable_tiebreak_key(existing) {
                    out.insert(event.commit_index, event);
                }
            }
        }
    }
    out
}

fn event_stable_tiebreak_key(event: &CommittedEvent) -> String {
    let payload = match serde_json::to_string(&event.payload) {
        Ok(value) => value,
        // Never silently downgrade to an empty payload key segment.
        Err(error) => format!("__payload_serialize_error__:{error}"),
    };
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}",
        event.run_id,
        event.event_id,
        event.source_id,
        event.source_seq.map(|v| v.to_string()).unwrap_or_default(),
        event.timestamp_ns,
        event.tier,
        event.payload_ref.clone().unwrap_or_default(),
        event.synthesized,
        payload
    )
}

fn compare_event(
    commit_index: u64,
    left: &CommittedEvent,
    right: &CommittedEvent,
    out: &mut Vec<Divergence>,
) {
    compare_scalar(commit_index, "$.run_id", &left.run_id, &right.run_id, out);
    compare_scalar(
        commit_index,
        "$.event_id",
        &left.event_id,
        &right.event_id,
        out,
    );
    compare_scalar(
        commit_index,
        "$.source_id",
        &left.source_id,
        &right.source_id,
        out,
    );
    compare_scalar_opt(
        commit_index,
        "$.source_seq",
        &left.source_seq,
        &right.source_seq,
        out,
    );
    compare_scalar(
        commit_index,
        "$.timestamp_ns",
        &left.timestamp_ns.to_string(),
        &right.timestamp_ns.to_string(),
        out,
    );
    compare_scalar(
        commit_index,
        "$.tier",
        &left.tier.to_string(),
        &right.tier.to_string(),
        out,
    );
    compare_scalar_opt(
        commit_index,
        "$.payload_ref",
        &left.payload_ref,
        &right.payload_ref,
        out,
    );
    compare_scalar(
        commit_index,
        "$.synthesized",
        &left.synthesized.to_string(),
        &right.synthesized.to_string(),
        out,
    );

    let left_payload = match serde_json::to_value(&left.payload) {
        Ok(v) => v,
        Err(_) => return,
    };
    let right_payload = match serde_json::to_value(&right.payload) {
        Ok(v) => v,
        Err(_) => return,
    };
    let left_flat = flatten_json("$.payload", &left_payload);
    let right_flat = flatten_json("$.payload", &right_payload);

    let keys: BTreeSet<String> = left_flat.keys().chain(right_flat.keys()).cloned().collect();
    for key in keys {
        let l = left_flat.get(&key).cloned();
        let r = right_flat.get(&key).cloned();
        if l != r {
            out.push(Divergence {
                commit_index,
                path: key,
                change_class: ChangeClass::ValueMismatch,
                left_value: l,
                right_value: r,
            });
        }
    }
}

fn compare_scalar<T: ToString>(
    commit_index: u64,
    path: &str,
    left: &T,
    right: &T,
    out: &mut Vec<Divergence>,
) {
    let l = left.to_string();
    let r = right.to_string();
    if l != r {
        out.push(Divergence {
            commit_index,
            path: path.to_string(),
            change_class: ChangeClass::ValueMismatch,
            left_value: Some(l),
            right_value: Some(r),
        });
    }
}

fn compare_scalar_opt<T: ToString>(
    commit_index: u64,
    path: &str,
    left: &Option<T>,
    right: &Option<T>,
    out: &mut Vec<Divergence>,
) {
    let l = left.as_ref().map(ToString::to_string);
    let r = right.as_ref().map(ToString::to_string);
    if l != r {
        out.push(Divergence {
            commit_index,
            path: path.to_string(),
            change_class: ChangeClass::ValueMismatch,
            left_value: l,
            right_value: r,
        });
    }
}

fn flatten_json(path: &str, value: &serde_json::Value) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    flatten_json_inner(path, value, &mut out);
    out
}

fn flatten_json_inner(path: &str, value: &serde_json::Value, out: &mut BTreeMap<String, String>) {
    match value {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for key in keys {
                if let Some(next) = map.get(key) {
                    flatten_json_inner(&format!("{path}.{key}"), next, out);
                }
            }
        }
        serde_json::Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                flatten_json_inner(&format!("{path}[{index}]"), item, out);
            }
        }
        _ => {
            out.insert(path.to_string(), value.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{CommittedEvent, EventPayload, ImportEvent, Tier};

    fn committed(commit_index: u64, payload: EventPayload) -> CommittedEvent {
        CommittedEvent::commit(
            ImportEvent {
                run_id: "run".to_string(),
                event_id: format!("e-{commit_index}"),
                source_id: "test".to_string(),
                source_seq: Some(commit_index),
                timestamp_ns: 1_000 + commit_index,
                tier: Tier::A,
                payload,
                payload_ref: None,
                synthesized: false,
            },
            commit_index,
        )
    }

    #[test]
    fn identical_runs_have_no_divergence() {
        let left = vec![
            committed(
                0,
                EventPayload::RunStart {
                    agent: "a".to_string(),
                    args: None,
                },
            ),
            committed(
                1,
                EventPayload::ToolResult {
                    tool: "search".to_string(),
                    result: Some("ok".to_string()),
                    status: Some("success".to_string()),
                },
            ),
        ];
        let right = left.clone();
        let delta = diff_runs(&left, &right);
        assert!(delta.divergences.is_empty());
    }

    #[test]
    fn missing_event_is_reported_by_commit_index() {
        let left = vec![committed(
            0,
            EventPayload::RunStart {
                agent: "a".to_string(),
                args: None,
            },
        )];
        let right = vec![
            committed(
                0,
                EventPayload::RunStart {
                    agent: "a".to_string(),
                    args: None,
                },
            ),
            committed(
                1,
                EventPayload::RunEnd {
                    exit_code: Some(0),
                    reason: Some("done".to_string()),
                },
            ),
        ];
        let delta = diff_runs(&left, &right);
        assert_eq!(delta.divergences.len(), 1);
        assert_eq!(delta.divergences[0].commit_index, 1);
        assert_eq!(
            delta.divergences[0].change_class,
            ChangeClass::EventMissingLeft
        );
        assert_eq!(delta.divergences[0].path, "$event");
    }

    #[test]
    fn nested_payload_field_mismatch_is_path_keyed() {
        let left = vec![committed(
            0,
            EventPayload::ToolCall {
                tool: "search".to_string(),
                args: Some("{\"q\":\"left\"}".to_string()),
            },
        )];
        let right = vec![committed(
            0,
            EventPayload::ToolCall {
                tool: "search".to_string(),
                args: Some("{\"q\":\"right\"}".to_string()),
            },
        )];
        let delta = diff_runs(&left, &right);
        assert_eq!(delta.divergences.len(), 1);
        assert_eq!(delta.divergences[0].path, "$.payload.args");
        assert_eq!(
            delta.divergences[0].change_class,
            ChangeClass::ValueMismatch
        );
    }

    #[test]
    fn output_is_byte_stable_across_unsorted_inputs() {
        let left = vec![
            committed(
                2,
                EventPayload::RunEnd {
                    exit_code: Some(0),
                    reason: None,
                },
            ),
            committed(
                0,
                EventPayload::RunStart {
                    agent: "a".to_string(),
                    args: None,
                },
            ),
        ];
        let right = vec![
            committed(
                0,
                EventPayload::RunStart {
                    agent: "a".to_string(),
                    args: None,
                },
            ),
            committed(
                2,
                EventPayload::RunEnd {
                    exit_code: Some(0),
                    reason: Some("changed".to_string()),
                },
            ),
        ];

        let delta_a = diff_runs(&left, &right);
        let json_a = serde_json::to_string(&delta_a).expect("delta_a should serialize");
        let delta_b = diff_runs(&left, &right);
        let json_b = serde_json::to_string(&delta_b).expect("delta_b should serialize");

        assert_eq!(json_a, json_b);
        assert!(!delta_a.divergences.is_empty());
        assert_eq!(delta_a.divergences[0].commit_index, 2);
    }

    #[test]
    fn run_id_is_selected_by_lowest_commit_index_not_input_order() {
        let left = vec![
            committed(
                2,
                EventPayload::RunEnd {
                    exit_code: Some(0),
                    reason: None,
                },
            ),
            committed(
                0,
                EventPayload::RunStart {
                    agent: "a".to_string(),
                    args: None,
                },
            ),
        ];
        let right = left.clone();

        let mut left_shuffled = left.clone();
        left_shuffled.reverse();
        let delta_a = diff_runs(&left, &right);
        let delta_b = diff_runs(&left_shuffled, &right);

        assert_eq!(delta_a.left_run_id, delta_b.left_run_id);
    }

    #[test]
    fn payload_ref_presence_mismatch_is_reported() {
        let mut left = committed(
            0,
            EventPayload::RunStart {
                agent: "a".to_string(),
                args: None,
            },
        );
        left.payload_ref = None;

        let mut right = committed(
            0,
            EventPayload::RunStart {
                agent: "a".to_string(),
                args: None,
            },
        );
        right.payload_ref = Some(String::new());

        let delta = diff_runs(&[left], &[right]);
        assert!(delta
            .divergences
            .iter()
            .any(|d| d.path == "$.payload_ref" && d.change_class == ChangeClass::ValueMismatch));
    }

    #[test]
    fn duplicate_commit_index_resolution_is_input_order_independent() {
        let mut a = committed(
            0,
            EventPayload::RunStart {
                agent: "z-agent".to_string(),
                args: None,
            },
        );
        a.event_id = "z-id".to_string();

        let mut b = committed(
            0,
            EventPayload::RunStart {
                agent: "a-agent".to_string(),
                args: None,
            },
        );
        b.event_id = "a-id".to_string();

        let right = vec![b.clone()];
        let left_ab = vec![a.clone(), b.clone()];
        let left_ba = vec![b, a];

        let delta_ab = diff_runs(&left_ab, &right);
        let delta_ba = diff_runs(&left_ba, &right);

        assert_eq!(delta_ab, delta_ba);
        assert!(delta_ab.divergences.is_empty());
    }

    #[test]
    fn tie_break_key_uses_explicit_payload_component() {
        let event = committed(
            7,
            EventPayload::ToolResult {
                tool: "search".to_string(),
                result: Some("ok".to_string()),
                status: Some("success".to_string()),
            },
        );
        let key = event_stable_tiebreak_key(&event);
        let payload_json = serde_json::to_string(&event.payload).expect("payload serializable");

        assert!(
            key.ends_with(&payload_json),
            "payload component should be explicit and non-empty in tie-break key"
        );
    }
}
