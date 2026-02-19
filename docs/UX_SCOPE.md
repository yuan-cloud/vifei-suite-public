# UX Scope v0.1 (Terminal-First)

## Purpose
Define what "premium" UX means for Vifei in v0.1 without drifting into web/SaaS aesthetics that do not improve operator outcomes.

This scope applies to:
- CLI command surfaces (`view`, `tour`, `export`)
- TUI lenses (Incident Lens, Forensic Lens, Truth HUD)
- README/operator-facing guidance that supports the same workflows

Constitutional behavior rules remain in:
- `docs/CAPACITY_ENVELOPE.md`
- `docs/BACKPRESSURE_POLICY.md`

Modality-specific expectations are defined in:
- `docs/UX_MODALITY_MATRIX.md`

Visual/copy tone standards are defined in:
- `docs/UX_VISUAL_TONE.md`

## Premium UX Principles for Vifei
1. Clarity first
Critical state and next safe action must be obvious in the first screenful.

2. Recovery without guesswork
Failures and refusals must explain cause, likely operator mistake, and exact next command.

3. Triage speed over decoration
Information hierarchy should prioritize what needs action now, then context, then detail.

4. Deterministic trust
UX surfaces must remain deterministic and auditable; no non-deterministic behavior hidden behind style.

5. Progressive guidance
New operators get concise orientation; experienced operators are not spammed by repetitive helper copy.

## Surface Mapping
- CLI errors/refusals -> principle 2 (`bd-gxd.2`)
- First-run hints/onboarding -> principles 1,5 (`bd-gxd.3`)
- Incident Lens section ordering -> principles 1,3 (`bd-gxd.4`)
- Context-aware key hints -> principles 1,5 (`bd-gxd.5`)
- Visual/copy tone -> principles 1,3,4 (`bd-gxd.6`)
- Modality matrix + validation -> principles 1,2,3,4,5 (`bd-gxd.8`, `bd-gxd.9`)
- Evidence asset refresh -> principles 3,4 (`bd-gxd.10`)

## Non-Goals (v0.1)
- Decorative "premium" effects that do not improve operator task success
- Web-app-first interactions transplanted into terminal UX
- Animation-heavy or non-deterministic presentation behavior
- Expanding scope into new UI platforms

## Prioritized Pain Points

| Rank | Pain point | User impact | Effort | Planned bead |
|---|---|---|---|---|
| 1 | CLI failures require docs lookup for recovery | High: blocks completion | Low | `bd-gxd.2` |
| 2 | First-run control discoverability is too implicit | High: slows adoption | Medium | `bd-gxd.3` |
| 3 | Incident Lens urgency hierarchy is weak | High: slower triage | Medium | `bd-gxd.4` |
| 4 | Key-hint relevance is inconsistent by context | Medium: cognitive load | Medium | `bd-gxd.5` |
| 5 | Visual/copy tone drifts across surfaces | Medium: trust/readability | Low | `bd-gxd.6` |
| 6 | Modality behavior is implicit, not specified | Medium: width regressions | Low | `bd-gxd.8` |
| 7 | Validation lacks modality proof linkage | Medium: weak closure criteria | Medium | `bd-gxd.9`, `bd-gxd.10` |

## Done Criteria for This Scope Document
- Single canonical UX scope reference exists (`docs/UX_SCOPE.md`).
- Principles and non-goals are explicit and terminal-first.
- Pain points are ranked and mapped to active beads.
- Downstream beads can implement without re-litigating UX strategy.
