Read these files in order BEFORE writing anything:
1. AGENTS.md
2. PLANS.md
3. docs/BACKPRESSURE_POLICY.md
4. docs/CAPACITY_ENVELOPE.md
5. docs/RESEARCH_REFERENCE.md (research compiled today Feb 16 2026 with latest crate versions and official Anthropic best practices)

Then create a docs/guides/ directory with best practices guides for this project. Use ultrathink.

Create these 7 guides:

1. ARCHITECTURE.md - High-level system architecture, crate layout, data flow from ingest to TUI, truth taxonomy. Include a BLAKE3 content addressing section covering blob store patterns and digest computation using the blake3 v1.8.3 API from RESEARCH_REFERENCE.md. This is the first thing a new agent reads after AGENTS.md.

2. RUST_SERDE_DETERMINISM.md - Byte-stable serialization, canonical JSON, field ordering via derive macros, BTreeMap not HashMap, no serde_json::Value on disk. Use the concrete serde patterns and version info from RESEARCH_REFERENCE.md. Show examples from crates/vifei-core/src/event.rs (the M1 code already written).

3. SECURITY_REDACTION.md - Secret scanning implementation patterns, entropy detection, known API key patterns, deterministic masking, export refusal workflow. Use the patterns from RESEARCH_REFERENCE.md §9. Reference (don't duplicate) the refusal report schema from PLANS.md.

4. TESTING_DETERMINISM.md - Round-trip byte stability tests, snapshot testing, state_hash verification, float determinism with Ryu, how to write hash stability assertions. Reference the M1 tests already in event.rs as examples.

5. TUI_RATATUI.md - Ratatui v0.30.0 patterns (modular workspace, HorizontalAlignment rename, MSRV 1.86.0, ratatui::run()). Lens architecture (Incident/Forensic), Truth HUD rendering, TestBackend snapshot testing. Use ONLY the v0.30.0 API from RESEARCH_REFERENCE.md §6 — do NOT use deprecated APIs like Frame::size().

6. CLI_DESIGN.md - Clap v4 patterns with derive API. Use the exact subcommand structure from RESEARCH_REFERENCE.md §8 as a starting template. Exit codes, UX conventions, hero loop mapping.

7. RUST_ERROR_HANDLING.md - thiserror v1.6.0 for library crates, anyhow v1.1.0 for binaries. FM-* failure mode mapping to thiserror variants. Use the Vifei-specific error enum example from RESEARCH_REFERENCE.md §7. Never .unwrap() in library code.

Rules:
- Do NOT duplicate content from BACKPRESSURE_POLICY.md or CAPACITY_ENVELOPE.md — link to them instead
- Do NOT create guides for append-only eventlog or backpressure patterns — those are already fully covered in PLANS.md and the constitution docs
- Make each guide project-specific with Vifei examples drawn from M1 and M2 code
- Keep each guide concise (under 200 lines) and actionable — rules first, rationale second
- Pin crate versions from RESEARCH_REFERENCE.md in every guide that mentions a crate
- Reference Anthropic's official agent team lessons (RESEARCH_REFERENCE.md §3 and §10) in ARCHITECTURE.md and TESTING_DETERMINISM.md where relevant
