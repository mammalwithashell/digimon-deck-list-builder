#!/usr/bin/env python3
"""Report whether the project's canonical rules manual is still the current one.

The committed derivations under ``docs/digimon-rules/`` are stamped with the
Comprehensive Rules Manual version they were verified against
(``rules-index.json`` -> ``source.version`` / ``source.last_updated``, mirrored
by the header of ``docs/digimon-rules/cr-4.3.txt``). This script reads that
stamp and compares it with the header of the *live* manual:

1. the base repo's ``Digimon TCG resources/general_rule.pdf`` (git-ignored,
   base-only; rendered with poppler's ``pdftotext -raw``) when present, and/or
2. the official PDF at https://world.digimoncard.com/rule/pdf/general_rule.pdf
   when ``--online`` is passed (downloads to a temp dir, never into the repo).

Exit codes: 0 = stamps agree, 1 = drift (the manual is newer / different),
2 = could not determine (no pdftotext, no PDF, network failure). Report-only:
this is NOT wired into a failing CI tier yet (Phase 0 of the rules-conformance
campaign, docs/superpowers/specs/2026-10-15-engine-rules-conformance-validation-design.md).

Standard library only. Usage::

    python code/tools/rules/check_rules_freshness.py            # local PDF
    python code/tools/rules/check_rules_freshness.py --online   # + official site
    python code/tools/rules/check_rules_freshness.py --pdftotext C:/path/to/pdftotext.exe
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path

OFFICIAL_URL = "https://world.digimoncard.com/rule/pdf/general_rule.pdf"
VERSION_RE = re.compile(r"Ver\.\s*(\d+\.\d+)")
DATE_RE = re.compile(r"Last Updated:\s*(\d{4}/\d{2}/\d{2})")


def repo_root() -> Path:
    return Path(__file__).resolve().parents[3]


def base_repo_root(root: Path) -> Path:
    """The main worktree's root (the PDFs live only there; CLAUDE.md rule 32)."""
    try:
        common = subprocess.run(
            ["git", "rev-parse", "--path-format=absolute", "--git-common-dir"],
            cwd=root, capture_output=True, text=True, check=True,
        ).stdout.strip()
        return Path(common).parent
    except Exception:
        return root


def committed_stamp(root: Path) -> tuple[str, str]:
    idx = json.loads((root / "docs/digimon-rules/rules-index.json").read_text(encoding="utf-8"))
    src = idx["source"]
    version = VERSION_RE.search(src["version"]).group(1)
    return version, src["last_updated"]


def text_copy_stamp(root: Path) -> tuple[str, str] | None:
    """Header stamp of the committed pdftotext copy, if present."""
    copies = sorted((root / "docs/digimon-rules").glob("cr-*.txt"))
    if not copies:
        return None
    head = copies[-1].read_text(encoding="utf-8", errors="replace")[:600]
    return parse_header(head)


def parse_header(text: str) -> tuple[str, str] | None:
    v = VERSION_RE.search(text)
    d = DATE_RE.search(text)
    if not v or not d:
        return None
    return v.group(1), d.group(1)


def find_pdftotext(explicit: str | None) -> str | None:
    if explicit:
        return explicit if Path(explicit).exists() else None
    found = shutil.which("pdftotext")
    if found:
        return found
    env = os.environ.get("PDFTOTEXT")
    if env and Path(env).exists():
        return env
    # Known local poppler drop (dev machine); harmless elsewhere.
    for cand in Path("C:/Users").glob("*/Downloads/Release-*/poppler-*/Library/bin/pdftotext.exe"):
        return str(cand)
    return None


def pdf_header(pdf: Path, pdftotext: str) -> tuple[str, str] | None:
    out = subprocess.run([pdftotext, "-raw", "-l", "1", str(pdf), "-"], capture_output=True, text=True,
                         encoding="utf-8", errors="replace")
    if out.returncode != 0:
        return None
    return parse_header(out.stdout)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--online", action="store_true", help="also fetch the official PDF header")
    ap.add_argument("--pdftotext", help="path to poppler's pdftotext")
    ap.add_argument("--pdf", help="explicit PDF path (default: base repo's Digimon TCG resources/general_rule.pdf)")
    args = ap.parse_args(argv)

    root = repo_root()
    committed = committed_stamp(root)
    print(f"committed stamp : Ver. {committed[0]} ({committed[1]})  [docs/digimon-rules/rules-index.json]")
    copy = text_copy_stamp(root)
    if copy:
        print(f"text copy stamp : Ver. {copy[0]} ({copy[1]})  [docs/digimon-rules/cr-*.txt]")
        if copy != committed:
            print("DRIFT: rules-index.json and the committed text copy disagree")
            return 1

    pdftotext = find_pdftotext(args.pdftotext)
    if not pdftotext:
        print("cannot determine: pdftotext not found (install poppler or pass --pdftotext)")
        return 2

    drift = False
    checked = False
    pdf = Path(args.pdf) if args.pdf else base_repo_root(root) / "Digimon TCG resources" / "general_rule.pdf"
    if pdf.exists():
        live = pdf_header(pdf, pdftotext)
        checked = True
        if live is None:
            print(f"cannot determine: no version header in {pdf}")
            return 2
        print(f"local PDF stamp : Ver. {live[0]} ({live[1]})  [{pdf}]")
        if live != committed:
            print("DRIFT: the base repo's general_rule.pdf is not the version the derivations were verified against")
            drift = True
    else:
        print(f"local PDF       : not found at {pdf} (worktree? the PDFs live base-only, rule 32)")

    if args.online:
        try:
            with tempfile.TemporaryDirectory() as tmp:
                dest = Path(tmp) / "general_rule.pdf"
                with urllib.request.urlopen(OFFICIAL_URL, timeout=60) as resp, dest.open("wb") as fh:
                    shutil.copyfileobj(resp, fh)
                live = pdf_header(dest, pdftotext)
            checked = True
            if live is None:
                print("cannot determine: no version header in the downloaded PDF")
                return 2
            print(f"official stamp  : Ver. {live[0]} ({live[1]})  [{OFFICIAL_URL}]")
            if live != committed:
                print("DRIFT: Bandai has published a newer Comprehensive Rules Manual; re-run the Phase 0 refresh")
                drift = True
        except Exception as exc:  # network / TLS / HTTP
            print(f"cannot determine: online check failed ({exc})")
            if not checked:
                return 2

    if not checked:
        return 2
    if drift:
        return 1
    print("OK: canonical rules source is current")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
