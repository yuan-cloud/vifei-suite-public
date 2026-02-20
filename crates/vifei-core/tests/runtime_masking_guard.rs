use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct Finding {
    path: PathBuf,
    line: usize,
    message: String,
    guidance: &'static str,
}

const WINDOW_LINES: usize = 5;
const DEFAULT_LITERAL_HINTS: &[&str] = &[
    "b\"{}\".to_vec()",
    "\"{}\".to_string()",
    "String::new()",
    "Vec::new()",
    "json!({})",
    "json!([])",
];

// Escape hatch for intentionally-reviewed exceptions.
// Tuple is (workspace-relative path suffix, line number).
const ALLOWLIST: &[(&str, usize)] = &[];

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("cannot find workspace root")
        .to_path_buf()
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn scan_source(path: &Path, source: &str) -> Vec<Finding> {
    let lines: Vec<&str> = source.lines().collect();
    let mut findings = Vec::new();

    for idx in 0..lines.len() {
        if !lines[idx].contains("serde_json::to_") {
            continue;
        }
        let window_end = usize::min(idx + WINDOW_LINES, lines.len());
        let window = lines[idx..window_end].join("\n");

        if window.contains("unwrap_or_default()") {
            findings.push(Finding {
                path: path.to_path_buf(),
                line: idx + 1,
                message: "serde serialization followed by unwrap_or_default()".to_string(),
                guidance: "Replace with explicit error handling or explicit non-masking sentinel encoding.",
            });
            continue;
        }

        if window.contains("unwrap_or_else(")
            && DEFAULT_LITERAL_HINTS
                .iter()
                .any(|hint| window.contains(hint))
        {
            findings.push(Finding {
                path: path.to_path_buf(),
                line: idx + 1,
                message: "serde serialization fallback defaults to placeholder literal".to_string(),
                guidance: "Return a structured runtime error instead of writing placeholder artifact content.",
            });
        }
    }

    findings
}

fn is_allowlisted(root: &Path, finding: &Finding) -> bool {
    let rel = finding.path.strip_prefix(root).unwrap_or(&finding.path);
    let rel_str = rel.to_string_lossy();
    ALLOWLIST
        .iter()
        .any(|(suffix, line)| rel_str.ends_with(suffix) && *line == finding.line)
}

#[test]
fn runtime_sources_have_no_silent_serde_fallbacks() {
    let root = workspace_root();
    let runtime_roots = [
        root.join("crates/vifei-core/src"),
        root.join("crates/vifei-tui/src"),
    ];

    let mut files = Vec::new();
    for dir in runtime_roots {
        collect_rs_files(&dir, &mut files);
    }

    let mut violations = Vec::new();
    for file in files {
        let body = match std::fs::read_to_string(&file) {
            Ok(body) => body,
            Err(_) => continue,
        };
        for finding in scan_source(&file, &body) {
            if !is_allowlisted(&root, &finding) {
                let rel = finding.path.strip_prefix(&root).unwrap_or(&finding.path);
                violations.push(format!(
                    "{}:{} - {}\n  Fix: {}",
                    rel.display(),
                    finding.line,
                    finding.message,
                    finding.guidance
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Runtime masking guard failed. Remove silent serde fallbacks in runtime paths.\n{}",
        violations.join("\n")
    );
}

#[test]
fn runtime_masking_guard_catches_placeholder_reintroduction() {
    let synthetic = r#"
let _bytes = serde_json::to_vec_pretty(&manifest)
    .unwrap_or_else(|_| b"{}".to_vec());
"#;

    let findings = scan_source(Path::new("synthetic.rs"), synthetic);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("placeholder"));
    assert!(findings[0].guidance.contains("structured runtime error"));
}
