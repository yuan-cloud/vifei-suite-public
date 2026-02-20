# Workspace Hygiene Guide

Goal: avoid accidental nested-project pushes and keep multi-agent work easy to reason about.

## 1. Root Sanity (run first)

```bash
pwd
git rev-parse --show-toplevel
git status -sb
```

Expected: `pwd` and `git rev-parse --show-toplevel` should point to the same repo root.

## 2. Nested Repo Guard (must be empty)

```bash
find . -mindepth 2 -type d -name .git -print
```

Expected result: no output.

If anything prints, stop and fix it before committing or pushing.

## 3. Structure Quick Scan

```bash
find . -maxdepth 2 -type d | sort
```

Expected top-level domains in this repo:
- `.beads`, `.github`, `crates`, `docs`, `fixtures`, plus standard local directories.

## 4. Commit Safety

```bash
git status --short
git add <explicit-files-only>
ubs --staged
cargo test
git commit -m "<scope>: <summary>"
```

When multiple agents are active, stage explicit files only.

Before committing, run a quick claim-to-diff check:

```bash
git diff --cached --stat
```

Then verify each claimed "fixed/changed" item in your commit message maps to at least one staged diff hunk.

## 5. Push Gate

```bash
git status --short
git push
```

Before pushing, confirm `git status --short` matches your intended change set.

## 6. Daily Learning Loop

At session start:
1. Re-read `AGENTS.md`, `PLANS.md`, constitution docs, and `docs/guides/*`.
2. Check agent mail before claiming work.
3. Confirm bead ID + role + file reservation before edits.

At session end:
1. Post handoff with commit SHA and exact files changed.
2. Add risk note when bead is completed.
3. Sync bead state.

## 7. One-Liner Guard

```bash
git rev-parse --show-toplevel && find . -mindepth 2 -type d -name .git -print && git status -sb
```

If nested `.git` appears, do not push.
