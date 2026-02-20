# UX Modality Matrix v0.1

## Purpose
Define explicit UX expectations by operating modality so terminal behavior stays clear and trustworthy across widths and README mobile viewing.

This document is implementation guidance for `bd-gxd.2` through `bd-gxd.7`.

## Width Buckets
- Wide desktop: `>= 140` columns
- Standard desktop: `100-139` columns
- Narrow terminal: `80-99` columns
- Emergency narrow: `< 80` columns

## First-Screen Requirements by Bucket

| Surface element | Wide desktop | Standard desktop | Narrow terminal | Emergency narrow |
|---|---|---|---|---|
| Truth HUD identity (6 required fields) | Required | Required | Required | Required |
| Active lens label (Incident/Forensic) | Required | Required | Required | Required |
| Highest-priority anomaly summary | Required | Required | Required | Required (condensed) |
| Next safe action hint | Required | Required | Required | Required (single-line) |
| Secondary summaries/detail panels | Required | Optional | Deferred below fold | Deferred |
| Expanded forensic inspector details | Required | Optional | Deferred by key action | Deferred |

## Copy and Readability Rules
- Critical labels must be short and unambiguous; avoid wrapping core status tokens.
- Helper copy must degrade from full guidance to concise prompts as width shrinks.
- In `< 80` mode, preserve meaning first; drop secondary prose before dropping key status.
- Do not shorten labels in ways that change incident semantics.

## Mobile README Consumption Rules
- Top order must remain: value proposition, quickstart, trust signals, architecture, troubleshooting.
- Every screenshot/capture section must include one-sentence caption explaining what to notice.
- Command blocks should remain copyable in narrow view; avoid long single lines where practical.
- Proof artifacts (`metrics.json`, `viewmodel.hash`, `ansi.capture`, `timetravel.capture`) must stay visible in first half of README flow.

## Anti-Goals
- No web-style decorative patterns that hide operator state.
- No non-deterministic or animation-led guidance behavior.
- No verbosity growth in narrow mode.

## Validation Hooks
- `bd-gxd.9`: validates width-bucket outcomes and mobile readability checks.
- `bd-gxd.10`: ensures refreshed deterministic evidence assets reflect modality-aware UX.
