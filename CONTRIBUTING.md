# Contributing

Thanks for taking interest in Vifei Suite.

## Contribution model

This project currently does not accept direct external code contributions for merge.
The maintainer reviews all changes through internal agent-assisted workflows and lands fixes directly.

What is welcome:

- Bug reports with clear reproduction steps
- Determinism break reports (hash drift, ordering drift, artifact mismatch)
- Performance regression reports with commands and measured output
- Security reports via the process in `SECURITY.md`
- Documentation clarity issues

What to expect for pull requests:

- PRs may be used as discussion artifacts
- PRs are not guaranteed to be reviewed or merged
- Equivalent fixes may be reimplemented internally
- Use `.github/pull_request_template.md` so reports include validation evidence

## Best way to help

1. Open an issue with exact commands and observed output.
2. Include environment details (OS, Rust toolchain, terminal/multiplexer).
3. Attach minimal fixtures or logs when possible.
4. If the issue is security-sensitive, use `SECURITY.md` instead of public issues.

If you still open a PR, include:

- exact validation commands run
- determinism/export evidence when relevant
- explicit scope boundaries and risk notes

## Report quality checklist

- Expected behavior
- Actual behavior
- Reproduction steps
- Minimal input fixture
- Logs or error output
- Impact category (correctness, safety, UX, performance)
