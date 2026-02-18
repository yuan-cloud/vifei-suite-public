use panopticon_tour::TourConfig;
use serde::Serialize;
use std::fs;
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
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

#[derive(Debug, Clone, Serialize)]
struct BenchmarkStats {
    iters: usize,
    run_ms_p50: f64,
    run_ms_p95: f64,
    run_ms_p99: f64,
    run_ms_mean: f64,
    throughput_events_per_sec_p50: f64,
    throughput_events_per_sec_p95: f64,
    throughput_events_per_sec_p99: f64,
    peak_rss_kib: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
struct CommandProvenance {
    argv: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    invoked_as: Option<String>,
    fixture_path: String,
    fixture_bytes: u64,
    fixture_line_count: u64,
    package_version: String,
    target_os: String,
    target_arch: String,
}

#[derive(Debug, Clone, Serialize)]
struct BenchArtifact {
    schema_version: String,
    stats: BenchmarkStats,
    command: CommandProvenance,
}

fn mean(samples: &[Duration]) -> Duration {
    if samples.is_empty() {
        return Duration::ZERO;
    }
    let total = samples
        .iter()
        .fold(Duration::ZERO, |acc, value| acc.saturating_add(*value));
    total / samples.len() as u32
}

fn throughput_eps(run_ms: f64, event_count: usize) -> f64 {
    if run_ms <= f64::EPSILON {
        return 0.0;
    }
    (event_count as f64) / (run_ms / 1000.0)
}

fn parse_iters() -> usize {
    std::env::var("PANOPTICON_TOUR_BENCH_ITERS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(10)
}

#[cfg(target_os = "linux")]
fn read_current_rss_kib() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let value = rest.split_whitespace().next()?;
            return value.parse::<u64>().ok();
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn read_current_rss_kib() -> Option<u64> {
    None
}

fn write_artifact(path: &PathBuf, artifact: &BenchArtifact) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("artifact path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("failed to create artifact dir {}: {e}", parent.display()))?;
    let payload = serde_json::to_vec_pretty(artifact)
        .map_err(|e| format!("failed to serialize bench artifact: {e}"))?;
    fs::write(path, payload)
        .map_err(|e| format!("failed to write bench artifact {}: {e}", path.display()))?;
    Ok(())
}

fn main() -> Result<(), String> {
    let iters = parse_iters();
    let fixture = fixture_path();
    let fixture_bytes = fs::metadata(&fixture)
        .map_err(|e| format!("failed to stat fixture {}: {e}", fixture.display()))?
        .len();
    let fixture_text = fs::read_to_string(&fixture)
        .map_err(|e| format!("failed to read fixture {}: {e}", fixture.display()))?;
    let fixture_line_count = fixture_text.lines().count() as u64;

    let mut samples = Vec::with_capacity(iters);
    let mut peak_rss_kib: Option<u64> = None;
    let mut event_count = None;

    for _ in 0..iters {
        let tmp = tempfile::tempdir().map_err(|e| format!("failed to create tempdir: {e}"))?;
        let output_dir = tmp.path().join("tour-output");
        let config = TourConfig::new(fixture.clone()).with_output_dir(output_dir);
        let start = Instant::now();
        let result = panopticon_tour::run_tour(&config)
            .map_err(|e| format!("tour run failed for fixture {}: {e}", fixture.display()))?;
        event_count.get_or_insert(result.metrics.event_count_total);
        std::hint::black_box(&result.viewmodel_hash);
        samples.push(start.elapsed());
        if let Some(rss) = read_current_rss_kib() {
            peak_rss_kib = Some(peak_rss_kib.map_or(rss, |prev| prev.max(rss)));
        }
    }

    samples.sort_unstable();
    let p50 = percentile(&samples, 0.50);
    let p95 = percentile(&samples, 0.95);
    let p99 = percentile(&samples, 0.99);
    let avg = mean(&samples);
    let events = event_count.unwrap_or(0);
    let p50_ms = p50.as_secs_f64() * 1000.0;
    let p95_ms = p95.as_secs_f64() * 1000.0;
    let p99_ms = p99.as_secs_f64() * 1000.0;
    let avg_ms = avg.as_secs_f64() * 1000.0;

    println!("tour_bench_iters={iters}");
    println!("tour_run_ms_p50={p50_ms:.2}");
    println!("tour_run_ms_p95={p95_ms:.2}");
    println!("tour_run_ms_p99={p99_ms:.2}");
    println!("tour_run_ms_mean={avg_ms:.2}");
    if let Some(peak) = peak_rss_kib {
        println!("tour_peak_rss_kib={peak}");
    }

    let stats = BenchmarkStats {
        iters,
        run_ms_p50: p50_ms,
        run_ms_p95: p95_ms,
        run_ms_p99: p99_ms,
        run_ms_mean: avg_ms,
        throughput_events_per_sec_p50: throughput_eps(p50_ms, events),
        throughput_events_per_sec_p95: throughput_eps(p95_ms, events),
        throughput_events_per_sec_p99: throughput_eps(p99_ms, events),
        peak_rss_kib,
    };
    let command = CommandProvenance {
        argv: std::env::args().collect(),
        invoked_as: std::env::var("PANOPTICON_TOUR_BENCH_CMD").ok(),
        fixture_path: fixture.display().to_string(),
        fixture_bytes,
        fixture_line_count,
        package_version: env!("CARGO_PKG_VERSION").to_string(),
        target_os: std::env::consts::OS.to_string(),
        target_arch: std::env::consts::ARCH.to_string(),
    };
    let artifact = BenchArtifact {
        schema_version: "panopticon-tour-bench-v1".to_string(),
        stats,
        command,
    };

    let artifact_path = std::env::var("PANOPTICON_TOUR_BENCH_ARTIFACT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".tmp/perf/bench_tour_metrics.json"));
    write_artifact(&artifact_path, &artifact)?;
    println!("tour_bench_artifact={}", artifact_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_empty_is_zero() {
        assert_eq!(percentile(&[], 0.95), Duration::ZERO);
    }

    #[test]
    fn throughput_handles_zero() {
        assert_eq!(throughput_eps(0.0, 1000), 0.0);
    }
}
