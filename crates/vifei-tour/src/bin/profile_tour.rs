use std::path::PathBuf;
use std::time::Duration;
use vifei_tour::{TourConfig, TourStageProfile};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate parent")
        .parent()
        .expect("workspace root")
        .join("fixtures/large-stress.jsonl")
}

fn dur_ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn stage_pct(stage_ms: f64, total_ms: f64) -> f64 {
    if total_ms <= f64::EPSILON {
        return 0.0;
    }
    stage_ms / total_ms * 100.0
}

fn main() {
    let iters = std::env::var("VIFEI_TOUR_PROFILE_ITERS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(10);

    let fixture = fixture_path();
    let mut parse_ms = Vec::with_capacity(iters);
    let mut append_ms = Vec::with_capacity(iters);
    let mut reducer_ms = Vec::with_capacity(iters);
    let mut projection_ms = Vec::with_capacity(iters);
    let mut emit_ms = Vec::with_capacity(iters);
    let mut total_ms = Vec::with_capacity(iters);

    for _ in 0..iters {
        let tmp = tempfile::tempdir().expect("tempdir");
        let output_dir = tmp.path().join("tour-output");
        let config = TourConfig::new(fixture.clone()).with_output_dir(output_dir);
        let (_, profile): (_, TourStageProfile) =
            vifei_tour::run_tour_with_profile(&config).expect("tour run");

        parse_ms.push(dur_ms(profile.parse_fixture));
        append_ms.push(dur_ms(profile.append_writer));
        reducer_ms.push(dur_ms(profile.reducer));
        projection_ms.push(dur_ms(profile.projection));
        emit_ms.push(dur_ms(profile.metrics_emit));
        total_ms.push(dur_ms(profile.total));
    }

    total_ms.sort_by(f64::total_cmp);
    println!("tour_profile_iters={iters}");
    println!(
        "tour_profile_total_ms_p50={:.2}",
        percentile(&total_ms, 0.50)
    );
    println!(
        "tour_profile_total_ms_p95={:.2}",
        percentile(&total_ms, 0.95)
    );
    println!(
        "tour_profile_total_ms_p99={:.2}",
        percentile(&total_ms, 0.99)
    );

    let parse_mean = mean(&parse_ms);
    let append_mean = mean(&append_ms);
    let reducer_mean = mean(&reducer_ms);
    let projection_mean = mean(&projection_ms);
    let emit_mean = mean(&emit_ms);
    let total_mean = mean(&total_ms);

    println!(
        "hotspot_parse_fixture_pct={:.2}",
        stage_pct(parse_mean, total_mean)
    );
    println!(
        "hotspot_append_writer_pct={:.2}",
        stage_pct(append_mean, total_mean)
    );
    println!(
        "hotspot_reducer_pct={:.2}",
        stage_pct(reducer_mean, total_mean)
    );
    println!(
        "hotspot_projection_pct={:.2}",
        stage_pct(projection_mean, total_mean)
    );
    println!(
        "hotspot_metrics_emit_pct={:.2}",
        stage_pct(emit_mean, total_mean)
    );
}
