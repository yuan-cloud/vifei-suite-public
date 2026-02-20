# UX Visual Tone v0.1

## Purpose
Define one restrained, professional visual/copy standard for terminal UI and README surfaces.

This keeps status semantics consistent and prevents tone drift while preserving deterministic behavior.

## Status Color Semantics

Use the same meaning everywhere:

- `success` (green): healthy, clean, or completed outcome
- `warning` (yellow): attention needed, degraded, or in-progress caution
- `error` (red): failure, refusal, or high-risk condition
- `info` (cyan): identifiers and neutral context labels
- `accent` (magenta): synthesized/special markers and secondary emphasis
- `muted` (dark gray): helper copy, metadata labels, and low-priority chrome

Do not invert these meanings per screen.

## Copy Tone Rules

- Prefer short imperative guidance over prose.
- Keep helper lines action-oriented (`Next action: ...`).
- Avoid hype language and decorative phrasing.
- Keep labels stable across lenses unless meaning changes.

## Emoji Policy (Approved v0.1)

Sparse use is allowed in README/docs, with caps:

- Maximum 1 emoji per section heading block.
- Maximum 8 emoji across the full README.
- No emoji in constitutional docs, risk register entries, or CLI/TUI runtime copy.
- Decorative-only emoji are allowed only when they do not reduce scannability.

No strict decorative ban is enforced; apply judgment and keep usage restrained.

## Scope

Applies to:

- `crates/vifei-tui` lens copy and status styling
- README narrative and launch-facing docs

Does not override constitutional behavior in:

- `docs/CAPACITY_ENVELOPE.md`
- `docs/BACKPRESSURE_POLICY.md`
