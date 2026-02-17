# Community Triage Playbook (v0.1)

Purpose: keep inbound issues high-signal while preserving maintainer velocity.

## Intake lanes

- `bug`: reproducible defects with clear commands/output
- `determinism`: hash drift, ordering drift, artifact mismatch
- `docs`: clarity or correctness gaps in operator docs
- `security`: never triaged publicly; route to `SECURITY.md`

## Severity buckets

- `S0`: deterministic truth or Tier A correctness risk (I1/I2/I5)
- `S1`: export safety, refusal-path, or secret redaction risk (I3)
- `S2`: CI/reproducibility degradation, tooling breakage
- `S3`: UX/docs polish, non-blocking quality improvements

## Target response posture

- S0: acknowledge same day; start fix bead immediately
- S1: acknowledge within 24h; prioritize next active cycle
- S2: acknowledge within 72h; schedule by impact/risk
- S3: batch into polish tracks

## Triage checklist

1. Confirm report has exact reproduction commands.
2. Confirm expected vs actual behavior is explicit.
3. Check for deterministic evidence artifacts when relevant:
   - `viewmodel.hash`
   - `metrics.json`
   - `ansi.capture`
   - `timetravel.capture`
4. Tag severity (`S0`..`S3`) and lane (`bug`, `determinism`, `docs`).
5. Create or link bead ID for tracked work.
6. Close as `needs-more-info` if reproduction is insufficient.

## Escalation

- Security or secret exposure:
  - move to private advisory flow immediately
  - redact public details
- Potential constitutional drift:
  - link affected section in constitution docs
  - require findings-first note before code change

## Resolution quality bar

- fix includes tests or explicit verification evidence
- risk register entry appended for completed bead work
- handoff note includes invariants touched and open questions

## Labeling scheme (recommended)

- Domain: `community`, `docs`, `ci`, `security`, `determinism`
- Priority: `P0`, `P1`, `P2`, `P3`
- State helpers: `needs-repro`, `needs-info`, `ready`, `blocked`
