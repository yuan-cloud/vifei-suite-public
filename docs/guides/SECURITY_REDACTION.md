# Security and Redaction Guide

Share-safe export (invariant I3) refuses when secrets remain. This guide
covers the M8 secret scanning and redaction patterns.

---

## Principle: redaction-first export

Export is not "bundle then check." It is "scan, refuse if dirty, bundle
only when clean."

```text
EventLog + blobs
  │
  ▼
Secret scanner  ──dirty──▶  refusal-report.json (exit code 3)
  │
  clean
  │
  ▼
Deterministic bundler  →  bundle.tar.zst + bundle_hash
```

---

## Secret detection patterns

### High-entropy strings

Shannon entropy above 4.5 on base64-like strings (length >= 20) is a
strong signal. Compute on candidate strings after stripping whitespace.

```rust
fn shannon_entropy(s: &str) -> f64 {
    let mut freq = [0u32; 256];
    for b in s.bytes() { freq[b as usize] += 1; }
    let len = s.len() as f64;
    freq.iter()
        .filter(|&&c| c > 0)
        .map(|&c| { let p = c as f64 / len; -p * p.log2() })
        .sum()
}
```

### Known API key patterns

| Pattern | Regex | Priority |
|---------|-------|----------|
| AWS access key | `AKIA[0-9A-Z]{16}` | High |
| GitHub token | `gh[ps]_[A-Za-z0-9]{36,}` | High |
| GitHub OAuth | `gho_[A-Za-z0-9]{36,}` | High |
| JWT | `eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}` | High |
| Generic secret key | `sk-[A-Za-z0-9]{20,}` | Medium |
| Generic public key | `pk-[A-Za-z0-9]{20,}` | Low |
| Bearer token | `Bearer\s+[A-Za-z0-9._\-]{20,}` | Medium |
| Private key header | `-----BEGIN .* PRIVATE KEY-----` | High |

### Where to scan

1. **Event payloads inline:** `ToolCall::args`, `ToolResult::result`,
   `Error::message`, `Generic::data` values.
2. **Blob contents:** Read each blob referenced by `payload_ref` and scan.
3. **Event metadata:** `run_id`, `event_id`, `source_id` (unlikely but
   defense in depth).

---

## Deterministic masking

Replace matched content with a fixed-length placeholder:

```
[REDACTED:aws_key]
[REDACTED:github_token]
[REDACTED:high_entropy]
```

**Masking must be deterministic:** same input always produces the same
masked output. This is required for `bundle_hash` stability.

Rules:
- Placeholder includes pattern name for debuggability
- Fixed length per pattern (not dependent on original content length)
- Applied in-place in the redacted copy, not in the original EventLog
  (truth is never modified)

---

## Refusal report

When the scanner finds secrets, export emits `refusal-report.json` and
exits with code 3. The schema is defined in `PLANS.md` § "Artifact
schema contracts" — reference it, do not duplicate it here.

Key fields per blocked item:
- `event_id` — which event contains the secret
- `field_path` — dot-delimited path (e.g., `"payload.args"`)
- `matched_pattern` — pattern name or regex that triggered the block
- `blob_ref` — optional, if the secret was in a blob

---

## Failure mode: FM-EXPORT-UNSAFE

Defined in `docs/BACKPRESSURE_POLICY.md` § "Failure modes v0.1". When
secret detection triggers during export:

- Refuse export
- Emit refusal report
- Do **not** change the ingest ladder level (this is an export-only concern)

---

## Implementation checklist for M8

1. Define pattern set as a configurable rule list (Vec of compiled regex)
2. Implement scanner that walks EventLog + blobs
3. On finding: collect into `blocked_items` list
4. If any blocked: write `refusal-report.json`, return error
5. If clean: proceed to deterministic bundling
6. Test with secret-seeded fixture
7. Test with clean fixture (verify bundle_hash stability)

---

## What NOT to do

- Do not modify the original EventLog during scanning (truth is immutable)
- Do not silently skip secrets — every match must appear in the refusal report
- Do not use heuristics alone — combine entropy + regex for precision
- Do not scan at import time in v0.1 — scanning happens at export time only
- Do not add `security_meta` annotations to events in v0.1 (explicitly deferred)
