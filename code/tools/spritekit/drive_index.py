"""Index the public "Digimon Up Assets" Google Drive folder.

The folder is link-shared, so it is NOT in the user's Drive corpus and the Drive
API/connector cannot list its children (``parentId = ...`` returns nothing). The
public ``embeddedfolderview`` endpoint, however, renders the *entire* folder as
plain HTML in a single response, which is what this module parses.

Output: ``data/sprite_refs/index.json`` -- one record per asset:
``{"id", "title", "folder", "stem", "kind", "subject"}``.
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT_FOLDER_ID = "1UWn4fjr_oOJjDDsytU82_9O1Ru_NJNiO"  # "Digimon Up Assets"
EMBED_URL = "https://drive.google.com/embeddedfolderview?id={fid}#list"
FILE_URL = "https://drive.google.com/uc?export=download&id={fid}"

_ENTRY_RE = re.compile(
    r'flip-entry" id="entry-([^"]+)".*?flip-entry-title">([^<]*)<', re.S
)
_IVD_RE = re.compile(r"_DRIVE_ivd'\]\s*=\s*'(.*?)';", re.S)

# UI_<kind>_<Subject>.png  ->  the single-pose sprite used on cards/lists.
_UI_RE = re.compile(r"^UI_(Partner|Enemy|Supporter|Pet|HolyWeapon)_(.+)$")


def _curl(url: str, timeout: int = 180) -> str:
    r = subprocess.run(
        ["curl", "-sS", "-L", url], capture_output=True, text=True, timeout=timeout
    )
    if r.returncode != 0:
        raise RuntimeError(f"curl failed for {url}: {r.stderr[:300]}")
    return r.stdout


def list_folder(fid: str) -> list[dict]:
    """Return ``[{"id","title","mime"}]`` for every child of *fid*.

    Tries ``embeddedfolderview`` first (returns the whole folder at once); falls
    back to the folder page's ``_DRIVE_ivd`` blob, which is capped at ~50 rows
    but is enough for the small top-level folder.
    """
    html = _curl(EMBED_URL.format(fid=fid))
    pairs = _ENTRY_RE.findall(html)
    if pairs:
        return [{"id": i, "title": t, "mime": ""} for i, t in pairs]

    html = _curl(f"https://drive.google.com/drive/folders/{fid}")
    m = _IVD_RE.search(html)
    if not m:
        return []
    data = json.loads(m.group(1).encode("utf-8").decode("unicode_escape"))
    return [{"id": e[0], "title": e[2], "mime": e[3]} for e in data[0]]


def classify(folder: str, title: str) -> tuple[str, str]:
    """Return ``(kind, subject)`` for an asset filename.

    ``kind`` is the role the asset plays for sprite authoring:

    ``ui_sprite``  a single hard-edged pose, ~33-100px -- the primary style ref
    ``atlas``      a packed animation sheet in Texture2D/<Subject>.png
    ``fx``         attack/hit effect sheets
    ``other``      UI chrome, backgrounds, avatar parts, icons
    """
    stem = title[:-4] if title.lower().endswith(".png") else title
    if folder == "Sprite":
        m = _UI_RE.match(stem)
        if m:
            return "ui_sprite", m.group(2)
        return "other", ""
    # Texture2D
    if stem.startswith("Fx_"):
        return "fx", stem[3:]
    if stem.startswith(("Character_", "Tex", "Sheet", "Bg", "BG", "UI_", "MapFx")):
        return "other", ""
    # A bare CapitalisedName.png in Texture2D is a character animation atlas.
    if re.fullmatch(r"[A-Z][A-Za-z0-9]*(_[A-Za-z0-9]+)*", stem):
        return "atlas", stem
    return "other", ""


def build(out: Path) -> list[dict]:
    top = list_folder(ROOT_FOLDER_ID)
    subs = [e for e in top if "folder" in e.get("mime", "")] or [
        {"id": e["id"], "title": e["title"]} for e in top
    ]
    if not subs:
        raise RuntimeError("no subfolders found under the root asset folder")

    records: list[dict] = []
    for sub in subs:
        name = sub["title"]
        children = list_folder(sub["id"])
        print(f"  {name}: {len(children)} files", file=sys.stderr)
        for c in children:
            kind, subject = classify(name, c["title"])
            stem = c["title"][:-4] if c["title"].lower().endswith(".png") else c["title"]
            records.append(
                {
                    "id": c["id"],
                    "title": c["title"],
                    "folder": name,
                    "stem": stem,
                    "kind": kind,
                    "subject": subject,
                }
            )
    out.parent.mkdir(parents=True, exist_ok=True)
    # Compact: this file is committed, and 6.3k records at indent=1 is ~1.2 MB.
    out.write_text(json.dumps(records, separators=(",", ":")), encoding="utf-8")
    return records


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--out", type=Path, default=Path("data/sprite_refs/index.json"))
    args = ap.parse_args()

    records = build(args.out)
    kinds: dict[str, int] = {}
    for r in records:
        kinds[r["kind"]] = kinds.get(r["kind"], 0) + 1
    subjects = {r["subject"] for r in records if r["kind"] in ("ui_sprite", "atlas")}
    print(f"indexed {len(records)} assets -> {args.out}")
    print(f"  by kind: {dict(sorted(kinds.items(), key=lambda kv: -kv[1]))}")
    print(f"  distinct sprite subjects: {len(subjects)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
