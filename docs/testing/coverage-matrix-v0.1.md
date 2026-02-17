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

Observed during this audit (refresh: 2026-02-17 after `bd-2fp.5`):

- `cargo llvm-cov` is not installed in this environment (`error: no such command: llvm-cov`).
- Fallback method used: explicit test inventory + risk-ranked uncovered-path mapping.

## Baseline Snapshot

| Crate | Source surfaces (`src/`) | Listed tests (from `--list`) | Integration/E2E signals |
|---|---:|---:|---|
| `panopticon-core` | 6 files | 157 (`156` unit + `1` docs guard) | No PTY e2e; strong invariant/property-style unit focus |
| `panopticon-import` | 2 files | 24 (`21` unit + `3` integration) | Fixture-backed integration exists |
| `panopticon-export` | 2 files | 50 (`38` unit + `12` integration) | Refusal-path integration exists |
| `panopticon-tour` | 3 files (+ bench targets) | 30 (`16` unit + `14` integration-like) | Stress fixtures + invariants present |
| `panopticon-tui` | 7 files | 76 (`64` unit + `12` integration/e2e) | Includes PTY interactive e2e and CLI contract integration tests |

Workspace inventory file lines:

- `/tmp/panopticon-test-list.txt`: `337` test entries (`: test$`)

## Surface Matrix

| Surface | Current coverage evidence | Uncovered/weak path | Risk | Owner bead |
|---|---|---|---|---|
| `core/event.rs` schema + serde | Extensive roundtrip/order tests | No mutation/fuzz lane for hostile JSONL inputs | P1 | `bd-2yv.3` |
| `core/eventlog.rs` append writer + skew checks | Monotonic/index/skew tests present | Large malformed-stream recovery scenarios limited | P1 | `bd-2yv.3` |
| `core/reducer.rs` deterministic replay | Determinism and checkpoint suites present | No long-run memory/regression envelope assertions | P2 | `bd-2yv.3` |
| `core/projection.rs` hash/invariant checks | Strong hash stability tests | No adversarial high-cardinality stress matrix | P2 | `bd-2yv.3` |
| `import/cassette.rs` mapping/parser | Unit + fixture integration | Broader malformed cassette corpus not yet codified | P1 | `bd-2yv.3` |
| `export/lib.rs` + `scanner.rs` | Refusal + clean path integration | Adversarial corpus breadth still limited for mixed secret payload forms | P1 | `bd-1z3.2` |
| `tour/lib.rs` + stress fixtures | Determinism and ladder invariants tested | Need stage-level benchmark trend tracking in CI artifacts | P1 | `bd-1z3.3` |
| `tui/lib.rs` app keyflow | Unit tests for tab/nav/quit/hud + PTY e2e | PTY behavior still environment-sensitive; logging can improve triage when preflight fails | P1 | `bd-1z3.3` |
| `tui/incident_lens.rs` + `forensic_lens.rs` | Rich unit render tests | Add more width/scroll interaction coverage under PTY flows | P1 | `bd-1z3.2` |
| `tui/src/bin/capture_readme_assets.rs` | Built in test target; no tests (`0`) | No direct snapshot/assertion contract for generated assets | P1 | `bd-gxd.10`, `bd-2yv.3` |
| CLI command path (`panopticon` bin) | Unit + integration contract tests | Need broader success-path JSON contract matrix and transcript-friendly golden checks | P1 | `bd-1z3.2` |
| CI test governance | Fastlane/full-confidence and PTY preflight in place | Need cleaner, denser failure artifacts for rapid operator triage | P1 | `bd-1z3.3` |

## No-Mock/Fake Assessment

- No explicit `mock`/`fake`/`stub` test frameworks detected in crate sources.
- Current tests rely on:
  - real fixtures (`fixtures/*.jsonl`, stress fixtures),
  - deterministic in-process backends (`ratatui::TestBackend`) for render assertions.
- Remaining gap is not mock removal; it is adding missing e2e/PTY and CI-governed execution lanes.

## Top 10 High-Risk Gaps (Mapped)

1. Expand CLI success-path JSON contract matrix and transcript checks -> `bd-1z3.2` (P1)
2. Add adversarial malformed-input corpus for importer/core boundaries -> `bd-1z3.2` (P1)
3. Expand export scanner mixed-secret corpus and edge delimiters -> `bd-1z3.2` (P1)
4. Add PTY width/scroll interaction assertions under scripted sessions -> `bd-1z3.2` (P1)
5. Add benchmark trend capture output to testing artifacts -> `bd-1z3.3` (P1)
6. Improve e2e failure logs with compact replay hints and expected markers -> `bd-1z3.3` (P1)
7. Add dedicated checks for `capture_readme_assets` outputs and shape stability -> `bd-1z3.2` (P1)
8. Audit and tighten public community-health docs for maintainability posture -> `bd-1bv.1` (P2)
9. Define public repo settings/taxonomy checklist for discoverability quality -> `bd-1bv.2` (P2)
10. Add coverage refresh cadence note to prevent stale matrix drift -> `bd-1z3.3` (P2)

## Notes

- This audit is the baseline for `TEST-HARDEN` sequencing; downstream beads should update this file when gaps close.
- Once `cargo llvm-cov` is available, add numeric line/function coverage percentages as an additive section without replacing risk-ranked analysis.

## Progress updates (post-baseline)

- `bd-2yv.8` completed: deterministic fastlane lane is implemented and enforced in CI (`scripts/e2e/fastlane.sh`, `.github/workflows/ci.yml`).
- `bd-2yv.7` completed: defer register ledger plus validator are enforced in CI (`docs/testing/defer-register-v0.1.json`, `scripts/testing/validate_defer_register.py`).
- `bd-2yv.6` completed: CI publishes `fastlane` and `full-confidence` lanes with structured artifacts.
- `bd-1un` completed: CI performs PTY preflight before interactive TUI E2E and publishes preflight status logs.
- `bd-2fp.5` completed: Tour now avoids rereading EventLog by consuming append-result committed sequence; parity test added in `crates/panopticon-tour/src/lib.rs`.
