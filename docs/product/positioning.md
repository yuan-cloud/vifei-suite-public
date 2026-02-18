# Panopticon Positioning

## Category

Deterministic run evidence for AI agent workflows.

## One-line value proposition

Panopticon records and replays agent runs as deterministic evidence bundles so teams can verify what happened, including under stress.

## Core problem

Most agent tooling is strong at logs and traces but weak on replay guarantees and export safety. Teams can inspect events, but they cannot always prove that two replays are semantically identical.

## Product promise

1. Canonical run truth is append-only and replayable.
2. Canonical ordering is explicit and stable (`commit_index`).
3. Under pressure, truth remains intact while projection degrades honestly.
4. Share-safe export refuses unsafe output and emits explicit refusal detail.

## Target users

1. AI platform engineers operating agent workflows.
2. Security and governance teams reviewing AI run behavior.
3. Reliability teams investigating incidents and regressions.
4. OSS builders who need reproducible demo and verification artifacts.

## Differentiation

1. Determinism-first product design, not best-effort observability.
2. Verifiable artifact flow (`metrics.json`, `viewmodel.hash`, captures).
3. Explicit refusal semantics for unsafe sharing.
4. Local-first architecture suitable for sensitive workflows.

## Positioning statement

For teams running AI agents in production or pre-production, Panopticon is the deterministic evidence cockpit that records, replays, and safely shares run truth with verifiable outputs.
