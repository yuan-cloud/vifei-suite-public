use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use vifei_tour::TourConfig;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchArtifact {
    schema_version: String,
    stats: BenchmarkStats,
    command: CommandProvenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrendKey {
    benchmark: String,
    git_sha: Option<String>,
    target_os: String,
    target_arch: String,
    package_version: String,
    fixture_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrendRecord {
    schema_version: String,
    metric_schema_version: String,
    key: TrendKey,
    stats: BenchmarkStats,
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
    std::env::var("VIFEI_TOUR_BENCH_ITERS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(10)
}

fn trend_base_dir() -> PathBuf {
    std::env::var("VIFEI_PERF_TREND_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".tmp/perf/trends"))
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
    validate_bench_artifact(artifact)?;
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

fn validate_bench_artifact(artifact: &BenchArtifact) -> Result<(), String> {
    if artifact.schema_version != "vifei-tour-bench-v1" {
        return Err(format!(
            "unexpected bench schema_version: {}",
            artifact.schema_version
        ));
    }
    if artifact.stats.iters == 0 {
        return Err("bench stats.iters must be greater than zero".to_string());
    }
    if artifact.command.fixture_path.trim().is_empty() {
        return Err("command.fixture_path must be non-empty".to_string());
    }
    Ok(())
}

fn validate_trend_record(record: &TrendRecord) -> Result<(), String> {
    if record.schema_version != "vifei-perf-trend-v1" {
        return Err(format!(
            "unexpected trend schema_version: {}",
            record.schema_version
        ));
    }
    if record.metric_schema_version != "vifei-tour-bench-v1" {
        return Err(format!(
            "unexpected metric_schema_version: {}",
            record.metric_schema_version
        ));
    }
    if record.key.benchmark != "bench_tour" {
        return Err(format!(
            "unexpected benchmark key: {}",
            record.key.benchmark
        ));
    }
    if record.stats.iters == 0 {
        return Err("trend stats.iters must be greater than zero".to_string());
    }
    Ok(())
}

fn trend_log_path(base: &std::path::Path, command: &CommandProvenance) -> PathBuf {
    base.join("bench_tour").join(format!(
        "{}-{}.jsonl",
        command.target_os, command.target_arch
    ))
}

fn write_trend_record(path: &PathBuf, record: &TrendRecord) -> Result<(), String> {
    validate_trend_record(record)?;
    let parent = path
        .parent()
        .ok_or_else(|| format!("trend path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("failed to create trend dir {}: {e}", parent.display()))?;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("failed to open trend log {}: {e}", path.display()))?;

    let line = serde_json::to_string(record)
        .map_err(|e| format!("failed to serialize trend record: {e}"))?;
    file.write_all(line.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|e| format!("failed to append trend record {}: {e}", path.display()))?;
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
        let result = vifei_tour::run_tour(&config)
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
        invoked_as: std::env::var("VIFEI_TOUR_BENCH_CMD").ok(),
        fixture_path: fixture.display().to_string(),
        fixture_bytes,
        fixture_line_count,
        package_version: env!("CARGO_PKG_VERSION").to_string(),
        target_os: std::env::consts::OS.to_string(),
        target_arch: std::env::consts::ARCH.to_string(),
    };
    let artifact = BenchArtifact {
        schema_version: "vifei-tour-bench-v1".to_string(),
        stats: stats.clone(),
        command: command.clone(),
    };

    let artifact_path = std::env::var("VIFEI_TOUR_BENCH_ARTIFACT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".tmp/perf/bench_tour_metrics.json"));
    write_artifact(&artifact_path, &artifact)?;
    println!("tour_bench_artifact={}", artifact_path.display());

    let trend_record = TrendRecord {
        schema_version: "vifei-perf-trend-v1".to_string(),
        metric_schema_version: artifact.schema_version.clone(),
        key: TrendKey {
            benchmark: "bench_tour".to_string(),
            git_sha: std::env::var("VIFEI_GIT_SHA").ok(),
            target_os: command.target_os.clone(),
            target_arch: command.target_arch.clone(),
            package_version: command.package_version.clone(),
            fixture_path: command.fixture_path.clone(),
        },
        stats: artifact.stats.clone(),
    };
    let trend_path = trend_log_path(&trend_base_dir(), &command);
    write_trend_record(&trend_path, &trend_record)?;
    println!("tour_perf_trend_log={}", trend_path.display());
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

    #[test]
    fn bench_artifact_validator_rejects_zero_iters() {
        let artifact = BenchArtifact {
            schema_version: "vifei-tour-bench-v1".to_string(),
            stats: BenchmarkStats {
                iters: 0,
                run_ms_p50: 1.0,
                run_ms_p95: 1.0,
                run_ms_p99: 1.0,
                run_ms_mean: 1.0,
                throughput_events_per_sec_p50: 10.0,
                throughput_events_per_sec_p95: 10.0,
                throughput_events_per_sec_p99: 10.0,
                peak_rss_kib: None,
            },
            command: CommandProvenance {
                argv: vec!["bench_tour".to_string()],
                invoked_as: None,
                fixture_path: "fixtures/large-stress.jsonl".to_string(),
                fixture_bytes: 1,
                fixture_line_count: 1,
                package_version: "0.1.0".to_string(),
                target_os: "linux".to_string(),
                target_arch: "x86_64".to_string(),
            },
        };
        let err = validate_bench_artifact(&artifact).expect_err("zero iters must be rejected");
        assert!(err.contains("iters"));
    }

    #[test]
    fn trend_record_roundtrip_schema_and_key() {
        let stats = BenchmarkStats {
            iters: 3,
            run_ms_p50: 10.0,
            run_ms_p95: 20.0,
            run_ms_p99: 30.0,
            run_ms_mean: 15.0,
            throughput_events_per_sec_p50: 100.0,
            throughput_events_per_sec_p95: 90.0,
            throughput_events_per_sec_p99: 80.0,
            peak_rss_kib: Some(1234),
        };
        let record = TrendRecord {
            schema_version: "vifei-perf-trend-v1".to_string(),
            metric_schema_version: "vifei-tour-bench-v1".to_string(),
            key: TrendKey {
                benchmark: "bench_tour".to_string(),
                git_sha: Some("abc123".to_string()),
                target_os: "linux".to_string(),
                target_arch: "x86_64".to_string(),
                package_version: "0.1.0".to_string(),
                fixture_path: "fixtures/large-stress.jsonl".to_string(),
            },
            stats,
        };
        validate_trend_record(&record).expect("trend record should validate");
        let json = serde_json::to_string(&record).expect("serialize trend record");
        let decoded: TrendRecord = serde_json::from_str(&json).expect("deserialize trend record");
        assert_eq!(decoded.schema_version, "vifei-perf-trend-v1");
        assert_eq!(decoded.metric_schema_version, "vifei-tour-bench-v1");
        assert_eq!(decoded.key.benchmark, "bench_tour");
    }
}
