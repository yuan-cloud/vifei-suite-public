# Coverage Matrix v0.1

## Scope

Baseline coverage audit for:

- `vifei-core`
- `vifei-import`
- `vifei-export`
- `vifei-tour`
- `vifei-tui`

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
cargo test --workspace --all-targets -- --list > /tmp/vifei-test-list.txt

# Per-crate inventory snapshots
cargo test -p vifei-core --all-targets -- --list > /tmp/core-tests.txt
cargo test -p vifei-import --all-targets -- --list > /tmp/import-tests.txt
cargo test -p vifei-export --all-targets -- --list > /tmp/export-tests.txt
cargo test -p vifei-tour --all-targets -- --list > /tmp/tour-tests.txt
cargo test -p vifei-tui --all-targets -- --list > /tmp/tui-tests.txt
```

Observed during this audit (refresh: 2026-02-18 after `bd-im6j`):

- Numeric coverage is produced in CI via `scripts/testing/coverage_numeric.sh` and uploaded in full-confidence artifacts.
- Local environments without `cargo llvm-cov` still use the inventory fallback plus risk-ranked uncovered-path mapping.

## Baseline Snapshot

| Crate | Source surfaces (`src/`) | Listed tests (from `--list`) | Integration/E2E signals |
|---|---:|---:|---|
| `vifei-core` | 6 files | 157 (`156` unit + `1` docs guard) | No PTY e2e; strong invariant/property-style unit focus |
| `vifei-import` | 2 files | 24 (`21` unit + `3` integration) | Fixture-backed integration exists |
| `vifei-export` | 2 files | 50 (`38` unit + `12` integration) | Refusal-path integration exists |
| `vifei-tour` | 3 files (+ bench targets) | 30 (`16` unit + `14` integration-like) | Stress fixtures + invariants present |
| `vifei-tui` | 7 files | 76 (`64` unit + `12` integration/e2e) | Includes PTY interactive e2e and CLI contract integration tests |

Workspace inventory file lines:

- `/tmp/vifei-test-list.txt`: `337` test entries (`: test$`)

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
| CLI command path (`vifei` bin) | Unit + integration contract tests | Need broader success-path JSON contract matrix and transcript-friendly golden checks | P1 | `bd-1z3.2` |
| CI test governance | Fastlane/full-confidence and PTY preflight in place | Need cleaner, denser failure artifacts for rapid operator triage | P1 | `bd-1z3.3` |

## No-Mock/Fake Assessment

- No explicit `mock`/`fake`/`stub` test frameworks detected in crate sources.
- Current tests rely on:
  - real fixtures (`fixtures/*.jsonl`, stress fixtures),
  - deterministic in-process backends (`ratatui::TestBackend`) for render assertions.
- Remaining gap is not mock removal; it is adding missing e2e/PTY and CI-governed execution lanes.

## v0.1 Completeness Contract ("full enough")

Vifei v0.1 does not claim mathematical full coverage. It claims contract-complete coverage for the risk surfaces that matter to truth, determinism, and operator trust.

Required for "full enough" in v0.1:

1. Truth path contract complete:
   - append writer ownership of `commit_index` is tested and guarded.
   - reducer/projection determinism tests are green for normal and stress fixtures.
2. CLI contract complete:
   - robot envelope shape and exit-code mapping are covered for success and high-value error classes.
   - alias behavior and bounded normalization behavior are covered.
3. TUI operator path complete:
   - incident/forensic lens transitions, Truth HUD surface, and PTY preflight behavior are covered.
4. Share-safe/export path complete:
   - clean export success and refusal/report behavior are covered with deterministic fixtures.
5. E2E diagnostics complete:
   - e2e outputs include structured stage logs, per-command transcripts, and replay hints.

Out-of-scope for v0.1 "full enough":

- full combinatorial argv proof over all token permutations.
- browser/web UI automation.
- probabilistic/fuzz proof over every malformed external payload family.

## Invariant And Decision Coverage Ownership

| Contract | Primary test ownership |
|---|---|
| I1: EventLog truth, Tier A lossless ordering | `vifei-core` eventlog/reducer tests; `vifei-tour` invariants |
| I2: deterministic projection, projection degrades before truth | `vifei-core` projection tests; `vifei-tour` deterministic artifact tests |
| I3: share-safe export refusal with structured report | `vifei-export` unit/integration tests; CLI contract tests for refusal code path |
| I4: determinism is testable in CI | `vifei-tour` invariants + CI fastlane/full-confidence workflows |
| I5: loud failure posture | CLI contract/error-path tests + explicit refusal/runtime error mapping |
| D1: Agent Cassette first importer | `vifei-import` cassette parser/mapping integration tests |
| D2: Tier A minimal set remains protected | `vifei-core` event typing + projection/Tour invariant tests |
| D3: local-only CLI/TUI posture | CLI/TUI integration tests and workflow surface checks |
| D4: append-only truth + rebuildable cache posture | `vifei-core` and importer/export integration behavior checks |
| D5: Incident default, Forensic toggle | `vifei-tui` unit tests + PTY interactive e2e |
| D6: canonical ordering by `commit_index` only | `vifei-core` append/replay tests; tour parity tests |
| D7: branch-policy neutrality | process/CI governance checks, not runtime tests |

## Required E2E Logging Gates

Any e2e lane considered passing for v0.1 must emit:

- `run.jsonl` with stable stage/status records.
- `summary.txt` with concise pass/fail and replay pointers.
- `cmd/*.stdout.log` and `cmd/*.stderr.log` transcripts.
- relevant artifact pointers (Tour/export/PTY assertion files) in logs.

Failure diagnostics minimums:

- failed stage id,
- expected vs actual exit code,
- exact replay command,
- transcript path(s) for first failure.

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
- Numeric line/function coverage is additive and does not replace risk-ranked analysis.

## Progress updates (post-baseline)

- `bd-2yv.8` completed: deterministic fastlane lane is implemented and enforced in CI (`scripts/e2e/fastlane.sh`, `.github/workflows/ci.yml`).
- `bd-2yv.7` completed: defer register ledger plus validator are enforced in CI (`docs/testing/defer-register-v0.1.json`, `scripts/testing/validate_defer_register.py`).
- `bd-2yv.6` completed: CI publishes `fastlane` and `full-confidence` lanes with structured artifacts.
- `bd-1un` completed: CI performs PTY preflight before interactive TUI E2E and publishes preflight status logs.
- `bd-2fp.5` completed: Tour now avoids rereading EventLog by consuming append-result committed sequence; parity test added in `crates/vifei-tour/src/lib.rs`.
- `bd-1jr` completed: CLI contract topology coverage expanded (global/subcommand ordering equivalence, unknown-argument deterministic guidance, human replay hints) with high-value envelope golden checks in `crates/vifei-tui/tests/cli_robot_mode_contract.rs`.
- `bd-15z` completed: PTY interactive diagnostics now converge on deterministic JSON schemas (`vifei-pty-preflight-v1`, `vifei-tui-e2e-assert-v1`) with stable reason-code taxonomy, transcript pointers, and CI artifact presence checks.
- `bd-12m` completed: export/tour E2E expanded from artifact existence checks to cross-artifact semantic consistency and refusal-report schema/order determinism checks, including mixed inline+blob secret corpus coverage.
- `bd-3fx` completed: CI now enforcing contract-tagged diagnostics (`FL0/FL1/CC*/FC1`) with explicit replay commands, coverage-contract path freshness checks, and artifact-stage presence validation across fastlane/full-confidence lanes.
- `bd-qra` completed: full-confidence now treats PTY as capability-gated, runs interactive PTY only on passing preflight, and enforces deterministic PTY assertion/flake budget contracts via `scripts/testing/check_pty_flake_contract.sh`.
- `bd-22i` completed: fastlane now records PTY preflight capability status explicitly and gates interactive PTY smoke deterministically, while CI validates `tui_pty_preflight` stage presence and preflight artifact output.
- `bd-2gs` completed: PTY flake checker now emits explicit lane-scope diagnostics for wrong-directory usage (fastlane vs full-confidence), preserving strict failures while reducing first-use operator confusion.
- `bd-qhk` completed: baseline refresh pack captured fastlane/cli-e2e results, 12-run Tour latency distribution, release bench percentiles, and resource snapshots to drive profiler-first candidate selection.
- `bd-2jw` completed: hotspot evidence captured with non-privileged stage profiling fallback (`profile_tour`) plus memory/I-O envelope counters; privileged profiler blocker and repro recipe documented in `docs/testing/perf-hotspots-a2-2026-02-17.md`.
- `bd-14f` completed: opportunity matrix established with evidence-linked scoring, explicit equivalence oracles, isomorphism proof sketches, regression guardrails, rollback plans, and a single-lever next implementation pick (C1 in-place reducer path).
- `bd-qx4` completed: implemented C1 (in-place reducer fold) with replay parity proof test and measured before/after latency evidence in `docs/testing/perf-c1-inplace-reducer-a2-2026-02-17.md`.
- `bd-18m` completed: investigation-flow UX audit captured desktop+narrow evidence, ranked top operator friction, and defined a single deterministic UX lever for implementation (`docs/testing/ux-audit-a2-2026-02-17.md`).
- `bd-hov` completed: Incident Lens now keeps explicit `Next action:` guidance visible in narrow layouts with width-aware hint text and wrap-aware section height budgeting; regression covered by `incident_lens_narrow_keeps_next_action_hint_visible`.
- `bd-3q5` completed: A2 enterprise-ready closeout consolidates before/after perf deltas, hotspot closure status, UX gains, determinism safety checks, and residual risk posture in `docs/testing/a2-closeout-report-2026-02-17.md`.
- `bd-6wkf` completed: A3 C2 streaming fixture parse is implemented with reader-mode equivalence proof (`stream_fixture_parse_matches_buffered_parse`) and full gate pass, documented in `docs/testing/perf-c2-streaming-parse-proof-a3-2026-02-17.md`.
- `bd-2aum` completed: post-C2 profile refresh confirms further p95 improvement with updated hotspot split (parse vs append) and refreshed memory/I-O envelope in `docs/testing/perf-hotspots-a3-post-c2-2026-02-17.md`.
- `bd-3ufi` completed: C3 decision gate recorded as no-go for A3 based on post-C2 hotspot split and proof/complexity tradeoff (`docs/testing/perf-c3-decision-gate-a3-2026-02-17.md`).
- `bd-141p` completed: A3 closeout report consolidates C2 implementation proof, post-C2 performance deltas, C3 no-go rationale, and next-round guidance in `docs/testing/a3-closeout-report-2026-02-17.md`.
- `bd-xx6w` completed: A4 typed-cassette parser path replaced generic `Value` lookup mapping in importer; tests remained green and profile evidence shows parse share reduction plus improved p95 envelope (`docs/testing/perf-a4-typed-cassette-parser-2026-02-17.md`).
- `bd-qhgs` completed: A4-2 fractional timestamp parser now uses a zero-allocation digit loop with explicit parity tests for truncation/padding/invalid-fraction behavior; profile evidence shows further parse-share reduction (`docs/testing/perf-a4-2-fractional-timestamp-parser-2026-02-17.md`).

## Coverage initiative closeout snapshot (TEST-4.6)

Proven in this initiative:

1. Coverage governance is explicit and auditable via this matrix, fastlane/full-confidence docs, and CI-enforced contract checks.
2. High-risk contract surfaces now have deterministic diagnostics with replay commands and stage markers for operator triage.
3. Refusal/export and Tour artifact checks moved from existence-only to semantic consistency assertions.

Still out-of-scope for v0.1:

1. Full combinatorial argv proof across all shell tokenization/quoting permutations.
2. Browser/web automation coverage.
3. Exhaustive probabilistic/fuzz proof over all malformed external payload families.
