# Adapter Fixture Template (v0.1)

Purpose: define a repeatable fixture shape for new provider adapters so conformance tests remain consistent.

## Required fixture metadata

- `adapter_name`: short stable id (for example `openai-responses`)
- `schema_version`: source schema version (if provided by source format)
- `fixture_id`: deterministic identifier used in test names
- `notes`: optional context for edge-case intent

## Required behavior checks per fixture

1. Source order preserved (no timestamp sorting).
2. `commit_index` rejected if source tries to provide it.
3. Unknown record types mapped deterministically (`Generic` or contract error by policy).
4. `synthesized` markers set for inferred fields.
5. Replay artifacts stable across reruns.

## JSONL template row

```json
{"type":"<source-type>","schema_version":"<source-schema-v1>","session_id":"<run-id>","id":"<event-id>","timestamp":"2026-02-16T10:00:01Z","payload":{}}
```

## Negative-case template row (`commit_index` violation)

```json
{"type":"<source-type>","session_id":"<run-id>","commit_index":7}
```

Expected result: Tier A `Error` with `kind="contract"` and message mentioning forbidden `commit_index`.
