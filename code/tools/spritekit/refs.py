"""Fetch + query the Digimon Up reference sprite library.

The index is built by ``drive_index.py``; actual PNGs are downloaded lazily into
a gitignored cache so the repo never carries the (large, third-party) asset dump.
"""
from __future__ import annotations

import json
import subprocess
from functools import lru_cache
from pathlib import Path

FILE_URL = "https://drive.google.com/uc?export=download&id={fid}"


def repo_root() -> Path:
    p = Path(__file__).resolve()
    for parent in p.parents:
        if (parent / ".git").exists():
            return parent
    return Path.cwd()


def index_path() -> Path:
    return repo_root() / "data" / "sprite_refs" / "index.json"


def cache_dir() -> Path:
    d = repo_root() / ".cache" / "sprite_refs"
    d.mkdir(parents=True, exist_ok=True)
    return d


@lru_cache(maxsize=1)
def load_index() -> list[dict]:
    p = index_path()
    if not p.exists():
        raise SystemExit(
            f"reference index missing: {p}\n"
            "build it first:  python code/tools/spritekit/drive_index.py"
        )
    return json.loads(p.read_text(encoding="utf-8"))


def find(kind: str | None = None, subject: str | None = None) -> list[dict]:
    """Records matching *kind* and/or a case-insensitive exact *subject*."""
    out = load_index()
    if kind:
        out = [r for r in out if r["kind"] == kind]
    if subject:
        s = subject.lower()
        out = [r for r in out if r["subject"].lower() == s]
    return out


def fetch(record: dict, force: bool = False) -> Path:
    """Download one indexed asset into the cache; return its local path."""
    dest = cache_dir() / f"{record['folder']}__{record['title']}"
    if dest.exists() and dest.stat().st_size > 0 and not force:
        return dest
    r = subprocess.run(
        ["curl", "-sSL", "--fail", "-o", str(dest), FILE_URL.format(fid=record["id"])],
        capture_output=True,
        timeout=180,
    )
    if r.returncode != 0 or not dest.exists() or dest.stat().st_size == 0:
        dest.unlink(missing_ok=True)
        raise RuntimeError(f"download failed: {record['title']}")
    return dest


def fetch_many(records: list[dict], workers: int = 12) -> dict[str, Path]:
    """Download *records* concurrently; return ``{title: path}`` for successes."""
    from concurrent.futures import ThreadPoolExecutor

    got: dict[str, Path] = {}
    with ThreadPoolExecutor(max_workers=workers) as ex:
        for rec, res in zip(records, ex.map(lambda r: _safe(fetch, r), records)):
            if res is not None:
                got[rec["title"]] = res
    return got


def _safe(fn, *a):
    try:
        return fn(*a)
    except Exception:
        return None


# --------------------------------------------------------------- ref quality

_QUALITY_CACHE = "quality.json"


def _measure_fill(path: Path) -> float:
    """Opaque pixels / bbox area. Character art sits well inside (0.15, 0.97)."""
    from PIL import Image

    im = Image.open(path).convert("RGBA")
    box = im.getbbox()
    if not box:
        return 0.0
    n = sum(1 for px in im.get_flattened_data() if px[3] >= 128)
    area = (box[2] - box[0]) * (box[3] - box[1])
    return n / area if area else 0.0


@lru_cache(maxsize=1)
def _quality() -> dict[str, float]:
    p = cache_dir() / _QUALITY_CACHE
    return json.loads(p.read_text("utf-8")) if p.exists() else {}


def is_usable(record: dict, lo: float = 0.15, hi: float = 0.97) -> bool:
    """False for empty placeholders and for full-bleed UI chrome.

    Some ``UI_*`` records in the dump are not character art at all — e.g.
    ``UI_Pet_Monzaemon.png`` is blank, and a few are solid banners. Both would
    otherwise be offered as donors.
    """
    q = _quality()
    key = record["title"]
    if key not in q:
        path = cache_dir() / f"{record['folder']}__{record['title']}"
        if not path.exists():
            return True  # not downloaded yet: don't pre-judge it
        try:
            q[key] = _measure_fill(path)
        except Exception:
            q[key] = 0.0
        (cache_dir() / _QUALITY_CACHE).write_text(json.dumps(q), encoding="utf-8")
    return lo <= q[key] <= hi
