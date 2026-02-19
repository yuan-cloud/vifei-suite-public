#!/usr/bin/env python3
"""Fail-closed secret hygiene scanner for demo/media outputs."""

from __future__ import annotations

import argparse
import os
import re
import sys
from dataclasses import dataclass
from pathlib import Path


TEXT_EXTENSIONS = {
    ".txt",
    ".json",
    ".jsonl",
    ".md",
    ".log",
    ".cast",
    ".yaml",
    ".yml",
    ".toml",
    ".csv",
}

MAX_SCAN_BYTES = 5 * 1024 * 1024

PATTERNS: list[tuple[str, re.Pattern[str]]] = [
    ("openai_key", re.compile(r"sk-[A-Za-z0-9]{20,}")),
    ("github_pat", re.compile(r"(?:ghp|github_pat)_[A-Za-z0-9_]{20,}")),
    ("aws_access_key", re.compile(r"AKIA[0-9A-Z]{16}")),
    ("private_key", re.compile(r"-----BEGIN (?:[A-Z ]+)?PRIVATE KEY-----")),
    ("bearer_token", re.compile(r"Bearer [A-Za-z0-9._-]{20,}")),
    ("jwt", re.compile(r"eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}")),
    ("slack_token", re.compile(r"xox[baprs]-[A-Za-z0-9-]{10,}")),
]


@dataclass(frozen=True)
class Finding:
    pattern_id: str
    path: Path
    line_no: int
    snippet: str

    def render(self) -> str:
        return f"{self.pattern_id}: {self.path}:{self.line_no}: {self.snippet}"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Check media/demo outputs for secret-like tokens.",
    )
    parser.add_argument(
        "targets",
        nargs="+",
        help="Files or directories to scan.",
    )
    parser.add_argument(
        "--allowlist",
        default="scripts/testing/media_hygiene_allowlist.txt",
        help="Path to regex allowlist file for known synthetic tokens.",
    )
    parser.add_argument(
        "--allow-unsafe",
        action="store_true",
        help="Do not fail non-zero on findings (still prints all findings).",
    )
    return parser.parse_args()


def load_allowlist(path: Path) -> list[re.Pattern[str]]:
    if not path.exists():
        return []
    patterns: list[re.Pattern[str]] = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        patterns.append(re.compile(line))
    return patterns


def iter_files(targets: list[str]) -> list[Path]:
    files: list[Path] = []
    for target in targets:
        path = Path(target)
        if not path.exists():
            continue
        if path.is_file():
            files.append(path)
            continue
        for nested in path.rglob("*"):
            if nested.is_file():
                files.append(nested)
    return files


def is_text_candidate(path: Path) -> bool:
    if path.suffix.lower() in TEXT_EXTENSIONS:
        return True
    return path.name.endswith(".capture")


def line_is_allowlisted(line: str, allow_patterns: list[re.Pattern[str]]) -> bool:
    return any(pattern.search(line) for pattern in allow_patterns)


def scan_file(path: Path, allow_patterns: list[re.Pattern[str]]) -> list[Finding]:
    if not is_text_candidate(path):
        return []
    if path.stat().st_size > MAX_SCAN_BYTES:
        return []

    findings: list[Finding] = []
    text = path.read_text(encoding="utf-8", errors="ignore")
    for i, line in enumerate(text.splitlines(), start=1):
        for pattern_id, pattern in PATTERNS:
            if not pattern.search(line):
                continue
            rendered = f"{pattern_id}|{path}:{i}|{line}"
            if line_is_allowlisted(rendered, allow_patterns):
                continue
            snippet = line.strip()
            if len(snippet) > 180:
                snippet = snippet[:180] + "..."
            findings.append(
                Finding(
                    pattern_id=pattern_id,
                    path=path,
                    line_no=i,
                    snippet=snippet,
                )
            )
    return findings


def main() -> int:
    args = parse_args()
    allowlist_path = Path(args.allowlist)
    allow_patterns = load_allowlist(allowlist_path)
    files = iter_files(args.targets)

    findings: list[Finding] = []
    for path in files:
        findings.extend(scan_file(path, allow_patterns))

    if findings:
        print("[hygiene] FAIL: potential secret-like content detected", file=sys.stderr)
        for finding in findings:
            print(f"  - {finding.render()}", file=sys.stderr)
        print(
            "[hygiene] remediation: remove secret-like content, or add a precise allowlist regex "
            f"in {allowlist_path} for known synthetic fixtures",
            file=sys.stderr,
        )
        if args.allow_unsafe or os.getenv("VIFEI_HYGIENE_ALLOW_UNSAFE") == "1":
            # maintain explicit operator acknowledgment path for emergency usage
            print(
                "[hygiene] WARNING: unsafe override enabled; continuing despite findings",
                file=sys.stderr,
            )
            return 0
        return 1

    print(f"[hygiene] PASS: scanned {len(files)} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
