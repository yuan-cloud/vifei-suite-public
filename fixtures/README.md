# Test Fixtures

## small-session.jsonl

- **Source:** Synthetically generated Agent Cassette session for testing.
  Represents a typical short Claude Code session: read a file, edit it,
  run tests, encounter an error, write a file, end session.
- **Redaction status:** Fully synthetic. No real secrets, API keys, or PII.
  All file paths, outputs, and content are fabricated test data.
- **Event type coverage:** session_start (RunStart), session_end (RunEnd),
  tool_use (ToolCall), tool_result (ToolResult), error (Error).
  Missing from this fixture: PolicyDecision, RedactionApplied,
  ClockSkewDetected (these are system-generated, not source events).
- **Event count:** 11 events.
- **Known limitations:** Does not cover multi-source scenarios, large
  payloads requiring blobbing, or clock skew conditions.
- **License:** Public domain (synthetic test data).
