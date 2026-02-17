# README Verification Log (`bd-x7q.4`)

Date: 2026-02-17
Workspace: `/data/projects/PanopticonAliveca2.5`

## Goal

Validate README commands and trust-challenge steps against the current codebase.

## Command Validation Matrix

1. `cargo run -p panopticon-tui --bin panopticon -- --help`
- Result: pass
- Evidence: `/tmp/panopticon_readme_verify/help.txt`

2. `cargo run -p panopticon-tui --bin panopticon -- tour fixtures/large-stress.jsonl --stress --output-dir /tmp/.../tour-a`
- Result: pass
- Evidence: `/tmp/panopticon_readme_verify/tour-a.txt`

3. `cargo run -p panopticon-tui --bin panopticon -- tour fixtures/large-stress.jsonl --stress --output-dir /tmp/.../tour-b`
- Result: pass
- Evidence: `/tmp/panopticon_readme_verify/tour-b.txt`

4. Tour determinism hash compare
- Result: pass
- Evidence: `/tmp/panopticon_readme_verify/hash-compare.txt`
- `hash_a`: `000573091386a86cabe6935bbe997897a83f42cf89595238e55c2f9c8d45eda6`
- `hash_b`: `000573091386a86cabe6935bbe997897a83f42cf89595238e55c2f9c8d45eda6`

5. `cargo run -p panopticon-tui --bin panopticon -- export docs/assets/readme/sample-export-clean-eventlog.jsonl --share-safe --output /tmp/.../bundle.tar.zst --refusal-report /tmp/.../refusal-report.json`
- Result: pass
- Evidence: `/tmp/panopticon_readme_verify/export-success.txt`

6. `cargo run -p panopticon-tui --bin panopticon -- export docs/assets/readme/sample-refusal-eventlog.jsonl --share-safe --output /tmp/.../refusal-bundle.tar.zst --refusal-report /tmp/.../refusal-report-refused.json`
- Result: expected refusal (pass)
- Evidence: `/tmp/panopticon_readme_verify/export-refused.txt`

7. `cargo run -p panopticon-tui --bin panopticon -- view docs/assets/readme/sample-eventlog.jsonl`
- Result: requires interactive TTY
- Non-TTY runner output: `Error: Permission denied (os error 13)`
- Evidence: `/tmp/panopticon_readme_verify/view-smoke.txt`

## Trust Challenge Validation

1. Tier A drops check
- Result: pass
- Value: `tier_a_drops=0`
- Evidence: `/tmp/panopticon_readme_verify/metrics-summary.txt`

2. Deterministic replay hash check
- Result: pass
- Evidence: `/tmp/panopticon_readme_verify/hash-compare.txt`

## Notes

- During verification, the previous README export example using `sample-eventlog.jsonl` produced false-positive refusal due conservative scanner matches on large numeric fields.
- README now points to `docs/assets/readme/sample-export-clean-eventlog.jsonl` for the successful export example.
