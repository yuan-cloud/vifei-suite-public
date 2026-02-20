# UX Evidence Asset Refresh · 2026-02-17

Bead: `bd-gxd.10`

## Commands Run
```bash
scripts/refresh_readme_assets.sh
find docs/assets/readme -maxdepth 1 -type f -print0 | sort -z | xargs -0 sha256sum > .tmp/readme-assets-sha256-pass1.txt

scripts/refresh_readme_assets.sh
find docs/assets/readme -maxdepth 1 -type f -print0 | sort -z | xargs -0 sha256sum > .tmp/readme-assets-sha256-pass2.txt

grep -v 'refusal-report.json' .tmp/readme-assets-sha256-pass1.txt > .tmp/readme-assets-sha256-pass1-norm.txt
grep -v 'refusal-report.json' .tmp/readme-assets-sha256-pass2.txt > .tmp/readme-assets-sha256-pass2-norm.txt
diff -u .tmp/readme-assets-sha256-pass1-norm.txt .tmp/readme-assets-sha256-pass2-norm.txt
```

## Result Summary
- Asset refresh completed successfully.
- Narrow terminal evidence artifact added: `docs/assets/readme/incident-lens-narrow-72.txt`.
- Determinism check passed for stable asset set (normalized check).

## Determinism Notes
- `refusal-report.json` is excluded from strict byte-equality check.
- Reason: report payload includes run-time metadata from export refusal generation, which can vary per run.
- Stable/readme-critical captures are deterministic after normalization:
  - `incident-lens.txt`
  - `incident-lens-narrow-72.txt`
  - `forensic-lens.txt`
  - `truth-hud-degraded.txt`
  - `export-refusal.txt`
  - `artifacts-view.txt`
  - `architecture.mmd`
  - `sample-eventlog.jsonl`
  - `sample-export-clean-eventlog.jsonl`
  - `sample-refusal-eventlog.jsonl`

## Linked Validation Context
- Modality validation report: `docs/testing/ux-modality-validation-2026-02-17.md`
- UX baseline report: `docs/testing/ux-baseline-2026-02-17.md`
