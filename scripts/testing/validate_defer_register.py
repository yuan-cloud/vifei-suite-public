#!/usr/bin/env python3
"""Validate deferred coverage waiver ledger entries.

Usage:
  python3 scripts/testing/validate_defer_register.py docs/testing/defer-register-v0.1.json
"""

from __future__ import annotations

import json
import re
import sys
from datetime import date
from pathlib import Path

RE_DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
RISK_LEVELS = {"P0", "P1", "P2", "P3"}
REQUIRED_ENTRY_FIELDS = {
    "id",
    "surface",
    "risk_rank",
    "owner",
    "rationale",
    "compensating_controls",
    "created_on",
    "revisit_on",
    "expires_on",
    "status",
    "linked_beads",
}


def parse_iso_date(raw: str, field: str, errors: list[str], entry_id: str) -> date | None:
    if not isinstance(raw, str) or not RE_DATE.match(raw):
        errors.append(f"{entry_id}: {field} must be YYYY-MM-DD")
        return None
    try:
        return date.fromisoformat(raw)
    except ValueError:
        errors.append(f"{entry_id}: {field} is not a valid date")
        return None


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: validate_defer_register.py <path-to-ledger.json>", file=sys.stderr)
        return 2

    ledger_path = Path(sys.argv[1])
    if not ledger_path.exists():
        print(f"error: ledger file not found: {ledger_path}", file=sys.stderr)
        return 2

    try:
        with ledger_path.open("r", encoding="utf-8") as handle:
            data = json.load(handle)
    except json.JSONDecodeError as exc:
        print(f"error: invalid JSON at {ledger_path}: {exc}", file=sys.stderr)
        return 2

    errors: list[str] = []

    if not isinstance(data, dict):
        print("error: top-level JSON must be an object", file=sys.stderr)
        return 2

    entries = data.get("entries")
    if not isinstance(entries, list):
        print("error: top-level 'entries' must be an array", file=sys.stderr)
        return 2

    seen_ids: set[str] = set()
    today = date.today()

    for index, entry in enumerate(entries):
        if not isinstance(entry, dict):
            errors.append(f"entry[{index}] must be an object")
            continue

        entry_id = entry.get("id", f"entry[{index}]")

        missing = sorted(REQUIRED_ENTRY_FIELDS.difference(entry.keys()))
        if missing:
            errors.append(f"{entry_id}: missing required fields: {', '.join(missing)}")
            continue

        if not isinstance(entry["id"], str) or not entry["id"].strip():
            errors.append(f"entry[{index}]: id must be a non-empty string")
        elif entry["id"] in seen_ids:
            errors.append(f"{entry['id']}: duplicate id")
        else:
            seen_ids.add(entry["id"])

        if entry.get("risk_rank") not in RISK_LEVELS:
            errors.append(f"{entry_id}: risk_rank must be one of {sorted(RISK_LEVELS)}")

        for field in ("surface", "owner", "rationale", "status"):
            value = entry.get(field)
            if not isinstance(value, str) or not value.strip():
                errors.append(f"{entry_id}: {field} must be a non-empty string")

        controls = entry.get("compensating_controls")
        if not isinstance(controls, list) or not controls:
            errors.append(f"{entry_id}: compensating_controls must be a non-empty array")
        else:
            if any(not isinstance(item, str) or not item.strip() for item in controls):
                errors.append(f"{entry_id}: compensating_controls must contain non-empty strings")

        linked = entry.get("linked_beads")
        if not isinstance(linked, list) or not linked:
            errors.append(f"{entry_id}: linked_beads must be a non-empty array")
        else:
            if any(not isinstance(item, str) or not item.strip() for item in linked):
                errors.append(f"{entry_id}: linked_beads must contain non-empty strings")

        created_on = parse_iso_date(entry.get("created_on"), "created_on", errors, entry_id)
        revisit_on = parse_iso_date(entry.get("revisit_on"), "revisit_on", errors, entry_id)
        expires_on = parse_iso_date(entry.get("expires_on"), "expires_on", errors, entry_id)

        if created_on and revisit_on and revisit_on < created_on:
            errors.append(f"{entry_id}: revisit_on must be on/after created_on")

        if expires_on and expires_on < today:
            errors.append(
                f"{entry_id}: expires_on={expires_on.isoformat()} is expired (today={today.isoformat()})"
            )

    if errors:
        print("defer-register validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(f"defer-register validation passed ({len(entries)} entries)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
