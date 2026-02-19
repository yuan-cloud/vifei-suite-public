//! CI-oriented invariant assertions for Tour artifacts (M7.4).

use serde::Deserialize;
use std::fs;
use std::io::{BufReader, Cursor};
use std::path::PathBuf;
use std::sync::OnceLock;
use vifei_core::eventlog::{read_eventlog, EventLogWriter};
use vifei_core::reducer::{reduce, State};
use vifei_import::cassette::parse_cassette;
use vifei_tour::{DegradationTransition, TimeTravelCapture, TourConfig};

type TransitionTuple = (String, String, String, u64);

#[derive(Debug, Deserialize)]
struct ArtifactMetrics {
    event_count_total: usize,
    queue_pressure: f64,
    projection_invariants_version: String,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace path")
        .parent()
        .expect("workspace path")
        .join("fixtures/large-stress.jsonl")
}

fn expected_policy_transitions() -> &'static Vec<TransitionTuple> {
    static EXPECTED: OnceLock<Vec<TransitionTuple>> = OnceLock::new();
    EXPECTED.get_or_init(|| {
        let fixture_content = fs::read_to_string(fixture_path()).expect("fixture");
        let parsed = parse_cassette(BufReader::new(Cursor::new(fixture_content)));

        let temp_dir = tempfile::tempdir().expect("tempdir");
        let eventlog_path = temp_dir.path().join("eventlog.jsonl");
        let mut writer = EventLogWriter::open(&eventlog_path).expect("open writer");
        for event in parsed {
            writer.append(event).expect("append");
        }
        drop(writer);

        let committed = read_eventlog(&eventlog_path).expect("read eventlog");
        let mut state = State::new();
        for event in &committed {
            state = reduce(&state, event);
        }

        state
            .policy_decisions
            .iter()
            .map(|pd| {
                (
                    pd.from_level.clone(),
                    pd.to_level.clone(),
                    pd.trigger.clone(),
                    pd.queue_pressure_micro,
                )
            })
            .collect()
    })
}

fn level_rank(level: &str) -> Option<u8> {
    match level {
        "L0" => Some(0),
        "L1" => Some(1),
        "L2" => Some(2),
        "L3" => Some(3),
        "L4" => Some(4),
        "L5" => Some(5),
        _ => None,
    }
}

fn transitions_respect_ladder_order(transitions: &[DegradationTransition]) -> bool {
    transitions.iter().all(|t| {
        let Some(from) = level_rank(&t.from_level) else {
            return false;
        };
        let Some(to) = level_rank(&t.to_level) else {
            return false;
        };

        // Any level may transition to L5 on fatal posture.
        if to == 5 {
            return true;
        }

        // Otherwise require single-step movement in either direction.
        from.abs_diff(to) <= 1
    })
}

#[test]
fn tier_a_drops_is_zero_in_stress_run() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output_dir = dir.path().join("out");
    let config = TourConfig::new(fixture_path()).with_output_dir(&output_dir);

    let result = vifei_tour::run_tour(&config).expect("tour run");
    assert_eq!(
        result.metrics.tier_a_drops, 0,
        "Tier A drops must remain zero under stress"
    );
}

#[test]
fn viewmodel_hash_stable_on_rerun() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out1 = dir.path().join("out1");
    let out2 = dir.path().join("out2");

    let r1 = vifei_tour::run_tour(&TourConfig::new(fixture_path()).with_output_dir(&out1))
        .expect("tour run 1");
    let r2 = vifei_tour::run_tour(&TourConfig::new(fixture_path()).with_output_dir(&out2))
        .expect("tour run 2");

    assert_eq!(
        r1.viewmodel_hash, r2.viewmodel_hash,
        "viewmodel.hash must be stable across reruns"
    );

    let h1 = fs::read_to_string(out1.join("viewmodel.hash")).expect("hash file 1");
    let h2 = fs::read_to_string(out2.join("viewmodel.hash")).expect("hash file 2");
    assert_eq!(h1, h2, "viewmodel.hash files must be byte-identical");
}

#[test]
fn degradation_transitions_respect_ladder_order() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output_dir = dir.path().join("out");
    let config = TourConfig::new(fixture_path()).with_output_dir(&output_dir);

    let result = vifei_tour::run_tour(&config).expect("tour run");
    assert!(
        transitions_respect_ladder_order(&result.metrics.degradation_transitions),
        "degradation_transitions violates ladder-order constraints"
    );
}

#[test]
fn degradation_transition_checker_is_non_tautological() {
    let ok = vec![
        DegradationTransition {
            from_level: "L0".to_string(),
            to_level: "L1".to_string(),
            trigger: "pressure".to_string(),
            queue_pressure: 0.70,
        },
        DegradationTransition {
            from_level: "L1".to_string(),
            to_level: "L5".to_string(),
            trigger: "fatal".to_string(),
            queue_pressure: 1.0,
        },
    ];
    assert!(transitions_respect_ladder_order(&ok));

    let bad = vec![DegradationTransition {
        from_level: "L0".to_string(),
        to_level: "L3".to_string(),
        trigger: "skip".to_string(),
        queue_pressure: 0.80,
    }];
    assert!(!transitions_respect_ladder_order(&bad));
}

#[test]
fn metrics_transitions_are_derivable_from_policy_decisions() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output_dir = dir.path().join("out");
    let config = TourConfig::new(fixture_path()).with_output_dir(&output_dir);
    let result = vifei_tour::run_tour(&config).expect("tour run");

    // Reuse cached expected transitions derived from fixture replay to avoid
    // redoing parse+append+replay in each assertion path.
    let expected: Vec<TransitionTuple> = expected_policy_transitions().clone();

    let actual: Vec<TransitionTuple> = result
        .metrics
        .degradation_transitions
        .iter()
        .map(|t| {
            (
                t.from_level.clone(),
                t.to_level.clone(),
                t.trigger.clone(),
                (t.queue_pressure * 1_000_000.0).round() as u64,
            )
        })
        .collect();

    assert_eq!(
        expected, actual,
        "metrics.degradation_transitions must be derivable from PolicyDecision events"
    );

    // Explicit correspondence check from metrics transitions to the expected set.
    for tr in &result.metrics.degradation_transitions {
        let tr_qp = (tr.queue_pressure * 1_000_000.0).round() as u64;
        let has_match = expected
            .iter()
            .any(|(from_level, to_level, trigger, qp_micro)| {
                from_level == &tr.from_level
                    && to_level == &tr.to_level
                    && trigger == &tr.trigger
                    && *qp_micro == tr_qp
            });
        assert!(
            has_match,
            "Transition {:?} has no matching PolicyDecision event",
            (&tr.from_level, &tr.to_level, &tr.trigger, tr_qp)
        );
    }
}

#[test]
fn tour_artifacts_are_cross_field_consistent_on_large_fixture() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output_dir = dir.path().join("out");
    let config = TourConfig::new(fixture_path()).with_output_dir(&output_dir);
    let result = vifei_tour::run_tour(&config).expect("tour run");

    let metrics: ArtifactMetrics = serde_json::from_str(
        &fs::read_to_string(output_dir.join("metrics.json")).expect("metrics"),
    )
    .expect("parse metrics");
    let timetravel: TimeTravelCapture = serde_json::from_str(
        &fs::read_to_string(output_dir.join("timetravel.capture")).expect("timetravel"),
    )
    .expect("parse timetravel");
    let ansi = fs::read_to_string(output_dir.join("ansi.capture")).expect("ansi");
    let hash_file = fs::read_to_string(output_dir.join("viewmodel.hash")).expect("hash");
    let hash_trimmed = hash_file.trim();

    assert!(
        !timetravel.seek_points.is_empty(),
        "seek_points must be non-empty"
    );
    let last = timetravel.seek_points.last().expect("last seek point");

    assert_eq!(
        metrics.event_count_total as u64 - 1,
        last.commit_index,
        "last seek commit_index must align with event_count_total - 1"
    );
    assert_eq!(
        timetravel.projection_invariants_version, metrics.projection_invariants_version,
        "metrics and timetravel must report the same projection invariants version"
    );
    assert_eq!(
        hash_trimmed, result.viewmodel_hash,
        "viewmodel.hash file and TourResult hash must match"
    );
    assert_eq!(
        last.viewmodel_hash, result.viewmodel_hash,
        "last seek point must reflect final viewmodel hash"
    );
    assert!(
        ansi.contains(hash_trimmed),
        "ansi.capture must include the final viewmodel hash"
    );
    assert!(
        (0.0..=1.0).contains(&metrics.queue_pressure),
        "queue_pressure must stay in [0.0, 1.0]"
    );
}
