#!/usr/bin/env python3
"""Fail CI when closed non-exempt beads lack risk-register evidence mapping."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path


def load_json(path: Path) -> dict[str, object]:
    try:
        with path.open("r", encoding="utf-8") as handle:
            data = json.load(handle)
    except FileNotFoundError:
        raise SystemExit(f"error: file not found: {path}")
    except json.JSONDecodeError as exc:
        raise SystemExit(f"error: invalid JSON in {path}: {exc}")
    if not isinstance(data, dict):
        raise SystemExit(f"error: expected JSON object in {path}")
    return data


def load_exemptions(path: Path) -> dict[str, str]:
    data = load_json(path)
    rows = data.get("exempt_gap_beads")
    if not isinstance(rows, list):
        raise SystemExit("error: exemptions JSON must contain array field 'exempt_gap_beads'")

    exemptions: dict[str, str] = {}
    for idx, row in enumerate(rows):
        if not isinstance(row, dict):
            raise SystemExit(f"error: exemption row[{idx}] must be an object")
        bead_id = row.get("id")
        rationale = row.get("rationale")
        if not isinstance(bead_id, str) or not bead_id.strip():
            raise SystemExit(f"error: exemption row[{idx}] has invalid id")
        if not isinstance(rationale, str) or not rationale.strip():
            raise SystemExit(f"error: exemption row[{idx}] has invalid rationale")
        if bead_id in exemptions:
            raise SystemExit(f"error: duplicate exemption id: {bead_id}")
        exemptions[bead_id] = rationale.strip()
    return exemptions


def run_parity_audit(
    python_bin: str,
    audit_script: Path,
    issues_jsonl: Path,
    risk_register: Path,
    output_json: Path,
    output_markdown: Path | None,
) -> None:
    cmd = [
        python_bin,
        str(audit_script),
        "--issues-jsonl",
        str(issues_jsonl),
        "--risk-register",
        str(risk_register),
        "--output-json",
        str(output_json),
    ]
    if output_markdown is not None:
        cmd.extend(["--output-markdown", str(output_markdown)])

    result = subprocess.run(cmd, check=False, capture_output=True, text=True)
    if result.stdout:
        print(result.stdout.strip())
    if result.returncode != 0:
        if result.stderr:
            print(result.stderr.strip(), file=sys.stderr)
        raise SystemExit(result.returncode)


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Run closed-bead parity audit and fail when unresolved non-exempt gaps remain."
        )
    )
    parser.add_argument("--python-bin", default=sys.executable, help="Python executable for audit")
    parser.add_argument(
        "--audit-script",
        default="scripts/testing/audit_bead_risk_parity.py",
        help="Path to parity audit script",
    )
    parser.add_argument(
        "--issues-jsonl",
        default=".beads/issues.jsonl",
        help="Path to beads JSONL tracker",
    )
    parser.add_argument(
        "--risk-register",
        default="docs/RISK_REGISTER.md",
        help="Path to risk register markdown",
    )
    parser.add_argument(
        "--exemptions",
        default="docs/testing/bead-closure-evidence-exemptions-v0.1.json",
        help="Path to explicit exemption ledger",
    )
    parser.add_argument(
        "--audit-output-json",
        default=".tmp/bead-closure-evidence/bead-risk-parity-audit.json",
        help="Path for generated parity JSON report",
    )
    parser.add_argument(
        "--audit-output-markdown",
        default=".tmp/bead-closure-evidence/bead-risk-parity-audit.md",
        help="Path for generated parity markdown report",
    )
    args = parser.parse_args()

    audit_script = Path(args.audit_script)
    issues_jsonl = Path(args.issues_jsonl)
    risk_register = Path(args.risk_register)
    exemptions_path = Path(args.exemptions)
    audit_output_json = Path(args.audit_output_json)
    audit_output_markdown = Path(args.audit_output_markdown) if args.audit_output_markdown else None

    audit_output_json.parent.mkdir(parents=True, exist_ok=True)
    if audit_output_markdown is not None:
        audit_output_markdown.parent.mkdir(parents=True, exist_ok=True)

    run_parity_audit(
        python_bin=args.python_bin,
        audit_script=audit_script,
        issues_jsonl=issues_jsonl,
        risk_register=risk_register,
        output_json=audit_output_json,
        output_markdown=audit_output_markdown,
    )

    report = load_json(audit_output_json)
    rows = report.get("rows")
    if not isinstance(rows, list):
        raise SystemExit("error: parity report missing rows array")

    exemptions = load_exemptions(exemptions_path)
    gap_ids = sorted(
        row.get("id", "")
        for row in rows
        if isinstance(row, dict) and row.get("classification") == "gap_requires_entry"
    )
    gap_ids = [bead_id for bead_id in gap_ids if bead_id]

    gap_set = set(gap_ids)
    exempt_set = set(exemptions.keys())

    unresolved = sorted(gap_set - exempt_set)
    stale_exemptions = sorted(exempt_set - gap_set)

    if stale_exemptions:
        print("CLOSURE_EVIDENCE_WARN stale exemptions (remove after A3 backfill):")
        for bead_id in stale_exemptions:
            print(f"  - {bead_id}")

    if unresolved:
        print("CONTRACT_FAIL[CLOSE-001] unresolved closure-evidence gaps:")
        for bead_id in unresolved:
            print(f"  - {bead_id}")
        print(f"audit_json={audit_output_json}")
        if audit_output_markdown is not None:
            print(f"audit_markdown={audit_output_markdown}")
        print(
            "replay: python3 scripts/testing/check_bead_closure_evidence.py "
            "--audit-output-json .tmp/bead-closure-evidence/bead-risk-parity-audit.json "
            "--audit-output-markdown .tmp/bead-closure-evidence/bead-risk-parity-audit.md"
        )
        print("fix: add risk-register entry or explicit exemption rationale per policy")
        return 1

    print(
        "CLOSURE_EVIDENCE_OK "
        f"gaps={len(gap_ids)} covered_by_exemptions={len(gap_set & exempt_set)} "
        f"unresolved=0"
    )
    if audit_output_markdown is not None:
        print(f"CLOSURE_EVIDENCE_OK markdown={audit_output_markdown}")
    print(f"CLOSURE_EVIDENCE_OK json={audit_output_json}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
