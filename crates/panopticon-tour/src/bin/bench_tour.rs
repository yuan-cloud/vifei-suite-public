use panopticon_tour::TourConfig;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate parent")
        .parent()
        .expect("workspace root")
        .join("fixtures/large-stress.jsonl")
}

fn percentile(sorted: &[Duration], p: f64) -> Duration {
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

fn main() {
    let iters = std::env::var("PANOPTICON_TOUR_BENCH_ITERS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(10);

    let fixture = fixture_path();
    let mut samples = Vec::with_capacity(iters);

    for _ in 0..iters {
        let tmp = tempfile::tempdir().expect("tempdir");
        let output_dir = tmp.path().join("tour-output");
        let config = TourConfig::new(fixture.clone()).with_output_dir(output_dir);
        let start = Instant::now();
        let result = panopticon_tour::run_tour(&config).expect("tour run");
        std::hint::black_box(&result.viewmodel_hash);
        samples.push(start.elapsed());
    }

    samples.sort_unstable();
    let p50 = percentile(&samples, 0.50);
    let p95 = percentile(&samples, 0.95);
    let p99 = percentile(&samples, 0.99);

    println!("tour_bench_iters={iters}");
    println!("tour_run_ms_p50={:.2}", p50.as_secs_f64() * 1000.0);
    println!("tour_run_ms_p95={:.2}", p95.as_secs_f64() * 1000.0);
    println!("tour_run_ms_p99={:.2}", p99.as_secs_f64() * 1000.0);
}
