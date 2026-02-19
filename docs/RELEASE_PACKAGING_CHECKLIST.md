# Release Packaging Matrix and Publish Checklist (v0.1)

This document is the operational checklist for `bd-3qq.3`.

It defines release channels, required artifacts, and go/no-go checks before publication.

## Channel Matrix

| Channel | v0.1 Scope | Required | Publish Trigger | Verification |
|---|---|---|---|---|
| GitHub Release | Binary distribution (`vifei`, `bench_tour`) + checksums + provenance | Yes | Tag `v*` push | `scripts/verify_release_artifacts.sh` + attestation verification |
| crates.io (`vifei-core`) | Library crate publish | Yes | Manual after `cargo publish --dry-run` | `cargo package --allow-dirty -p vifei-core` |
| crates.io (`vifei-import`) | Library crate publish | Yes | Manual after `cargo publish --dry-run` | `cargo package --allow-dirty -p vifei-import` |
| crates.io (`vifei-export`) | Library crate publish | Yes | Manual after `cargo publish --dry-run` | `cargo package --allow-dirty -p vifei-export` |
| crates.io (`vifei-tour`) | Library crate publish | Yes | Manual after `cargo publish --dry-run` | `cargo package --allow-dirty -p vifei-tour` |
| crates.io (`vifei-tui`) | Binary crate publish | Optional for v0.1 | Manual decision | `cargo package --allow-dirty -p vifei-tui` |
| Homebrew tap | Convenience installer for binaries | Optional | Post v0.1 baseline release | Install smoke on clean host |
| winget | Windows package distribution | Optional | Post v0.1 baseline release | Install smoke on Windows runner |

## Release Artifact Contract

`dist/` must contain:

- `vifei`
- `bench_tour`
- `sha256sums.txt`

Generated via:

```bash
scripts/release_artifacts.sh dist
scripts/verify_release_artifacts.sh dist
```

Launch/demo media outputs should include a provenance manifest:

```bash
scripts/demo_quickcheck.sh /tmp/vifei_demo_run
cargo run -p vifei-tour --bin media_provenance -- \
  --verify /tmp/vifei_demo_run/media-provenance.json \
  --base-dir /tmp/vifei_demo_run
```

And pass media hygiene scanning:

```bash
scripts/testing/check_media_hygiene.sh /tmp/vifei_demo_run
```

Bundle proof assets, transcript, and replay notes:

```bash
scripts/demo/package_launch_bundle.sh .tmp/launch-media-bundle
```

Confirm launch bundle includes:
- `trust-cut/TRUST_DEMO_SUMMARY.txt`
- `visual-cut/VISUAL_SHOWCASE_SUMMARY.txt`
- `COMMAND_ASSET_MAP.md`

False-positive handling and safe override:
- Add precise regex entries to `scripts/testing/media_hygiene_allowlist.txt` only for known synthetic content.
- Keep entries narrow (pattern + file context), with a comment explaining why.
- `VIFEI_HYGIENE_ALLOW_UNSAFE=1` is emergency-only and requires explicit release approval.

Trust verification details are in `docs/RELEASE_TRUST_VERIFICATION.md`.

## Go/No-Go Checklist

All items below must pass for GO:

1. Quality gates pass:
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`

2. Release artifact checks pass:
- `scripts/release_artifacts.sh dist`
- `scripts/verify_release_artifacts.sh dist`

3. README/trust verification is current:
- `docs/README_VERIFICATION.md` reflects current command behavior
- README command samples are executable as written

4. Naming and migration communication is current:
- Any user-facing install/run snippets use `vifei` crate/bin/repo names.
- Canonical repo URLs in badges/templates target the active GitHub slug.

5. Release-trust CI run is green for target commit/tag:
- `fastlane` job success (PR default smoke lane)
- `full-confidence` job success (merge/release confidence lane)
- defer register validation passes (`docs/testing/defer-register-v0.1.json`)
- `release-trust` job success
- attestation step success

6. No open P1/P2 blocker beads for release track:
- `bd-3qq.1` complete
- `bd-3qq.2` complete
- `bd-3qq.3` complete
- `bd-3qq.4` complete

If any item fails, NO-GO.

## crates.io Preflight Checklist

For each crate selected for publish:

1. Package locally:

```bash
cargo package --allow-dirty -p <crate-name>
```

2. Dry-run publish:

```bash
cargo publish --dry-run -p <crate-name>
```

3. Confirm version and changelog alignment.

4. Publish in dependency order if needed:

- `vifei-core`
- `vifei-import`
- `vifei-export`
- `vifei-tour`
- `vifei-tui` (if publishing)

## Rollback Notes

If release verification fails after tag creation:

1. Do not publish to crates.io.
2. Do not mark GitHub Release as final.
3. Open a fix bead with findings-first description.
4. Patch and re-run full checklist.
5. Re-tag with next patch version (`vX.Y.(Z+1)`), do not reuse broken tag.

If attestation tooling fails but checksums pass:

- Follow rollback order from `docs/RELEASE_TRUST_VERIFICATION.md`.
- Preserve checksum verification path; do not bypass checksum checks.
