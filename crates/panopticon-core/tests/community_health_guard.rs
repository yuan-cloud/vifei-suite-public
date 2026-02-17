//! Community-health presence guard.
//!
//! Prevents accidental removal of required repository health surfaces.

use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("cannot find workspace root")
        .to_path_buf()
}

#[test]
fn required_community_health_files_exist() {
    let root = workspace_root();
    let required = [
        "CONTRIBUTING.md",
        "SECURITY.md",
        "SUPPORT.md",
        ".github/ISSUE_TEMPLATE/config.yml",
        ".github/ISSUE_TEMPLATE/bug_report.yml",
        ".github/ISSUE_TEMPLATE/determinism_regression.yml",
        ".github/pull_request_template.md",
        "docs/PUBLIC_REPO_SETTINGS_CHECKLIST.md",
    ];

    for rel in required {
        let path = root.join(rel);
        assert!(
            path.is_file(),
            "required community-health file is missing: {}",
            rel
        );
    }
}

#[test]
fn readme_links_to_community_and_settings_docs() {
    let root = workspace_root();
    let readme_path = root.join("README.md");
    let readme = std::fs::read_to_string(&readme_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", readme_path.display()));

    let required_links = [
        "CONTRIBUTING.md",
        "SECURITY.md",
        "SUPPORT.md",
        "docs/PUBLIC_REPO_SETTINGS_CHECKLIST.md",
    ];

    for link in required_links {
        assert!(
            readme.contains(link),
            "README.md must reference community health doc: {}",
            link
        );
    }
}
