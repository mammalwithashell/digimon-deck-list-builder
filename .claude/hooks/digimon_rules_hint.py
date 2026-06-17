#!/usr/bin/env python3
"""UserPromptSubmit hook: when a prompt mentions a Digimon TCG keyword or a rule
number, point the assistant at the authoritative rules PDF (resolved in the BASE
repo, rule 29) + the exact pages, using the committed rules-index.json.

Conservative (per the user's chosen tuning): fires only on
  (1) explicit keyword names from rules-index.json `keywords`, and
  (2) rule-number patterns (e.g. 16-36) whose leading section exists.
It does NOT scan `topics` (those are for the /digimon-rules skill) and does NOT
fire on bare common verbs ("digivolve", "attack"). Common English keyword names
(Draw, Save, Training, Fragment, ...) are stored bracketed ("<Draw") in the index
so they only match the bracketed notation, keeping noise low in an RL/codebase.
To widen to the "Broad" tuning later, add "topics" to GROUPS below.

No Pinecone / network: reads the committed index and resolves the base PDF path
via git. Silent on no match or any error, exactly like digimon_card_image_hint.py.
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path

GROUPS = ("keywords",)  # add "topics" to widen to the Broad tuning
RULE_NUM_RE = re.compile(r"\b\d{1,2}-\d{1,2}(?:-\d{1,3}){0,3}\b")


def repo_root() -> Path:
    # .claude/hooks/<this>.py -> repo root (the worktree, where the index is committed)
    return Path(__file__).resolve().parents[2]


def base_resources_dir() -> str | None:
    env = os.environ.get("DIGIMON_RULES_PDF_DIR")
    if env:
        return env
    try:
        common = subprocess.check_output(
            ["git", "rev-parse", "--path-format=absolute", "--git-common-dir"],
            cwd=str(repo_root()), stderr=subprocess.DEVNULL, text=True,
        ).strip()
        return str(Path(common).parent / "Digimon TCG resources")
    except Exception:
        return None


def load_index() -> dict | None:
    try:
        return json.loads(
            (repo_root() / "docs/digimon-rules/rules-index.json").read_text(encoding="utf-8")
        )
    except Exception:
        return None


def _matches(name: str, prompt: str) -> bool:
    # Letter-only boundaries so "<Security Attack +1>" / "<Link +2>" still match,
    # while avoiding matches inside larger words.
    pat = r"(?<![A-Za-z])" + re.escape(name) + r"(?![A-Za-z])"
    return re.search(pat, prompt, re.IGNORECASE) is not None


def main() -> int:
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass
    try:
        payload = json.load(sys.stdin)
    except Exception:
        return 0
    prompt = str(payload.get("prompt", "") or "")
    if not prompt:
        return 0
    idx = load_index()
    if not idx:
        return 0

    base = base_resources_dir()
    sections = idx.get("sections", {})
    hits: list[str] = []
    seen: set[str] = set()

    def path_for(pdf: str) -> str:
        return f"{base}/{pdf}" if base else f"(base repo)/Digimon TCG resources/{pdf}"

    # (1) Keyword-name matches.
    for grp in GROUPS:
        for key, entry in idx.get(grp, {}).items():
            if key in seen:
                continue
            for name in entry.get("names", []):
                if _matches(name, prompt):
                    seen.add(key)
                    pdf = entry.get("pdf", "general_rule.pdf")
                    pages = entry.get("pages", "")
                    hits.append(
                        f'  - "{name.lstrip("<")}" (§{entry.get("section", "?")}): '
                        f"{pdf} p.{pages} -> {path_for(pdf)}"
                    )
                    break

    # (2) Rule-number matches -> enclosing section.
    for m in RULE_NUM_RE.finditer(prompt):
        num = m.group(0)
        sec = num.split("-", 1)[0]
        tag = f"rule:{num}"
        if sec in sections and tag not in seen:
            seen.add(tag)
            s = sections[sec]
            pdf = s.get("pdf", "general_rule.pdf")
            hits.append(
                f'  - Rule {num}: §{sec} {s.get("title", "")}, '
                f'{pdf} pp.{s.get("pages", "")} -> {path_for(pdf)}'
            )

    if not hits:
        return 0

    print(
        "[digimon-rules] This prompt touches Digimon TCG rules. The authoritative source "
        "is the Comprehensive Rules Manual (general_rule.pdf) — read the cited pages with "
        "the Read tool before reasoning about the rule. Cheap quick-reference: "
        "docs/digimon-rules/keyword-semantics.md; full verified digest via `/digimon-rules deep`."
    )
    print("\n".join(hits))
    return 0


if __name__ == "__main__":
    sys.exit(main())
