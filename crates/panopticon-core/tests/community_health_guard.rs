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
        "https://github.com/yuan-cloud/panopticon-suite/security/advisories/new",
        "https://github.com/yuan-cloud/panopticon-suite/blob/main/SUPPORT.md",
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
        "https://img.shields.io/github/actions/workflow/status/yuan-cloud/panopticon-suite/ci.yml",
        "https://github.com/yuan-cloud/panopticon-suite/actions/workflows/ci.yml",
        "https://img.shields.io/github/v/tag/yuan-cloud/panopticon-suite",
        "https://github.com/yuan-cloud/panopticon-suite/releases",
    ];

    for fragment in required_fragments {
        assert!(
            readme.contains(fragment),
            "README.md must include canonical badge/release fragment: {}",
            fragment
        );
    }
}
