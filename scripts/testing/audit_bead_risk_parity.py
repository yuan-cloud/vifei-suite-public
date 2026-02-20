#!/usr/bin/env python3
"""
Audit parity between closed beads and risk-register evidence headings.

Outputs:
  - JSON report with classification for every closed bead
  - Optional markdown summary with taxonomy + remediation plan
"""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import cast


MILESTONE_RE = re.compile(r"\b(M\d+(?:\.\d+)?)\b")
RISK_HEADING_RE = re.compile(r"^##\s+(.+)$")


@dataclass(frozen=True)
class Classification:
    code: str
    severity: str
    rationale: str


CLASSIFICATIONS: dict[str, Classification] = {
    "covered_exact": Classification(
        code="covered_exact",
        severity="ok",
        rationale="Exact bead heading found in risk register.",
    ),
    "covered_milestone_alias": Classification(
        code="covered_milestone_alias",
        severity="ok",
        rationale=(
            "No exact bead heading, but milestone heading exists (legacy style). "
            "Counts as covered for transitional compatibility."
        ),
    ),
    "covered_parent_rollup": Classification(
        code="covered_parent_rollup",
        severity="ok",
        rationale=(
            "Feature/program parent has child beads and all children are covered; "
            "parent bead may remain rollup-only."
        ),
    ),
    "exempt_program_meta": Classification(
        code="exempt_program_meta",
        severity="info",
        rationale="Program/track meta bead intentionally exempt from per-bead risk entry.",
    ),
    "gap_requires_entry": Classification(
        code="gap_requires_entry",
        severity="action",
        rationale="Closed bead lacks acceptable evidence mapping and needs risk-register action.",
    ),
}


def load_issues(path: Path) -> list[dict[str, object]]:
    issues: list[dict[str, object]] = []
    decoder = json.JSONDecoder()
    for idx, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        stripped = line.strip()
        if stripped:
            try:
                parsed = decoder.decode(stripped)
            except json.JSONDecodeError as exc:
                raise ValueError(
                    f"Invalid JSONL at {path}:{idx}: {exc.msg}"
                ) from exc
            issues.append(cast(dict[str, object], parsed))
    return issues


def load_risk_headings(path: Path) -> list[str]:
    headings: list[str] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        m = RISK_HEADING_RE.match(line)
        if m:
            headings.append(m.group(1))
    return headings


def milestone_tokens(title: str) -> list[str]:
    return sorted(set(MILESTONE_RE.findall(title)))


def has_exact_heading(risk_text: str, bead_id: str) -> bool:
    return f"## {bead_id} " in risk_text


def has_milestone_heading(headings: list[str], token: str) -> bool:
    for heading in headings:
        if heading.startswith(f"{token} "):
            return True
    return False


def classify_issue(
    issue: dict[str, object],
    risk_text: str,
    risk_headings: list[str],
    children_by_parent: dict[str, list[str]],
    covered_lookup: dict[str, bool],
) -> tuple[str, list[str]]:
    issue_id = issue["id"]
    title = issue.get("title", "")
    issue_type = issue.get("issue_type", "")

    if has_exact_heading(risk_text, issue_id):
        return "covered_exact", []

    milestone_hits: list[str] = []
    for token in milestone_tokens(title):
        base = token.split(".")[0]
        if has_milestone_heading(risk_headings, token):
            milestone_hits.append(token)
        elif has_milestone_heading(risk_headings, base):
            milestone_hits.append(base)
    if milestone_hits:
        return "covered_milestone_alias", sorted(set(milestone_hits))

    if title.startswith("PROGRAM:") or title.startswith("TRACK-"):
        return "exempt_program_meta", []

    children = sorted(children_by_parent.get(issue_id, []))
    if issue_type == "feature" and children:
        if all(covered_lookup.get(child, False) for child in children):
            return "covered_parent_rollup", children

    return "gap_requires_entry", []


def render_markdown(report: dict[str, object]) -> str:
    counts = report["summary"]["classification_counts"]
    actions = report["summary"]["action_required"]
    top_gap_ids = [row["id"] for row in report["rows"] if row["classification"] == "gap_requires_entry"][:30]
    lines = [
        "# Bead Closure vs Risk Register Parity Audit",
        "",
        f"Generated at: `{report['generated_at']}`",
        "",
        "## Taxonomy",
        "",
        "- `covered_exact`: exact `## <bead-id>` risk heading exists.",
        "- `covered_milestone_alias`: milestone heading (for example `M4`, `M5.1`) covers legacy bead naming.",
        "- `covered_parent_rollup`: parent/feature bead can rely on fully-covered child evidence.",
        "- `exempt_program_meta`: program/track orchestration bead exempt from per-bead risk entry.",
        "- `gap_requires_entry`: no accepted evidence mapping; requires remediation.",
        "",
        "## Summary",
        "",
        f"- Total closed beads audited: `{report['summary']['closed_total']}`",
        f"- Action-required gaps: `{actions}`",
        "",
        "Classification counts:",
    ]
    for key in sorted(counts):
        lines.append(f"- `{key}`: `{counts[key]}`")
    lines.extend(
        [
            "",
            "## Remediation Plan",
            "",
            "1. Add CI guard for non-exempt closed beads (`A2`).",
            "2. Backfill historical `gap_requires_entry` beads or apply explicit exemption rationale (`A3`).",
            "3. Keep parent/feature rollups auditable by preserving child evidence links.",
            "",
            "## Initial Action Queue (gap IDs)",
            "",
        ]
    )
    if not top_gap_ids:
        lines.append("- None")
    else:
        for bead_id in top_gap_ids:
            lines.append(f"- `{bead_id}`")
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Audit closed-bead/risk-register evidence parity")
    parser.add_argument(
        "--issues-jsonl",
        default=".beads/issues.jsonl",
        help="Path to beads issues JSONL",
    )
    parser.add_argument(
        "--risk-register",
        default="docs/RISK_REGISTER.md",
        help="Path to risk register markdown",
    )
    parser.add_argument(
        "--output-json",
        required=True,
        help="Output JSON report path",
    )
    parser.add_argument(
        "--output-markdown",
        help="Optional markdown summary output path",
    )
    args = parser.parse_args()

    issues_path = Path(args.issues_jsonl)
    risk_path = Path(args.risk_register)
    output_json = Path(args.output_json)
    output_md = Path(args.output_markdown) if args.output_markdown else None

    issues = load_issues(issues_path)
    risk_text = risk_path.read_text(encoding="utf-8")
    headings = load_risk_headings(risk_path)

    closed = sorted(
        (i for i in issues if i.get("status") == "closed"),
        key=lambda x: str(x["id"]),
    )

    children_by_parent: dict[str, list[str]] = defaultdict(list)
    for issue in issues:
        for dep in issue.get("dependencies", []):
            if dep.get("type") == "parent-child":
                parent = dep.get("depends_on_id")
                if parent:
                    children_by_parent[parent].append(issue["id"])
    for key in list(children_by_parent):
        children_by_parent[key] = sorted(set(children_by_parent[key]))

    # First pass computes direct coverage only (exact + milestone alias + meta exempt)
    precovered: dict[str, bool] = {}
    for issue in closed:
        issue_id = issue["id"]
        title = issue.get("title", "")
        exact = has_exact_heading(risk_text, issue_id)
        tokens = milestone_tokens(title)
        milestone_hits: list[str] = []
        if not exact:
            for token in tokens:
                base = token.split(".")[0]
                if has_milestone_heading(headings, token):
                    milestone_hits.append(token)
                elif has_milestone_heading(headings, base):
                    milestone_hits.append(base)
        is_meta = title.startswith("PROGRAM:") or title.startswith("TRACK-")
        covered = exact or bool(milestone_hits) or is_meta
        precovered[issue_id] = covered

    # Second pass includes parent rollup check using stage1 coverage of children.
    rows: list[dict[str, object]] = []
    for issue in closed:
        issue_id = issue["id"]
        classification, evidence_refs = classify_issue(
            issue=issue,
            risk_text=risk_text,
            risk_headings=headings,
            children_by_parent=children_by_parent,
            covered_lookup=precovered,
        )
        covered = classification in {
            "covered_exact",
            "covered_milestone_alias",
            "covered_parent_rollup",
            "exempt_program_meta",
        }
        rows.append(
            {
                "id": issue_id,
                "title": issue.get("title", ""),
                "issue_type": issue.get("issue_type", ""),
                "classification": classification,
                "covered": covered,
                "evidence_refs": evidence_refs,
                "dependencies_parent_children": children_by_parent.get(issue_id, []),
                "close_reason": issue.get("close_reason", ""),
            }
        )

    rows = sorted(rows, key=lambda r: r["id"])
    counts = Counter(row["classification"] for row in rows)
    action_required = counts.get("gap_requires_entry", 0)

    report = {
        "generated_at": "2026-02-18T00:00:00Z",
        "inputs": {
            "issues_jsonl": str(issues_path),
            "risk_register": str(risk_path),
        },
        "taxonomy": {k: vars(v) for k, v in CLASSIFICATIONS.items()},
        "summary": {
            "closed_total": len(rows),
            "classification_counts": dict(sorted(counts.items())),
            "action_required": action_required,
        },
        "rows": rows,
    }

    output_json.parent.mkdir(parents=True, exist_ok=True)
    output_json.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if output_md:
        output_md.parent.mkdir(parents=True, exist_ok=True)
        output_md.write_text(render_markdown(report), encoding="utf-8")

    print(
        f"PARITY_AUDIT_OK closed={len(rows)} action_required={action_required} "
        f"json={output_json}"
    )
    if output_md:
        print(f"PARITY_AUDIT_OK markdown={output_md}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
