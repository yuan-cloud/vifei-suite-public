## Summary

- What changed
- Why this change is needed
- What was intentionally not changed

## Evidence

- Reproduction or validation commands:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test`
- Determinism/export evidence when relevant:
  - `viewmodel.hash` comparison notes
  - refusal report behavior notes (`--share-safe`)

## Scope and Risk

- Invariants touched (I1..I5 from `PLANS.md`):
- Constitution impact (`docs/CAPACITY_ENVELOPE.md` / `docs/BACKPRESSURE_POLICY.md`):
- Security/privacy impact:
- Performance impact:

## Submission Notes

- This repository may use PRs as discussion artifacts.
- External PRs are not guaranteed to be merged as-is.
- Maintainers may reimplement equivalent fixes through internal agent workflows.

## Safety Checklist

- [ ] I did not include secrets, tokens, or private credentials.
- [ ] I provided exact commands and observable outputs.
- [ ] I linked any related issue(s) and context.
- [ ] If this is a security issue, I used private reporting in `SECURITY.md`.
