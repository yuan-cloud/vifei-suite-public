# Post-Launch Feedback Intake and Triage Rubric (v0.1)

Single-source runbook for `bd-3qq.5`.

Goal: convert first-week launch feedback into actionable beads without violating deterministic/truth-first constraints.

## 1) Intake Channels

Capture feedback from these channels only:

- GitHub Issues in this repository
- GitHub Release discussion/comments
- X/LinkedIn launch replies (manually transcribed into a GitHub issue)
- Direct user reports from deterministic repros (CLI output, refusal report, Tour artifacts)

Do not track feedback in ad-hoc markdown TODOs.

## 2) Required Issue Metadata

Every new feedback issue should include:

- Environment: OS, terminal, Rust/tool version
- Command run (exact, copy-pastable)
- Expected behavior
- Actual behavior
- Evidence:
  - for runtime correctness: `metrics.json`, `viewmodel.hash`, `timetravel.capture`
  - for export refusal: `refusal-report.json`
  - for release artifact trust: checksum verification output

If evidence is missing, tag as `needs-repro`.

## 3) Label Taxonomy (first week)

Use these labels:

- `feedback`
- `bug`
- `docs`
- `ux`
- `perf`
- `security`
- `determinism`
- `backpressure`
- `release-ops`
- `needs-repro`
- `accepted`
- `deferred`
- `wontfix`

## 4) Priority Rubric

Assign each issue to one queue:

1. P0 (same day)
- Tier A loss/reorder risk
- Determinism/hash instability on identical inputs
- Share-safe export false-negative (secret missed)
- Release artifact trust verification failure

2. P1 (within 48h)
- Incorrect refusal report details
- Reproducible CLI/TUI behavior bug without Tier A loss
- Major docs mismatch for trust-challenge path

3. P2 (within 1 week)
- UX friction, polish, non-critical docs clarity
- Nice-to-have channel automation and comms refinements

## 5) One-Week Triage Loop

Daily loop for first 7 days:

1. 09:00 UTC: ingest new reports; enforce metadata and labels.
2. 10:00 UTC: classify P0/P1/P2 and assign owner.
3. 14:00 UTC: convert accepted items into beads with dependencies.
4. 18:00 UTC: close duplicates; update canonical issue with status.

Day-7 review:

- summarize counts by label and priority
- publish top 3 reliability findings and fixes shipped
- create next-iteration bead bundle (stabilization vs polish split)

## 6) Feedback-to-Bead Conversion Rules

Create a bead when all are true:

- repro is deterministic or evidence-backed
- scope can be bounded to clear files/modules
- acceptance criteria are testable

Bead naming:

- `BUG-<short>` for correctness issues
- `PERF-<short>` for measured performance regressions
- `UX-<short>` for usability polish
- `DOCS-<short>` for trust-path documentation gaps

Each accepted bead must include:

- failing/guard test plan
- invariants touched (`I1..I5` where applicable)
- risk register requirement reminder

## 7) Guardrails (do not derail core)

- No optimization/polish bead may preempt an open P0/P1 determinism/security issue.
- Do not merge fixes that change canonical behavior without explicit plan/bead update.
- Keep launch-week fixes minimal; defer broad refactors unless they unblock a P0/P1.

## 8) Weekly Outputs

By end of week 1, produce:

- `launch-week-feedback-summary.md` (counts, top incidents, resolved/unresolved)
- updated bead graph reflecting accepted follow-up work
- explicit deferred list with rationale

