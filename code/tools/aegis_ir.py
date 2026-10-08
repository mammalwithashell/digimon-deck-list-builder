#!/usr/bin/env python3
"""Aegis compiled-effect IR — a LOW-trust, third-party structural hint per card.

Aegis (https://github.com/vinicius3333/aegis-digimon-tcg, MIT) compiles each
card's printed text into an IR record: trigger, an ordered ``actions[]`` list,
filters, ``optional``, ``frequency``, plus self-reported ``coverage`` /
``residual`` fields. We vendor a pinned, gzipped snapshot under
``data/third_party/aegis/`` (see its README for the commit and provenance).

Trust: BELOW printed text (card image / official bundle), DCGO C#,
``general_rule.pdf`` and ``card_overrides.json`` / ``cards.json``. Use a record
only to cross-check how a card's text decomposes into clauses. Its
``coverage`` is self-audited (every record claims ``"full"``), and its
digivolve / DNA / DigiXros / Assembly / Link / App-Fusion requirement data is
never to be copied — ``aegis_section`` strips those fields.

Usage:
  python code/tools/aegis_ir.py BT15-003 [AD1-001 ...]   # labeled context-pack section(s)
  python code/tools/aegis_ir.py --no-heading BT15-003    # body only (template carries the label)
  python code/tools/aegis_ir.py --check                   # verify the snapshot against SNAPSHOT.json
  python code/tools/aegis_ir.py refresh --clone D:/tmp_aegis/a --commit <sha>
"""
from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import shutil
import subprocess
import sys
from functools import lru_cache
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SNAPSHOT_DIR = REPO_ROOT / "data" / "third_party" / "aegis"
SNAPSHOT_GZ = SNAPSHOT_DIR / "effects.json.gz"
SNAPSHOT_META = SNAPSHOT_DIR / "SNAPSHOT.json"
UPSTREAM_URL = "https://github.com/vinicius3333/aegis-digimon-tcg"
UPSTREAM_PATH = "packages/shared/src/effects/effects.json"

SECTION_TITLE = (
    "Aegis IR (third-party, self-audited, LOW trust — a structural hint only; "
    "printed text, DCGO and general_rule.pdf govern; never copy its "
    "digivolve/Assembly requirement data)"
)

# Requirement data we must never copy into a card spec. Stripped from every
# rendered section so it cannot leak into an authoring agent's context.
REQUIREMENT_FIELDS = frozenset({
    "digivolutionRequirement",
    "dnaDigivolveRequirement",
    "digiXrosRequirement",
    "assemblyRequirement",
    "linkRequirement",
    "appFusionRequirement",
    "baseGrantedDigivolve",
})


@lru_cache(maxsize=1)
def load_effects() -> dict:
    """The full cardId -> CompiledCard map from the vendored snapshot."""
    with gzip.open(SNAPSHOT_GZ, "rb") as fh:
        return json.loads(fh.read().decode("utf-8"))


@lru_cache(maxsize=1)
def snapshot_meta() -> dict:
    return json.loads(SNAPSHOT_META.read_text(encoding="utf-8"))


def aegis_record(card_id: str) -> dict | None:
    """The card's record with requirement fields stripped, or None if absent."""
    rec = load_effects().get(card_id.upper())
    if rec is None:
        return None
    return {k: v for k, v in rec.items() if k not in REQUIREMENT_FIELDS}


def aegis_section(card_id: str, heading: str | None = "##") -> str:
    """A labeled markdown section ready to drop into a per-card context pack.

    ``heading=None`` omits the title line, for templates that already carry it.
    """
    commit = snapshot_meta().get("commit", "?")[:9]
    lines = [f"{heading} {SECTION_TITLE}", ""] if heading else []
    raw = load_effects().get(card_id.upper())
    if raw is None:
        lines.append(f"(no Aegis record for {card_id.upper()} at {commit})")
        return "\n".join(lines)
    stripped = sorted(REQUIREMENT_FIELDS & raw.keys())
    lines += [
        f"Source: Aegis {commit} `{UPSTREAM_PATH}` (MIT; vendored at "
        "`data/third_party/aegis/`). Use only to cross-check clause decomposition "
        "(trigger / action order / optional / frequency). `coverage` is self-reported.",
    ]
    if stripped:
        lines.append(f"Omitted requirement fields (never copy): {', '.join(stripped)}.")
    lines += ["```json", json.dumps(aegis_record(card_id), indent=1, ensure_ascii=False), "```"]
    return "\n".join(lines)


# --- snapshot maintenance ----------------------------------------------------

def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def check() -> list[str]:
    """Problems with the vendored snapshot (empty list = OK)."""
    meta = snapshot_meta()
    problems = []
    gz = SNAPSHOT_GZ.read_bytes()
    raw = gzip.decompress(gz)
    if _sha256(raw) != meta["effects_json_sha256"]:
        problems.append("effects.json sha256 does not match SNAPSHOT.json")
    n = len(json.loads(raw.decode("utf-8")))
    if n != meta["card_count"]:
        problems.append(f"card_count {n} != SNAPSHOT.json {meta['card_count']}")
    if not (SNAPSHOT_DIR / "LICENSE").is_file():
        problems.append("LICENSE missing")
    return problems


def refresh(clone: Path | None, commit: str) -> None:
    """Re-vendor effects.json + LICENSE from upstream at ``commit``.

    Uses an existing clone if given (its HEAD is moved to ``commit``), else
    clones a fresh blobless copy into a temp dir with core.longpaths=true.
    """
    import tempfile

    tmp = None
    if clone is None:
        tmp = Path(tempfile.mkdtemp(prefix="aegis_"))
        clone = tmp / "a"
        subprocess.run(["git", "-c", "core.longpaths=true", "clone", "--filter=blob:none",
                        "--no-checkout", UPSTREAM_URL, str(clone)], check=True)
    try:
        git = ["git", "-C", str(clone), "-c", "core.longpaths=true"]
        subprocess.run([*git, "fetch", "--quiet", "origin"], check=False)
        full = subprocess.run([*git, "rev-parse", f"{commit}^{{commit}}"], check=True,
                              capture_output=True, text=True).stdout.strip()
        raw = subprocess.run([*git, "show", f"{full}:{UPSTREAM_PATH}"], check=True,
                             capture_output=True).stdout
        license_bytes = subprocess.run([*git, "show", f"{full}:LICENSE"], check=True,
                                       capture_output=True).stdout
        cards = json.loads(raw.decode("utf-8"))
        SNAPSHOT_DIR.mkdir(parents=True, exist_ok=True)
        # mtime=0 + no filename -> byte-reproducible gzip for a given input.
        SNAPSHOT_GZ.write_bytes(gzip.compress(raw, compresslevel=9, mtime=0))
        (SNAPSHOT_DIR / "LICENSE").write_bytes(license_bytes)
        meta = {
            "upstream": UPSTREAM_URL,
            "path": UPSTREAM_PATH,
            "commit": full,
            "effects_json_sha256": _sha256(raw),
            "effects_json_bytes": len(raw),
            "card_count": len(cards),
        }
        SNAPSHOT_META.write_text(json.dumps(meta, indent=2) + "\n", encoding="utf-8")
        print(f"vendored {len(cards)} records from {full[:9]} "
              f"({len(raw):,} B -> {SNAPSHOT_GZ.stat().st_size:,} B gz)")
    finally:
        if tmp is not None:
            shutil.rmtree(tmp, ignore_errors=True)


def main(argv=None) -> int:
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="replace")
        except Exception:
            pass
    argv = sys.argv[1:] if argv is None else argv
    if argv and argv[0] == "refresh":
        ap = argparse.ArgumentParser(prog="aegis_ir.py refresh")
        ap.add_argument("--clone", type=Path, help="existing local clone (else clones to a temp dir)")
        ap.add_argument("--commit", required=True, help="upstream commit to pin")
        a = ap.parse_args(argv[1:])
        refresh(a.clone, a.commit)
        return 0
    ap = argparse.ArgumentParser(description="Print Aegis IR context-pack sections.")
    ap.add_argument("card_ids", nargs="*")
    ap.add_argument("--check", action="store_true", help="verify the vendored snapshot")
    ap.add_argument("--no-heading", action="store_true",
                    help="omit the section title (for prompt templates that already carry it)")
    a = ap.parse_args(argv)
    if a.check:
        problems = check()
        for p in problems:
            print(f"FAIL: {p}")
        if not problems:
            print(f"OK: Aegis snapshot {snapshot_meta()['commit'][:9]}, "
                  f"{snapshot_meta()['card_count']} records")
        return 1 if problems else 0
    if not a.card_ids:
        ap.error("give card IDs, --check, or the refresh subcommand")
    heading = None if a.no_heading else "##"
    print("\n\n".join(aegis_section(cid, heading) for cid in a.card_ids))
    return 0


if __name__ == "__main__":
    sys.exit(main())
