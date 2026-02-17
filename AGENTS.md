# AGENTS.md · Panopticon Suite · v0.1

Guidelines for AI coding agents working in this repo.
Read `PLANS.md` first for project context, then follow every rule below.

## Read this first (if you only read 20 lines)

- This repo is governed by the Two Doc Constitution: `docs/CAPACITY_ENVELOPE.md` and `docs/BACKPRESSURE_POLICY.md`.
- Milestones `M0..M8` in `PLANS.md` are beads. Claim one bead at a time.
- Tier A must never drop and must never reorder. Canonical order is `commit_index`.
- After every meaningful code change: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- After completing a bead: append to `docs/RISK_REGISTER.md`, then land the plane (commit + handoff).

If anything below conflicts with the user, the user wins.

---

## RULE 0. THE USER OVERRIDE

If the user instructs something that conflicts with this document, the user instruction wins. No exceptions.

---

## RULE 1. NO FILE DELETION

You may not delete any file or directory unless the user explicitly provides the exact deletion command in this session. This includes files you just created.

---

## RULE 2. IRREVERSIBLE ACTIONS ARE FORBIDDEN

Forbidden unless the user provides the exact command and explicit approval in the same message:

- `git reset --hard`
- `git clean -fd` (or `-fxd`)
- `rm -rf`
- any command that can delete or overwrite committed code or data

If unsure what a command will change, stop and ask.

---

## RULE 3. THE TWO DOC CONSTITUTION IS LAW (v6.2)

Only these two docs are constitutional for v0.1:

- `docs/CAPACITY_ENVELOPE.md`
- `docs/BACKPRESSURE_POLICY.md` (includes "Projection invariants v0.1")

Do not duplicate their numbers, tiers, ladder steps, or failure mode definitions in other files. Link to them instead.

If your change affects behavior under load, update a constitution doc or add a test that enforces the doc.

---

## PROJECT NORTH STAR (do not drift)

Panopticon Suite is a deterministic, local-first cockpit for recording, replaying, and safely sharing agent runs as evidence bundles.

The EventLog is truth. The UI is a projection. Under overload, truth never degrades. Only the projection degrades.

---

## TECH STACK (v0.1)

- Language. Rust (stable toolchain). All code in Rust unless the user approves otherwise.
- TUI. FrankenTUI (Ratatui-compatible fork). Snapshot-testable via `ftui-harness`.
- Serialization. `serde` plus `serde_json`. Deterministic output requires stable container ordering.
- Hashing. BLAKE3 for all content hashes (`viewmodel.hash`, blob addresses, bundle integrity).
- Storage. Append-only JSONL (truth) plus content-addressed blobs plus SQLite cache (rebuildable).
- CLI. `clap` for argument parsing.

Do not add dependencies without checking that they do not introduce nondeterminism (for example HashMap iteration in serialized output, thread-local RNG seeding). Document any new dependency in your handoff note.

## CLI ROBOT MODE (v1 policy for agent users)

The `panopticon` CLI supports a robot-oriented mode for AI agents and automation:

- `--json` emits machine-readable output envelopes.
- If stdout is not a TTY, output auto-switches to JSON unless `--human` is set.
- Error envelopes include `code`, `message`, and actionable `suggestions`.
- Minor syntax variants may be normalized when intent is unambiguous (for example underscore flag variants).
- Ambiguous or invalid syntax must fail with clear structured guidance, not silent guessing.

Use this mode for scripted checks, bead automation, and CI diagnostics where token efficiency and determinism matter.

Parser authority rule for robot mode:

- `clap` is the only authority for command parsing and subcommand aliases.
- Pre-parse normalization is limited to an explicit allowlist of option spelling repairs (for example underscore-to-hyphen long flags).
- Normalization must never rewrite positional tokens and must stop at `--`.
- Any change to parser/normalization behavior must include updates to CLI contract tests (`crates/panopticon-tui/tests/cli_robot_mode_contract.rs` and related unit tests) in the same commit.

---

## MULTI-AGENT REALITY

This repo may have many agents editing concurrently.

- Do not stash, revert, reset, or "clean up" changes you did not author.
- Treat unexpected diffs as legitimate work by other agents.
- Never "restore a clean working tree" by destructive actions.
- If you encounter a merge conflict, resolve only the files you touched. Leave other conflicts for their author.

---

## AGENT MAIL — MULTI-AGENT COORDINATION

Agent Mail is the coordination substrate for multi-agent work on this repo. If you are the only agent in this session (no other coding agents running concurrently), skip this section entirely.

### Placeholders used in examples

- `project_key="<PROJECT_KEY>"` is the Agent Mail project identifier for this repo. It is environment-specific. Do not hardcode filesystem paths into the repo docs.
- For v0.1 milestone beads, the bead ID is `M{n}` (for example `M2`) and the recommended Agent Mail `thread_id` is `PS-M{n}` (for example `PS-M2`).
- File reservation `reason` should be the bead ID (for example `M2`), not the thread id.

### How agents access Agent Mail

Coding agents (Claude Code, Codex CLI, Gemini CLI) access Agent Mail natively via MCP tools. You do NOT need to implement HTTP wrappers or client classes. MCP tools are available directly in your environment (`ensure_project`, `register_agent`, `send_message`, `fetch_inbox`).

If MCP tools are not available, flag it to the user immediately. They may need to start the Agent Mail server.

### What Agent Mail provides

- Identities. Register as a named agent with a task description.
- Inbox and outbox. Send and receive coordination messages.
- Searchable threads. Organize by milestone or bead.
- Advisory file reservations. Prevent edit conflicts between agents.
- Persistent artifacts. All coordination in git (human-auditable).

### Registration

```
ensure_project(project_key="<PROJECT_KEY>")
register_agent(
  project_key="<PROJECT_KEY>",
  program="claude-code",       # or "codex-cli", "gemini-cli"
  model="<your-model>",        # e.g. "opus-4.6"
  name="SchemaBuilder",        # AdjectiveNoun format
  task_description="Implementing M1 event schema"
)
```

### File reservations (before editing)

```
file_reservation_paths(
  project_key="<PROJECT_KEY>",
  agent_name="SchemaBuilder",
  paths=["crates/panopticon-core/src/event.rs"],
  ttl_seconds=3600,
  exclusive=true,
  reason="M1"
)
```

Recommended reservation patterns:

```
"crates/panopticon-core/src/**"       # Core logic (event, reducer, projection)
"crates/panopticon-import/src/**"     # Importer
"crates/panopticon-export/src/**"     # Export
"crates/panopticon-tui/src/**"        # TUI
"crates/panopticon-tour/src/**"       # Tour harness
"docs/**"                             # Constitution and risk register
"fixtures/**"                         # Test fixtures
```

Prefer narrow patterns. Reserve `crates/panopticon-core/src/event.rs` over `crates/panopticon-core/src/**` when possible.

### Communication

```
send_message(
  project_key="<PROJECT_KEY>",
  sender_name="SchemaBuilder",
  to=["WriterBuilder"],
  subject="PS-M1 done — event schema ready",
  body_md="All Tier A event types defined. Round-trip byte stability tests passing. Ready for M2.",
  thread_id="PS-M1"
)
```

### Inbox processing

```
fetch_inbox(
  project_key="<PROJECT_KEY>",
  agent_name="SchemaBuilder",
  since_ts="<ISO8601_UTC>",
  limit=10
)
```

Check inbox:

- Before starting any new bead.
- After completing current bead.
- Periodically during long beads, and immediately after you post or receive coordination messages.

### Fast resource reads

```
resource://inbox/SchemaBuilder?project=<PROJECT_KEY>&limit=5
resource://thread/PS-M1?project=<PROJECT_KEY>&include_bodies=true
```

### Macros vs granular tools

Prefer macros for speed: `macro_start_session`, `macro_prepare_thread`, `macro_file_reservation_cycle`.

Use granular tools when you need precise control: `register_agent`, `file_reservation_paths`, `send_message`, `fetch_inbox`, `acknowledge_message`.

### Troubleshooting Agent Mail

- The `model` string in `register_agent` is metadata only. It does not gate authentication.
- If Agent Mail seems unavailable, verify:
  1. `ensure_project(project_key="<PROJECT_KEY>")` succeeds.
  2. `register_agent(...)` succeeds with your current CLI model string (for example `opus-4.6`).
  3. `fetch_inbox(...)` returns data without transport/tool errors.
- If calls fail before reaching Agent Mail (tool missing or transport failure), this is an MCP client configuration issue in that agent session, not a repo code issue.

### Thread naming conventions

- Milestones: `PS-M{n}` (e.g., `PS-M0`, `PS-M1`, `PS-M8`)
- Features: `PS-{feature}` (e.g., `PS-DOCS-GUARD`, `PS-BLOB-STORE`)
- Issues: `PS-BUG-{desc}` or `PS-PERF-{desc}`

### Common pitfalls

- `"from_agent not registered"`: call `register_agent` with the correct `project_key` first.
- `"FILE_RESERVATION_CONFLICT"`: adjust patterns, wait for TTL expiry, or use non-exclusive reservation.
- File reservations auto-expire after TTL. Release explicitly when done.

---

## BEAD PROTOCOL (claim, implement, verify, risk, handoff)

Every milestone in `PLANS.md` is a bead. Follow this protocol for each.

### 1. Claim

- Read the bead table in `PLANS.md`. Depends on, inputs, outputs, files touched, done when.
- Verify dependencies are complete (their outputs exist and tests pass).
- Announce the bead ID you are taking in your PR description or in your handoff note.

If dependencies are not met, stop and report. Do not start work.

### 2. Implement

- Touch only the files listed in the bead's "Files touched" row, plus test files.
- Shared wiring files (workspace `Cargo.toml`, crate root `lib.rs` module wiring, CI config) are owned by M0 unless your bead explicitly lists them.
- If you discover you must touch an unlisted file, note it in your handoff and explain why.
- Keep changes minimal. Prefer tight diffs over rewrites.

### 3. Verify

- Run the bead acceptance criteria as tests or manual checks.
- Run quality gates (see **QUALITY GATES** below).
- If the bead involves projection or rendering, verify Tour artifact shapes.
- If the bead involves schema, ensure round-trip byte stability tests exist.

### 4. Risk assessment (mandatory between beads)

After finishing verification and before starting any next bead, answer the five risk questions and append to `docs/RISK_REGISTER.md`.

### 5. Handoff

Follow the "LANDING THE PLANE" template exactly. Do not mark a bead complete unless all acceptance criteria and risk assessment are done.

---

## BEADS TOOLS — br AND bv

This project uses Beads for task tracking. Two CLI tools:

- `br` (beads_rust). Create, update, manage beads. Administration.
- `bv` (beads_viewer). Find next work, triage, plan. Work sessions.

In this repo, the v0.1 milestone beads are `M0..M8` as defined in `PLANS.md`. If you create additional beads during implementation, use the bead ID assigned by `br` and name Agent Mail threads `PS-<bead-id>`.

`br` is non-invasive. It never runs git commands automatically. You must manually commit after `br sync --flush-only`.

### Finding and claiming work

```bash
# Find the optimal next bead
bv --robot-next

# Get prioritized triage (full context)
bv --robot-triage

# Get details of a specific bead
br show <bead-id>   # for milestones, bead-id is M0..M8

# Claim a bead
br update <bead-id> --status in_progress

# Complete a bead
br close <bead-id> --reason "Acceptance criteria met"

# Sync beads state to disk
br sync --flush-only
```

### bv robot-safe commands

**CRITICAL: Use ONLY `--robot-*` flags. Bare `bv` launches an interactive TUI that blocks your session.**

| Command | Purpose |
|---------|---------|
| `bv --robot-triage` | Prioritized triage with scores, recommendations, quick wins |
| `bv --robot-next` | Single top pick with claim command |
| `bv --robot-plan` | Parallel execution tracks with unblocks lists |
| `bv --robot-priority` | Priority misalignment detection |
| `bv --robot-insights` | Full graph metrics (PageRank, critical path, cycles) |
| `bv --robot-alerts` | Stale issues, blocking cascades, priority mismatches |

### Agent Mail integration with beads

When starting work on a bead (multi-agent sessions only):

1. Check inbox for coordination messages.
2. Reserve files you will edit.
3. Announce in Agent Mail thread `PS-M{n}`.
4. Do the work.
5. Post completion summary when done.
6. Release file reservations.

### Mapping cheat sheet

| Concept | Value |
|---------|-------|
| Milestone bead ID | `M{n}` (for example `M2`) |
| Milestone mail `thread_id` | `PS-M{n}` (for example `PS-M2`) |
| Mail subject prefix | `[PS-M{n}]` (or `[PS-<bead-id>]` for non-milestone beads) |
| File reservation `reason` | Bead ID (`M{n}` or `<bead-id>`) |
| Commit messages | Start with bead ID (`M2: ...` or `<bead-id>: ...`) |

### Rules

- Do not keep TODO lists in markdown. Create beads instead.
- Do not invent a parallel tracker.
- If you discover work during implementation, create a new bead.
- Mark beads in-progress before starting.
- Mark beads closed only when acceptance criteria are met.

---

## Bead quick recipe. Example for M2 (append writer)

If you claim `M2`:

- Reserve: `crates/panopticon-core/src/eventlog.rs` and `crates/panopticon-core/src/blob_store.rs` (reason `M2`).
- Prove the invariant: `commit_index` assignment exists in exactly one place (append writer). Use `rg -n 'commit_index' -t rust` to audit quickly.
- Run: `cargo test -p panopticon-core` plus the full quality gates before handoff.

---

## SINGLE SOURCE OF TRUTH RULE

| What | Lives in | Nowhere else |
|---|---|---|
| Capacity numbers, thresholds, TARGETs | `docs/CAPACITY_ENVELOPE.md` | anywhere else |
| Backpressure tiers, degradation ladder, projection invariants | `docs/BACKPRESSURE_POLICY.md` | anywhere else |
| Planning, milestones, decisions | `PLANS.md` | anywhere else |
| Agent rules | `AGENTS.md` (this file) | anywhere else |
| Running risks | `docs/RISK_REGISTER.md` | anywhere else |

`PLANS.md` may link and summarize the constitution docs. It must not duplicate their tables, ladder steps, or numeric thresholds.

### Constitution echo guard

If the repo has a `docs_guard` check, it must pass.
Never copy-paste any line from a guarded snippet (marked with `<!-- DOCS_GUARD:BEGIN ... -->` / `<!-- DOCS_GUARD:END ... -->`) in either constitution doc into any other markdown file. This includes table rows, numeric thresholds, tier definitions, and ladder level bullets.
The `docs_guard` test matches character-exact lines after whitespace trimming, ignoring blank lines and pure-formatting lines.
Instead, link to the relevant constitution section and reference it by heading.

---

## ALWAYS UPDATED ARTIFACTS

If your change affects behavior, update the relevant artifact in the same commit:

- `PLANS.md`
- `AGENTS.md`
- `docs/CAPACITY_ENVELOPE.md`
- `docs/BACKPRESSURE_POLICY.md`
- `docs/RISK_REGISTER.md` (append only, never delete)

---

## V0.1 LOCKED DECISIONS (must not be violated)

These are defined in `PLANS.md` under "LOCKED v0.1 DECISIONS". Summary for quick reference:

1. First importer is Agent Cassette JSONL.
2. Canonical ordering is `commit_index` assigned by the single append writer.
3. `timestamp_ns` is metadata only. Never used for canonical ordering.
4. v0.1 is local-only (CLI plus TUI). No daemon.
5. Canonical store is append-only JSONL plus blobs. SQLite is a rebuildable cache.
6. UX defaults to Incident Lens, with `Tab` toggle to Forensic Lens.
7. Importers do not set `commit_index`. The append writer is the sole assigner. The type system or immediate runtime check must enforce this.

If you believe a locked decision must change, propose a patch to `PLANS.md` first. Do not silently implement the change.

---

## DETERMINISM RULES (projection and hashing)

Projection invariants must remain narrowly scoped to honesty mechanics only. No design taste, no palette, no layout rules.

Forbidden in projection logic:

- wall clock dependence for ordering or sampling
- randomness (including thread-local RNG, nondeterministic iteration)
- nondeterministic iteration (example: HashMap iteration without stable ordering)
- terminal size or focus state in truth hash
- floating point formatting without explicit precision

Required:

- iterate events by `commit_index`
- importer-facing event types must not set canonical `commit_index`; only the append writer materializes committed rows
- stable serialization for hashing (stable field order plus stable container ordering)
- explicit include and exclude lists for `viewmodel.hash`, documented near the hashing code
- explicit include and exclude lists for `state_hash`, documented near the reducer hashing code
- `reducer_version` and `projection_invariants_version` included in their respective hash inputs

Practical tip. Prefer `BTreeMap` or sorted `Vec` in hashed paths. Avoid `HashMap` unless you sort before hashing.

---

## BACKPRESSURE AND SAFE FAILURE POSTURE

- Tier A events are never dropped and never reordered.
- Under overload, UI degrades in the documented ladder (see `docs/BACKPRESSURE_POLICY.md`).
- If Tier A cannot be recorded, the system must alarm loudly and enter a defined safe failure mode. No silent limp mode.

---

## SHARE-SAFE EXPORT

- Redaction is required before export.
- Export must refuse when secrets remain.
- Refusal must emit a `refusal-report.json` explaining exactly what blocked export (event id, field path, pattern). See schema contract in `PLANS.md` § "Artifact schema contracts".

---

## TOUR ARTIFACTS ARE NOT OPTIONAL

Any work on overload behavior, projection, or rendering must preserve Tour artifact shapes:

| Artifact | Format |
|---|---|
| `metrics.json` | JSON |
| `viewmodel.hash` | BLAKE3 hex string |
| `ansi.capture` | ANSI text or asciicast v2 |
| `timetravel.capture` | JSON or JSONL |

If your change causes an artifact shape to change, update the CI assertion script and document why in your handoff.

---

## NO SCRIPT-BASED MASS EDITS

Do not run or propose scripts that bulk modify the repo. Make edits deliberately, file by file, and review diffs.

---

## NO FILE PROLIFERATION

Do not create "v2" or "improved" copies of files. New files are allowed only when they represent genuinely new domains that do not fit existing modules.

---

## BRANCH POLICY

Do not assume `main` or `master`. Treat the repo's configured default branch as canonical. Do not add automation that syncs branches in v0.1.

---

## QUALITY GATES

If you changed code, you must run all three:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

If you changed only docs, at least run:

```bash
cargo test   # docs_guard test catches constitution drift
```

Do not commit code that fails any gate.

---

## UBS — BUG SCANNING BEFORE COMMITS

If the `ubs` command is not available in your environment, skip this section and rely on `cargo clippy` and tests. Do not add product features to support UBS.

Golden rule. Run UBS on changed files before every commit. Exit 0 means safe. Exit greater than 0 means fix and re-run.

```bash
ubs file.rs file2.rs                          # Specific files (< 1s) — USE THIS
ubs $(git diff --name-only --cached)          # Staged files — before commit
ubs --only=rust,toml crates/                  # Language filter (3-5x faster)
ubs --ci --fail-on-warning .                  # CI mode — before PR
ubs .                                         # Whole project (ignores target/, Cargo.lock)
```

Output format:

```
Warning  Category (N errors)
    file.rs:42:5 - Issue description
    Suggested fix
Exit code: 1
```

Parse: `file:line:col` for location. Fix hint for how to fix. Exit 0/1 for pass/fail.

Fix workflow:

1. Read finding. Category plus fix suggestion.
2. Navigate `file:line:col`. View context.
3. Verify real issue (not false positive).
4. Fix root cause, not symptom.
5. Re-run `ubs <file>` until exit 0.
6. Commit.

Speed matters. Scope to changed files. `ubs crates/panopticon-core/src/event.rs` (under 1 second) versus `ubs .` (30 seconds). Never full-scan for small edits.

Bug severity:

- Critical (always fix). Memory safety, data races, use-after-free.
- Important (production). Unwrap panics, resource leaks, overflow checks.
- Contextual (judgment). TODO/FIXME, println! debugging.

---

## CODE SEARCH — ast-grep vs ripgrep

If `ast-grep` is not installed, use `rg` and manual edits. Do not add product features to support code search tooling.

**Use `ast-grep` when structure matters.** Parses AST nodes, ignores comments and strings, and can safely rewrite code.

**Use `ripgrep` when text is enough.** Fastest way to grep literals and regex.

Rule of thumb:

- Need correctness or applying changes → `ast-grep`
- Need raw speed or hunting text → `rg`
- Often combine: `rg` to shortlist files, then `ast-grep` to match or modify

Rust examples:

```bash
# Find structured code (ignores comments)
ast-grep run -l Rust -p 'fn $NAME($$$ARGS) -> $RET { $$$BODY }'

# Find all unwrap() calls
ast-grep run -l Rust -p '$EXPR.unwrap()'

# Quick textual hunt
rg -n 'commit_index' -t rust

# Combine speed + precision
rg -l -t rust 'unwrap\(' | xargs ast-grep run -l Rust -p '$X.unwrap()' --json
```

### Morph Warp Grep — AI-powered code search

If the MCP tool is not available in your environment, skip this section and use `rg`. Treat warp_grep as an optional workflow accelerator, not a repo requirement.

Use `mcp__morph-mcp__warp_grep` for exploratory "how does X work?" questions. An AI agent expands your query, greps the codebase, reads relevant files, and returns precise line ranges with context.

```
mcp__morph-mcp__warp_grep(
  repoPath: "<PROJECT_KEY>",
  query: "How does the backpressure ladder transition work?"
)
```

When to use what:

| Scenario | Tool |
|----------|------|
| "How is the event reducer implemented?" | `warp_grep` |
| "Where is `commit_index` assigned?" | `ripgrep` |
| "Find all uses of `BTreeMap`" | `ripgrep` |
| "Replace all `unwrap()` with `expect()`" | `ast-grep` |

Anti-patterns:

- Do not use `warp_grep` to find a specific function name. Use `ripgrep`.
- Do not use `ripgrep` to understand "how does X work." Use `warp_grep`.
- Do not use `ripgrep` for codemods. Use `ast-grep`.

---

## CASS AND CASS MEMORY — LEARNING FROM HISTORY

If `cass` or `cm` is not available in your environment, skip this section. Do not implement product features to support CASS or CM.

### cass — cross-agent session search

`cass` indexes prior agent conversations (Claude Code, Codex, Gemini CLI) so you can reuse solved problems instead of re-solving them.

Never run bare `cass` (TUI). Always use `--robot` or `--json`.

```bash
cass health                                         # Check index status
cass search "determinism hash stability" --robot --limit 5   # Find prior solutions
cass view /path/to/session.jsonl -n 42 --json       # View specific exchange
cass expand /path/to/session.jsonl -n 42 -C 3 --json  # Expand with context
```

Tips:

- Use `--fields minimal` for lean output.
- Filter by agent with `--agent`.
- Use `--days N` to limit to recent history.
- stdout is data only, stderr is diagnostics, exit 0 means success.

### cm — procedural memory

The Cass Memory System gives agents persistent memory distilled from prior sessions.

Before starting complex tasks, retrieve relevant context:

```bash
cm context "implementing backpressure ladder transitions" --json
```

Returns:

- `relevantBullets`. Rules that may help with your task.
- `antiPatterns`. Pitfalls to avoid.
- `historySnippets`. Past sessions that solved similar problems.
- `suggestedCassQueries`. Searches for deeper investigation.

Protocol:

1. START. Run `cm context "<task>" --json` before non-trivial work.
2. WORK. Reference rule IDs when following them (e.g., "Following b-8f3a2c...").
3. FEEDBACK. Leave inline comments when rules help or hurt.
4. END. Finish your work. Learning happens automatically.

---

## RCH — REMOTE COMPILATION HELPER

If `rch` is not available, run builds locally. Do not change the repo to require remote compilation.

RCH offloads `cargo build`, `cargo test`, `cargo clippy`, and other compilation commands to remote workers instead of building locally. This prevents compilation storms when many agents run simultaneously.

If RCH is installed and hooked into your agent's PreToolUse, builds are intercepted and offloaded transparently. Otherwise, you can manually offload:

```bash
rch exec -- cargo build --release
rch exec -- cargo test
rch exec -- cargo clippy
```

Quick commands:

```bash
rch doctor              # Health check
rch status              # Overview of current state
rch queue               # See active/waiting builds
```

If RCH or its workers are unavailable, it fails open — builds run locally as normal.

---

## RISK ASSESSMENT (run after every bead)

After completing a bead, answer these five questions and append to `docs/RISK_REGISTER.md`:

1. Coupling. What new coupling did we introduce that will be painful in 3 months.
2. Untested claims. What correctness claim did we make that we did not test.
3. Nondeterminism. What nondeterminism could have entered (time, randomness, concurrency, HashMap iteration, floats).
4. Security and privacy. What security or privacy risk did we create (secrets, tokens, PII).
5. Performance cliffs. What performance cliff did we create (burst load, disk stall, huge payload, unbounded allocation).

---

## LANDING THE PLANE (session completion)

When ending a work session, you MUST complete ALL steps below. Work is NOT complete until changes are committed.

### Mandatory workflow

1. Run quality gates (if code changed).
2. Run UBS on staged files: `ubs $(git diff --name-only --cached)`. Fix until exit 0.
3. Update beads: close finished work, update in-progress items.
4. Risk assessment: append to `docs/RISK_REGISTER.md` if a bead was completed.
5. Sync beads: `br sync --flush-only`.
6. Claim-to-diff check (required): every "fixed/changed X" statement in your commit message and handoff must map to an actual staged diff hunk in the touched files.
7. Commit all changes:

```bash
git status
git add -A
git commit -m "<bead-id>: <summary>

<what changed and why>"
```

8. Produce the handoff note (template below).
9. Release Agent Mail file reservations (if multi-agent session).

### Handoff note template

```markdown
## Handoff: {bead ID} · {bead name}

### What changed
- {file}: {what and why}

### Invariants touched
- {list which invariants were relevant. Use I1..I5 from PLANS.md}

### Tests added or updated
- {test name}: {what it asserts}

### Tour artifacts
- {affected or not affected}. If affected: {what changed and why}

### Risk assessment
- Added to `docs/RISK_REGISTER.md`: {summary}

### Open questions
- {anything the next agent needs to decide}
```

If you have no open questions, write "None". Do not omit the section.

### Reviewer checklist (hygiene)

Before approving or handing off:
- For each claim line ("fixed X", "added Y", "removed Z"), point to at least one corresponding diff hunk.
- If a claim has no diff evidence, either revise the claim text or add the missing change.
- Keep commit body and handoff sections consistent with the actual staged files.

### Critical rules

- Work is NOT complete until `git commit` succeeds.
- Never stop mid-bead without committing what you have and noting status.
- Never say "ready to commit when you are." YOU must commit.
- If commit fails, resolve and retry.
