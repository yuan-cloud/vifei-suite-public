//! Constitution echo guard test.
//!
//! Prevents accidental copy-paste drift from the two constitutional docs
//! (`docs/CAPACITY_ENVELOPE.md` and `docs/BACKPRESSURE_POLICY.md`) into any
//! other markdown file in the repo.
//!
//! Guarded snippets are delimited by:
//!   `<!-- DOCS_GUARD:BEGIN ... -->`
//!   `<!-- DOCS_GUARD:END ... -->`
//!
//! The test fails if any non-constitutional `*.md` file contains a line that
//! is a character-exact match (after trimming leading/trailing whitespace) with
//! any line inside a guarded snippet.
//!
//! Blank lines and pure-formatting lines (`---`, `|---|---|`, etc.) are ignored.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Find the workspace root by walking up from the manifest dir.
fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // vifei-core lives at crates/vifei-core, workspace root is two levels up.
    manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("cannot find workspace root")
        .to_path_buf()
}

/// Recursively collect all `*.md` files under `dir`.
fn collect_md_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_md_files_inner(dir, &mut files);
    files
}

fn collect_md_files_inner(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip hidden dirs and target
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name.starts_with('.') || name == "target" {
                continue;
            }
            collect_md_files_inner(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

/// Returns true if `line` (after trimming) is blank or a pure-formatting line.
fn is_ignorable(trimmed: &str) -> bool {
    if trimmed.is_empty() {
        return true;
    }
    // Pure horizontal rules
    if trimmed.chars().all(|c| c == '-' || c == ' ') {
        return true;
    }
    // Table separator rows like |---|---|
    if trimmed.starts_with('|')
        && trimmed.ends_with('|')
        && trimmed
            .chars()
            .all(|c| c == '|' || c == '-' || c == ':' || c == ' ')
    {
        return true;
    }
    false
}

/// Extract guarded lines from a constitution doc.
fn extract_guarded_lines(content: &str) -> HashSet<String> {
    let mut guarded = HashSet::new();
    let mut in_guard = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<!-- DOCS_GUARD:BEGIN") {
            in_guard = true;
            continue;
        }
        if trimmed.starts_with("<!-- DOCS_GUARD:END") {
            in_guard = false;
            continue;
        }
        if in_guard {
            let t = trimmed.to_string();
            if !is_ignorable(&t) {
                guarded.insert(t);
            }
        }
    }
    guarded
}

#[test]
fn docs_guard_no_constitution_drift() {
    let root = workspace_root();

    let constitution_files: Vec<PathBuf> = vec![
        root.join("docs/CAPACITY_ENVELOPE.md"),
        root.join("docs/BACKPRESSURE_POLICY.md"),
    ];

    // Collect all guarded lines from constitution docs.
    let mut guarded_lines = HashSet::new();
    for path in &constitution_files {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        guarded_lines.extend(extract_guarded_lines(&content));
    }

    assert!(
        !guarded_lines.is_empty(),
        "no guarded lines found — constitution docs may be missing DOCS_GUARD markers"
    );

    // Collect all .md files in the repo.
    let all_md = collect_md_files(&root);

    // Canonical paths of constitution docs for comparison.
    let constitution_canonical: HashSet<PathBuf> = constitution_files
        .iter()
        .map(|p| p.canonicalize().unwrap_or_else(|_| p.clone()))
        .collect();

    let mut violations = Vec::new();

    for md_path in &all_md {
        let canonical = md_path.canonicalize().unwrap_or_else(|_| md_path.clone());
        if constitution_canonical.contains(&canonical) {
            continue;
        }

        let content = match std::fs::read_to_string(md_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim().to_string();
            if is_ignorable(&trimmed) {
                continue;
            }
            if guarded_lines.contains(&trimmed) {
                violations.push(format!(
                    "  {}:{} — {:?}",
                    md_path.strip_prefix(&root).unwrap_or(md_path).display(),
                    line_num + 1,
                    if trimmed.len() > 80 {
                        format!("{}...", &trimmed[..77])
                    } else {
                        trimmed
                    }
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Constitution echo guard failed!\n\
         The following lines in non-constitutional markdown files are character-exact\n\
         copies of guarded lines from the constitution docs.\n\
         Link to the relevant section instead of copying.\n\n{}\n",
        violations.join("\n")
    );
}
