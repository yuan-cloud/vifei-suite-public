# Dependency and Provenance Audit (v0.1)

Date: 2026-02-19

## Scope

- Confirm whether Ratatui code was imported directly.
- Inspect Rust dependency license signals from cached crate metadata.
- Verify workspace/crate license metadata consistency.

## Findings

1. Ratatui usage mode:
- No Ratatui git submodule or vendored source was found.
- TUI implementation depends on `ratatui` crate (`crates/vifei-tui/Cargo.toml`), not a Ratatui crate.
- Ratatui appears in docs as design/reference inspiration only.

2. Provenance indicators in repository:
- No explicit "copied from <repo>" or upstream code attribution headers were found in source files.
- This is not a cryptographic/code-similarity proof; it is a repository-text audit.

3. Dependency license distribution (cached crates from `Cargo.lock`):
- Most cached dependencies are permissive (`MIT`, `MIT OR Apache-2.0`, or equivalent permissive dual forms).
- Additional permissive entries observed include `BSD-2-Clause`, `Zlib`, and `BSL-1.0` in transitive deps.
- Some platform-specific/transitive crates were not present in local cache, so their license fields could not be read offline in this environment.

4. Workspace manifest consistency:
- Workspace root already declares `license = "MIT"`.
- All workspace crates now inherit it via `license.workspace = true`:
  - `crates/vifei-core/Cargo.toml`
  - `crates/vifei-import/Cargo.toml`
  - `crates/vifei-export/Cargo.toml`
  - `crates/vifei-tui/Cargo.toml`
  - `crates/vifei-tour/Cargo.toml`

## Limits of this audit

- Network-restricted environment prevented live download-based tools (`cargo-deny`, `cargo-license`, ScanCode/FOSSA fetch workflows) from running.
- This audit is technical due diligence, not legal advice.
- A definitive legal/compliance sign-off requires a full bill-of-materials scan in a network-enabled CI or legal review environment.

## Recommended next compliance step

When network access is available, run one of:

```bash
cargo deny check licenses
```

or

```bash
cargo install cargo-license
cargo license --json
```

and archive the resulting report as release evidence.
