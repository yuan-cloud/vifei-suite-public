# Feature Priority Decision · 2026-02-19

Context: post-audit planning after full fast/full demo verification and closure-evidence review.

## Six candidate feature directions

1. Strict trust command for operators and automation.
2. Incident evidence-pack UX as first-class daily workflow.
3. Refusal explainability surface beyond raw JSON reports.
4. Adapter diagnostics and support matrix visibility.
5. Determinism duel as product command, not only script.
6. One-command publish-ready visual proof bundle.

## Counter-argument pass (what could be wrong)

1. Strict trust command could become shell-wrapper noise.
2. Evidence-pack UX could over-index on polish before onboarding.
3. Refusal explainability could duplicate refusal-report JSON.
4. Adapter expansion could create low-quality surface sprawl.
5. Determinism duel could remain demo theater without release gating value.
6. Visual proof bundling could consume focus without improving core utility.

## Senior-level decision

Decision rule used: prioritize features that increase operator trust, compress time-to-proof, and preserve deterministic contracts with low complexity growth.

Now:
- Build strict trust verification as a native command (`verify --strict`).
- Keep incident-pack as the core operator handoff artifact and improve summary ergonomics next.
- Add adapter diagnostics (supported inputs plus failure hints) before adding new adapters.

Next:
- Productize determinism duel under the same verify surface.
- Add refusal explainability view only if it produces ranked remediation steps.

Later:
- Visual/publish bundle consolidation after trust and operator workflow improvements are stable.

## Why this ordering

- It converts internal rigor into user-visible product value.
- It avoids over-engineering presentation before operator outcomes.
- It keeps change-safety high by reusing already-tested trust primitives.
