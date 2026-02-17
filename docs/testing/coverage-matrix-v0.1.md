# Coverage Matrix v0.1

## Scope

Baseline coverage audit for:

- `panopticon-core`
- `panopticon-import`
- `panopticon-export`
- `panopticon-tour`
- `panopticon-tui`

Test classes tracked:

- unit
- integration
- e2e/pty (where available)
- docs guard

## Repro Commands

```bash
# Coverage tool availability check
cargo llvm-cov --version

# Workspace-wide test inventory list
cargo test --workspace --all-targets -- --list > /tmp/panopticon-test-list.txt

# Per-crate inventory snapshots
cargo test -p panopticon-core --all-targets -- --list > /tmp/core-tests.txt
cargo test -p panopticon-import --all-targets -- --list > /tmp/import-tests.txt
cargo test -p panopticon-export --all-targets -- --list > /tmp/export-tests.txt
cargo test -p panopticon-tour --all-targets -- --list > /tmp/tour-tests.txt
cargo test -p panopticon-tui --all-targets -- --list > /tmp/tui-tests.txt
```

Observed during this audit:

- `cargo llvm-cov` is not installed in this environment (`error: no such command: llvm-cov`).
- Fallback method used: explicit test inventory + risk-ranked uncovered-path mapping.

## Baseline Snapshot

| Crate | Source surfaces (`src/`) | Listed tests (from `--list`) | Integration/E2E signals |
|---|---:|---:|---|
| `panopticon-core` | 6 files | 157 (`156` unit + `1` docs guard) | No PTY e2e; strong invariant/property-style unit focus |
| `panopticon-import` | 2 files | 24 (`21` unit + `3` integration) | Fixture-backed integration exists |
| `panopticon-export` | 2 files | 49 (`37` unit + `12` integration) | Refusal-path integration exists |
| `panopticon-tour` | 3 files (+ bench targets) | 29 (`15` unit + `14` integration-like) | Stress fixtures + invariants present; no interactive TUI e2e |
| `panopticon-tui` | 7 files | 60 (`56` unit + `4` integration snapshot) | No PTY harness yet (`bd-2yv.4`) |

Workspace inventory file lines:

- `/tmp/panopticon-test-list.txt`: `347` lines (includes target headers and summaries)

## Surface Matrix

| Surface | Current coverage evidence | Uncovered/weak path | Risk | Owner bead |
|---|---|---|---|---|
| `core/event.rs` schema + serde | Extensive roundtrip/order tests | No mutation/fuzz lane for hostile JSONL inputs | P1 | `bd-2yv.3` |
| `core/eventlog.rs` append writer + skew checks | Monotonic/index/skew tests present | Large malformed-stream recovery scenarios limited | P1 | `bd-2yv.3` |
| `core/reducer.rs` deterministic replay | Determinism and checkpoint suites present | No long-run memory/regression envelope assertions | P2 | `bd-2yv.3` |
| `core/projection.rs` hash/invariant checks | Strong hash stability tests | No adversarial high-cardinality stress matrix | P2 | `bd-2yv.3` |
| `import/cassette.rs` mapping/parser | Unit + fixture integration | Broader malformed cassette corpus not yet codified | P1 | `bd-2yv.3` |
| `export/lib.rs` + `scanner.rs` | Refusal + clean path integration | No dedicated CI lane artifact retention for export failures | P1 | `bd-2yv.2`, `bd-2yv.6` |
| `tour/lib.rs` + stress fixtures | Determinism and ladder invariants tested | No explicit fastlane subset contract documented/enforced | P1 | `bd-2yv.8`, `bd-2yv.6` |
| `tui/lib.rs` app keyflow | Unit tests for tab/nav/quit/hud | No real PTY interaction assertions | P0 | `bd-2yv.4` |
| `tui/incident_lens.rs` + `forensic_lens.rs` | Rich unit render tests | Width-bucket truncation guarantees not automated end-to-end | P1 | `bd-gxd.9`, `bd-2yv.5` |
| `tui/src/bin/capture_readme_assets.rs` | Built in test target; no tests (`0`) | No direct snapshot/assertion contract for generated assets | P1 | `bd-gxd.10`, `bd-2yv.3` |
| CLI command path (`panopticon` bin) | Limited unit tests in `main.rs` | No end-to-end command transcript suite | P0 | `bd-2yv.2` |
| CI test governance | Current checks run manually in this audit | No enforced fastlane/full-confidence split and waiver gate | P0 | `bd-2yv.6`, `bd-2yv.7`, `bd-2yv.8` |

## No-Mock/Fake Assessment

- No explicit `mock`/`fake`/`stub` test frameworks detected in crate sources.
- Current tests rely on:
  - real fixtures (`fixtures/*.jsonl`, stress fixtures),
  - deterministic in-process backends (`ratatui::TestBackend`) for render assertions.
- Remaining gap is not mock removal; it is adding missing e2e/PTY and CI-governed execution lanes.

## Top 10 High-Risk Gaps (Mapped)

1. Missing CLI e2e transcript suite with deterministic logs -> `bd-2yv.2` (P0)
2. Missing interactive PTY TUI e2e harness -> `bd-2yv.4` (P0)
3. Missing CI-enforced full-confidence gate -> `bd-2yv.6` (P0)
4. Missing explicit defer/waiver ledger validation -> `bd-2yv.7` (P1)
5. Missing sub-5-minute deterministic fastlane contract -> `bd-2yv.8` (P1)
6. Missing operator UX protocol baseline run and scoring artifacts -> `bd-2yv.5` (P1)
7. Missing width-bucket modality validation evidence table -> `bd-gxd.9` (P1)
8. Missing refreshed deterministic UX evidence asset assertions -> `bd-gxd.10` (P1)
9. Missing targeted tests for asset-capture binary behavior (`capture_readme_assets`) -> `bd-2yv.3` (P1)
10. Missing expanded malformed-input corpus in importer/core boundary tests -> `bd-2yv.3` (P1)

## Notes

- This audit is the baseline for `TEST-HARDEN` sequencing; downstream beads should update this file when gaps close.
- Once `cargo llvm-cov` is available, add numeric line/function coverage percentages as an additive section without replacing risk-ranked analysis.

## Progress updates (post-baseline)

- `bd-2yv.8` completed: deterministic fastlane lane is now implemented and enforced in CI (`scripts/e2e/fastlane.sh`, `.github/workflows/ci.yml`).
- `bd-2yv.7` completed: explicit defer register ledger plus validator added and enforced in CI (`docs/testing/defer-register-v0.1.json`, `scripts/testing/validate_defer_register.py`).
- `bd-2yv.6` completed: CI now publishes distinct `fastlane` (PR default) and `full-confidence` (push/merge gate) lanes, uploads structured logs/artifacts, and gates `release-trust` on `full-confidence`.
- `bd-1un` completed: CI now performs explicit PTY preflight before interactive TUI E2E and publishes preflight status logs for operator triage.
