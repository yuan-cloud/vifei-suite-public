# Defer Register (Coverage/Test Gaps)

## Purpose
Track explicit, time-bounded waivers for uncovered or deferred test surfaces discovered in `docs/testing/coverage-matrix-v0.1.md`.

## Canonical file
- `docs/testing/defer-register-v0.1.json`

## Validation command
```bash
scripts/testing/validate_defer_register.py docs/testing/defer-register-v0.1.json
```

Validation fails if:
- required fields are missing,
- dates are malformed,
- `expires_on` is in the past,
- or entry IDs are duplicated.

## Weekly review protocol
Run once per week before merging high-risk testing changes:

1. Run validator command.
2. Review each active entry.
3. If the gap is closed, remove the entry in the same change that closes the gap.
4. If the gap remains, update `revisit_on` and (if justified) `expires_on` with rationale in commit message.

## Field contract (per entry)
- `id`: stable unique identifier
- `surface`: uncovered/deferred path description
- `risk_rank`: one of `P0|P1|P2|P3`
- `owner`: accountable owner
- `rationale`: why defer is acceptable short-term
- `compensating_controls`: non-empty list of current protections
- `created_on`, `revisit_on`, `expires_on`: `YYYY-MM-DD`
- `status`: current waiver status (for example `active`, `closed`)
- `linked_beads`: non-empty list of related bead IDs
