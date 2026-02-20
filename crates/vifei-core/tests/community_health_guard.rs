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
        "LICENSE",
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
    let readme = std::fs::read_to_string(&readme_path).expect("cannot read README.md");

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

#[test]
fn issue_template_contact_links_target_canonical_repo() {
    let root = workspace_root();
    let config_path = root.join(".github/ISSUE_TEMPLATE/config.yml");
    let config = std::fs::read_to_string(&config_path).expect("cannot read issue template config");

    let required_urls = [
        "https://github.com/yuan-cloud/vifei-suite/security/advisories/new",
        "https://github.com/yuan-cloud/vifei-suite/blob/main/SUPPORT.md",
    ];

    for url in required_urls {
        assert!(
            config.contains(url),
            "issue template config must include canonical URL: {}",
            url
        );
    }
}

#[test]
fn readme_badges_target_canonical_repo() {
    let root = workspace_root();
    let readme_path = root.join("README.md");
    let readme = std::fs::read_to_string(&readme_path).expect("cannot read README.md");

    let required_fragments = [
        "https://github.com/yuan-cloud/vifei-suite/actions/workflows/ci.yml/badge.svg",
        "https://github.com/yuan-cloud/vifei-suite/actions/workflows/ci.yml",
    ];

    for fragment in required_fragments {
        assert!(
            readme.contains(fragment),
            "README.md must include canonical badge/release fragment: {}",
            fragment
        );
    }
}

#[test]
fn key_public_doc_local_links_resolve_to_existing_paths() {
    let root = workspace_root();
    let checks: &[(&str, &[&str])] = &[
        (
            "README.md",
            &[
                "CONTRIBUTING.md",
                "SECURITY.md",
                "SUPPORT.md",
                "docs/COMMUNITY_TRIAGE_PLAYBOOK.md",
                "docs/PUBLIC_REPO_SETTINGS_CHECKLIST.md",
                "docs/CAPACITY_ENVELOPE.md",
                "docs/BACKPRESSURE_POLICY.md",
                "docs/assets/readme/incident-lens.txt",
                "docs/assets/readme/forensic-lens.txt",
                "docs/assets/readme/truth-hud-degraded.txt",
                "docs/assets/readme/export-refusal.txt",
                "docs/assets/readme/architecture.mmd",
            ],
        ),
        (
            "CONTRIBUTING.md",
            &["SECURITY.md", ".github/pull_request_template.md"],
        ),
        (
            "SUPPORT.md",
            &["SECURITY.md", "docs/COMMUNITY_TRIAGE_PLAYBOOK.md"],
        ),
        ("SECURITY.md", &[]),
    ];

    for (doc_rel, required_paths) in checks {
        let doc_path = root.join(doc_rel);
        assert!(
            doc_path.is_file(),
            "required public doc is missing: {doc_rel}"
        );
        let doc_text = std::fs::read_to_string(&doc_path).expect("cannot read public doc");

        for rel in *required_paths {
            assert!(
                doc_text.contains(rel),
                "{doc_rel} must reference required local path: {rel}"
            );
            let target = root.join(rel);
            assert!(
                target.exists(),
                "{doc_rel} references missing local path: {rel}"
            );
        }
    }
}
