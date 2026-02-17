# PLAN_FOR_ADVANCED_OPTIMIZATIONS_ROUND_1__CODEX

Status: Round A1 refinement (plan-only).

This revision tightens Round A to senior-grade production foundations while keeping scope appropriate for Panopticon v0.1.
No behavior-changing code is proposed in this file.

## 0) Guardrails (non-negotiable)

All optimization candidates must preserve:
- Invariants `I1..I5`
- Locked decisions `D1..D7`
- Canonical ordering by `commit_index`
- Deterministic surfaces (`state_hash`, `viewmodel.hash`, Tour artifacts)

Disallowed in optimization beads unless explicitly approved as a spec-change bead:
- Any change that alters canonical outputs for same canonical inputs
- Any tradeoff that risks Tier A correctness/ordering for speed

## 1) Methodology Compliance (A..G)

A. Baseline first: measured on representative workload with exact commands
B. Profile before proposing: attempted CPU profiling; kernel policy blocked function-level sampling
C. Equivalence oracle: define deterministic artifact and transition parity checks per candidate
D. Isomorphism sketch: each candidate includes semantic-preservation argument
E. Opportunity matrix: rank by `(Impact x Confidence) / Effort`
F. Minimal diffs: one lever per bead
G. Regression guardrails: benchmark checks and deterministic CI assertions

## 2) Baseline (Round A measured)

Environment:
- Repo: `/data/projects/PanopticonAliveca2.5`
- Fixture: `fixtures/large-stress.jsonl` (19,475 events)

Primary command:
- `cargo run -q -p panopticon-tui -- tour --stress fixtures/large-stress.jsonl --output-dir <dir>`

### 2.1 Latency distribution (15 runs)

Command harness:
- repeated `/usr/bin/time -f '%e'` over 15 sequential runs

Results:
- n = 15
- mean = 3.425s
- p50 = 3.410s
- p95 = 3.600s
- p99 = 3.600s
- min = 3.170s
- max = 3.730s

### 2.2 Resource snapshot (`/usr/bin/time -v`)

Tour run:
- wall = 3.74s
- user = 3.56s
- system = 0.21s
- CPU = 101%
- max RSS = 47,520 KB

Full suite (`cargo test`):
- wall = 16.32s
- user = 35.47s
- system = 2.66s
- CPU = 233%
- max RSS = 250,428 KB

## 3) Profiling constraints and fallback

Attempted:
- `perf record/report`

Blocked by host policy:
- `perf_event_paranoid=4` prevented unprivileged sampling

Interpretation:
- No function-by-percent hotspot table is available yet
- Candidate ranking therefore uses measured wall-time + code-path analysis
- Round B should include privileged profiling in a controlled environment before deeper pipeline refactor work

## 4) Current hotspot model (code-path grounded)

Likely dominant Tour costs:
1. Full JSONL parse (`parse_cassette`) for large fixture
2. Append writer replay to EventLog
3. EventLog reread + reducer replay
4. Artifact serialization/writes

Likely dominant test costs:
1. Repeated Tour execution in integration tests
2. Invariant checks that independently reconstruct parse+append+replay

## 5) Scalability and failure-envelope posture (v0.1-appropriate)

This section sets foundation quality without over-engineering.

### 5.1 What matters now
- Predictable behavior near envelope edges (load spikes, larger fixtures, write-path stalls)
- Deterministic degradation signaling, never silent correctness drift
- Fast diagnosis when regressions appear

### 5.2 What we do now vs defer
Do now:
- stage-level benchmark instrumentation
- deterministic regression thresholds
- release integrity basics (attestations/checksums/provenance verification)

Defer:
- distributed ingest architecture
- multi-node scaling controls
- heavy infra observability stacks

## 6) SLO/Error Budget starter set (for this stage)

Proposed initial SLO candidates (documentation + CI gates, not runtime SLIs yet):
1. Tour determinism SLO: identical `viewmodel.hash` and deterministic artifact parity on rerun of same fixture in CI matrix
2. Export safety SLO: secret-seeded export must refuse with refusal report schema intact
3. Truth integrity SLO: `tier_a_drops == 0` in stress harness checks

Proposed error-budget policy:
- If any determinism or Tier A integrity gate fails on main/release branch, feature work pauses until corrected

Reference model:
- Google SRE error budget policy pattern

## 7) Supply-chain and release trust posture (stage-appropriate)

Recommended foundation (now):
1. Artifact checksums in release outputs
2. GitHub artifact attestations for build provenance
3. Optional SBOM attestation publication with release assets
4. Verification steps documented for offline/CI consumers

Reference model:
- GitHub artifact attestation docs
- SLSA staged levels/tracks (adopt practical subset)
- OpenSSF Scorecard checks as a hygiene baseline

## 8) Opportunity matrix (A1)

### Candidate A1-1: Stage-level benchmark harness for Tour
- Impact: High (enables targeted optimization)
- Confidence: High
- Effort: Low-Medium
- Score: High
- Isomorphism sketch: non-functional metrics instrumentation only; no artifact/path changes
- Rollback: remove benchmark harness and related docs/tests

### Candidate A1-2: Reduce duplicated parse/replay work in Tour invariant tests
- Impact: Medium (CI wall-time reduction)
- Confidence: High
- Effort: Low
- Score: High
- Isomorphism sketch: test-path optimization only; product behavior unchanged
- Rollback: revert test helper refactor

### Candidate A1-3: Deterministic artifact serialization mode policy (pretty vs compact)
- Impact: Low-Medium
- Confidence: Medium
- Effort: Low
- Score: Medium
- Isomorphism sketch: if artifact bytes are contract surfaces, keep stable mode default and gate any mode switch by explicit version/contract note
- Rollback: restore prior serializer mode

### Candidate A1-4: Pipeline refactor to reduce parse/append/replay passes
- Impact: Medium-High
- Confidence: Medium-Low (pending function-level profile)
- Effort: Medium-High
- Score: Medium
- Isomorphism sketch: must prove identical committed event sequence and identical derived artifacts/hash outputs
- Rollback: keep old path behind feature flag or revert single bead

### Candidate A1-5: Release trust hardening track (attestations + verification docs)
- Impact: High (enterprise trust signal)
- Confidence: High
- Effort: Medium
- Score: High
- Isomorphism sketch: release-process only, no runtime semantics changes
- Rollback: disable attestation steps in workflow

## 9) Equivalence oracle definitions

For any accepted optimization bead, require these unchanged outputs for fixed fixture input:
1. `metrics.json` required-key presence and semantic consistency
2. `viewmodel.hash` exact value
3. `timetravel.capture` structural validity and commit-index ordering
4. `ansi.capture` deterministic byte stability (where applicable)
5. `degradation_transitions` parity with `PolicyDecision` derivation rules

## 10) Proposed execution sequence (123 mapped to ABCD)

### Web/Research sequence
1. Round 1: external best-practice collection (completed)
2. Round 2: map external guidance to Panopticon constraints (this A1 refinement)
3. Round 3: beadize accepted items with dependencies

### Optimization sequence
A. (this doc) Baseline + candidate ranking
B. Convert accepted items into narrow beads with explicit proof/rollback notes
C. Implement highest-score low-risk items first (`A1-1`, `A1-2`, `A1-5`)
D. Fresh-eye review loop after each bead, then re-baseline

## 11) Do/Don’t at this stage

Do:
- strengthen determinism and operational trust posture
- optimize where measured and low-risk
- improve benchmark observability before architectural rewrites

Don’t:
- chase hypothetical scale with major refactors before evidence
- weaken artifact determinism for cosmetic speed wins
- expand system complexity without clear envelope benefit

## 12) References (current)

- GitHub artifact attestations:
  - https://docs.github.com/actions/security-for-github-actions/using-artifact-attestations/using-artifact-attestations-to-establish-provenance-for-builds
  - https://docs.github.com/en/actions/security-for-github-actions/using-artifact-attestations/verifying-attestations-offline
- SLSA:
  - https://slsa.dev/blog/2025/04/slsa-v1.1
  - https://slsa.dev/spec/v1.0-rc1/levels
- OpenSSF Scorecard checks:
  - https://github.com/ossf/scorecard
- SRE error budget policy reference:
  - https://sre.google/workbook/error-budget-policy/
- NIST SSDF:
  - https://csrc.nist.gov/pubs/sp/800/218/final
  - https://csrc.nist.gov/pubs/sp/800/218/a/final

## 13) Recommendation

Approve A1 and proceed to Round 3 beadization with this order:
1. Benchmark harness (A1-1)
2. Test-path dedup (A1-2)
3. Release trust hardening (A1-5)
4. Re-baseline and decide if pipeline refactor (A1-4) is justified
