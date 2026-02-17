# Support

## Where to ask for help

- Use GitHub Issues for bugs, reproducibility failures, and docs gaps.
- Use Discussions if/when enabled for broader design questions.
- Maintainers triage using `docs/COMMUNITY_TRIAGE_PLAYBOOK.md`.
- Use the issue forms under `.github/ISSUE_TEMPLATE/` for bug and determinism reports.

## What to include

- command(s) run
- exact output and exit code
- fixture path or minimal input
- environment details (OS, terminal, Rust version)
- expected vs actual behavior

## Not for public issues

- security vulnerabilities: follow `SECURITY.md`
- secrets, private keys, bearer tokens, personal data

## Triage priorities

1. Determinism and integrity regressions
2. Tier A correctness and ordering risks
3. Export safety and redaction failures
4. CI reproducibility failures
5. UX/documentation issues
