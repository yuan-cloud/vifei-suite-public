use std::path::PathBuf;
use vifei_tour::TourConfig;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate parent")
        .parent()
        .expect("workspace root")
        .join("fixtures/large-stress.jsonl")
}

fn run_tour_once() -> Result<(String, usize), String> {
    let fixture = fixture_path();
    let tmp = tempfile::tempdir().map_err(|e| format!("failed to create tempdir: {e}"))?;
    let output_dir = tmp.path().join("tour-output");
    let config = TourConfig::new(fixture.clone()).with_output_dir(output_dir);
    let result = vifei_tour::run_tour(&config)
        .map_err(|e| format!("tour run failed for fixture {}: {e}", fixture.display()))?;
    Ok((result.viewmodel_hash, result.metrics.event_count_total))
}

#[test]
fn tour_pipeline_benchmark_lane_smoke() {
    let (viewmodel_hash, event_count) = run_tour_once().expect("tour smoke lane should succeed");
    assert!(
        !viewmodel_hash.is_empty(),
        "viewmodel hash must be populated"
    );
    assert!(event_count > 0, "fixture should contain at least one event");
}
