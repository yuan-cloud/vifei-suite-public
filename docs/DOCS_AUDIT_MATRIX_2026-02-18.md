# Docs Audit Matrix (2026-02-18)

Purpose: confirm that plan/checklist/strategy/todo-style docs are either fully acted upon, intentionally deferred with rationale, or explicitly marked as manual platform work.

Scope source:

- `docs/*` files with names containing `PLAN`, `CHECKLIST`, `PLAYBOOK`, `VERIFICATION`, `QA_REPORT`, `RUBRIC`, `MATRIX`
- markdown files containing actionable checkboxes or deferred/pending markers

## Status Legend

- `DONE`: doc intent is implemented and evidence-linked
- `ACTIVE`: doc governs ongoing operation and remains current
- `MANUAL`: requires GitHub UI/admin action or release-day execution

## Audit Table

| Doc | Role | Status | Evidence |
|---|---|---|---|
| `docs/README_LAUNCH_PLAN.md` | launch and distribution strategy | DONE | status updated to execution-tracked; sequencing constraints and artifact ownership defined |
| `docs/LAUNCH_COMMS_PACK.md` | release messaging templates | DONE | templates align with verified claims and trust docs |
| `docs/README_VERIFICATION.md` | command/trust reproducibility log | DONE | refreshed to 2026-02-18 and aligned to current CLI behavior |
| `docs/README_QA_REPORT.md` | summary QA gate for README | DONE | synced to verification log and current command surface |
| `docs/README_ASSET_PLAYBOOK.md` | deterministic asset generation workflow | DONE | `scripts/refresh_readme_assets.sh` and `capture_readme_assets` path are present and in use |
| `docs/RELEASE_PACKAGING_CHECKLIST.md` | release go/no-go operations | ACTIVE | release-time checklist; no stale/contradicting items found |
| `docs/RELEASE_TRUST_VERIFICATION.md` | checksum/attestation trust procedure | ACTIVE | references CI release-trust flow and rollback rules |
| `docs/POST_LAUNCH_TRIAGE_RUBRIC.md` | post-launch issue triage process | ACTIVE | intentionally process-oriented; deferred bucket policy documented |
| `docs/COMMUNITY_TRIAGE_PLAYBOOK.md` | maintainer intake and escalation workflow | ACTIVE | linked from community-health surfaces; no unresolved placeholders |
| `docs/UX_TEST_PLAN.md` | UX validation workflow | ACTIVE | tied to modality/validation artifacts; no unchecked action list |
| `docs/UX_MODALITY_MATRIX.md` | modality contract (desktop/narrow/readme-mobile) | ACTIVE | enforced by modality tests; remains governance doc |
| `docs/testing/coverage-matrix-v0.1.md` | coverage/risk surface map | ACTIVE | continuously updated with completed beads and explicit out-of-scope sections |
| `docs/testing/perf-opportunity-matrix-a2-2026-02-17.md` | perf candidate ranking with deferrals | DONE | historical decision artifact; deferred C3 path captured and justified |
| `docs/PUBLIC_REPO_SETTINGS_CHECKLIST.md` | public-flip settings checklist | MANUAL | local/in-repo items reconciled; GitHub UI/admin and release-time items remain intentionally unchecked |

## Manual Pending Items (Platform/Release-Time)

From `docs/PUBLIC_REPO_SETTINGS_CHECKLIST.md`, the remaining unchecked items are expected manual tasks:

1. GitHub repository metadata:
- description
- homepage
- topics
- social preview image

2. Branch protection and policy settings:
- required checks enforcement
- force-push and deletion protections
- optional linear-history setting

3. Actions/release-time operations:
- actions policy/retention confirmation in GitHub UI
- release tag + release notes section completion
- operator log signoff for final flip

These are intentionally not auto-marked complete from local workspace state.

## Conclusion

- The planning/checklist/strategy doc set is coherent and largely executed.
- Remaining work is clearly isolated to manual GitHub settings and release-event steps, not untracked repo tasks.
- No ad-hoc markdown TODO tracker was introduced; execution remains bead-driven.
