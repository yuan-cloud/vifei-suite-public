# README Asset Playbook (`bd-1w9.3`)

This playbook defines the reproducible path for README visuals.

No AI-generated hero art is used. All assets come from deterministic project commands.

## 1) Generate core assets

```bash
scripts/refresh_readme_assets.sh
```

This command regenerates:

- `docs/assets/readme/incident-lens.txt`
- `docs/assets/readme/forensic-lens.txt`
- `docs/assets/readme/truth-hud-degraded.txt`
- `docs/assets/readme/export-refusal.txt`
- `docs/assets/readme/artifacts-view.txt`
- `docs/assets/readme/architecture.mmd`
- sample eventlog fixtures used by README commands

## 2) Optional terminal recordings for social/docs

`asciinema` (replayable terminal cast):

```bash
scripts/capture_showcase_cast.sh --fast /tmp/vifei-readme-cast
```

Use recordings only when they map to real commands in this repo.

## 3) Asset quality constraints

- Use command output from current `main`.
- Keep captures short and legible.
- Avoid decorative-only visuals.
- Keep evidence-first framing: artifact names, hashes, refusal behavior.

## 4) Verification checklist

- Asset files exist after refresh script run.
- README commands still execute as written.
- No contradiction with:
  - `docs/README_VERIFICATION.md`
  - `docs/RELEASE_TRUST_VERIFICATION.md`
  - constitutional docs (`docs/CAPACITY_ENVELOPE.md`, `docs/BACKPRESSURE_POLICY.md`)
