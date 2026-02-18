# Panopticon Messaging

## Messaging hierarchy

1. Headline
- Deterministic evidence for AI agent runs.

2. Subheadline
- Record canonical run truth, replay it reliably, and share results safely.

3. Proof message
- Same input, same replay hash, with explicit artifact outputs.

## Audience-specific messaging

### AI platform teams

- Keep run history auditable and reproducible.
- Investigate incidents with deterministic replay, not guesswork.
- Maintain operator trust under load with explicit degradation posture.

### Security and governance

- Preserve run provenance with canonical ordering.
- Enforce safe sharing with refusal reports when secrets remain.
- Keep evidence flows inspectable and scriptable.

### OSS and developer audience

- Run one command and verify outputs directly.
- Use machine-readable CLI mode for automation.
- Use showcase profile for premium presentation without changing truth semantics.

## Trust claims (only claim what can be verified)

1. Replay determinism can be checked by re-running Tour and comparing `viewmodel.hash`.
2. Tier A drop posture can be checked in `metrics.json`.
3. Share-safe refusal behavior can be checked by running export with refusal-report output.

## Anti-claims

1. Do not claim perfect security coverage.
2. Do not claim universal interoperability beyond implemented import/export paths.
3. Do not claim performance targets not backed by current benchmarks.

## Copy blocks for launch pages

### Short product description

Panopticon is a deterministic, local-first cockpit for recording and replaying AI agent runs as evidence bundles.

### Three bullets

1. Canonical ordering and replayable run truth.
2. Verifiable artifact outputs for deterministic checks.
3. Share-safe export refusal with explicit blocked-field reporting.
