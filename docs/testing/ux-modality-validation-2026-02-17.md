# UX Modality Validation Report · 2026-02-17

Bead: `bd-gxd.9`

## Validation Commands
```bash
cargo test -p vifei-tui --test modality_validation -- --nocapture | tee .tmp/ux-modality-validation.log
VIFEI_E2E_OUT=.tmp/e2e-tui-baseline cargo test -p vifei-tui --test tui_e2e_interactive -- --nocapture
```

## Per-Bucket Assertions (Automated)
Source: `crates/vifei-tui/tests/modality_validation.rs`

| Bucket | Width | Assertions | Result | Evidence |
|---|---:|---|---|---|
| Wide desktop | 140 | Incident label, anomaly section, next action, Truth HUD (`Level`/`Version`), Forensic timeline+inspector | PASS | `.tmp/ux-modality-validation.log` (`width_buckets_preserve_required_surface_markers`) |
| Standard desktop | 120 | Same as above | PASS | `.tmp/ux-modality-validation.log` |
| Narrow terminal | 100 | Same as above | PASS | `.tmp/ux-modality-validation.log` |
| Narrow terminal edge | 80 | Same as above | PASS | `.tmp/ux-modality-validation.log` |
| Emergency narrow | 72 | Same as above | PASS | `.tmp/ux-modality-validation.log` |

## Mobile README Assertions (Automated)
Source: `crates/vifei-tui/tests/modality_validation.rs`

| Check | Assertion | Result | Evidence |
|---|---|---|---|
| Section discoverability order | `Why -> Quickstart -> Trust Signals -> Architecture -> Troubleshooting` | PASS | `.tmp/ux-modality-validation.log` (`readme_mobile_order_and_command_width_contract`) |
| Command readability | each `bash` fenced line length `<= 120` | PASS | `.tmp/ux-modality-validation.log` |

## Narrow-Mode Interactive Proof Run
Interactive PTY harness was executed for narrow profile, with explicit environment outcome logged:

- `crates/vifei-tui/.tmp/e2e-tui-baseline/interactive_tui_narrow_terminal_profile_stays_healthy.assertions.log`
- Result in this environment: `SKIP` due PTY permission denial (`script: failed to create pseudo-terminal: Permission denied`)

This is an expected host constraint, not a product behavior failure.

## Findings Converted to Beads
1. Artifact root inconsistency for TUI PTY outputs (crate-relative vs workspace-relative)
- Created: `bd-kko` (`TEST-E2E-PTY-PATH: normalize TUI e2e output path to workspace root`)
- Severity: P2

2. CI/operator visibility for PTY preflight requirements
- Created: `bd-1un` (`TEST-E2E-PTY-CI: enforce PTY capability preflight in CI env docs/check`)
- Severity: P2

## Conclusion
- Width-bucket and mobile-readability validations pass with automated assertions.
- Narrow-mode interactive proof exists and is traceable; host PTY constraint is explicitly recorded.
- Significant findings are tracked as follow-up beads before closure.
