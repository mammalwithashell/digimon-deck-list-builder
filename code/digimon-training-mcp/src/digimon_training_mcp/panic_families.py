"""Panic-line parsing and family bucketing.

The training-recording wrapper at ``code/digimon_gym/agents/training_recording.py:194``
catches engine panics and logs them to stderr in a fixed format::

    [recorder env=N game=M step=K] ENGINE PANIC #C: <ExceptionType>: <message[:200]>

This module parses that line shape and buckets each panic into one of the
engine-gap families defined in ``qa/archetype-qa/panic-families.json``.
Unmatched panics roll up under the synthetic family id ``"other"``.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, List, Optional, Pattern


PANIC_LINE_RE: Pattern[str] = re.compile(
    r"^\[recorder env=(?P<env>\d+) game=(?P<game>\d+) step=(?P<step>\d+)\] "
    r"ENGINE PANIC #(?P<count>\d+): (?P<exc_type>\S+): (?P<msg>.*)$"
)
"""Matches the exact panic-line format emitted by ``training_recording.py``.

Field semantics (verified against the source):

- ``env``    — environment index (parallel-env slot)
- ``game``   — game counter within that env
- ``step``   — step count within that game when the panic surfaced
- ``count``  — process-wide crash counter (1-indexed)
- ``exc_type`` — Python exception class name (typically ``PanicException``)
- ``msg``    — the panic message, truncated to 200 chars by the wrapper
"""


@dataclass(frozen=True)
class PanicFamily:
    """A single engine-gap family definition with a compiled match pattern."""

    family_id: str
    description: str
    pattern: Pattern[str]
    status: str
    panic_site: Optional[str] = None


def load_panic_families(repo_root: Optional[Path]) -> List[PanicFamily]:
    """Load ``qa/archetype-qa/panic-families.json`` relative to ``repo_root``.

    Returns an empty list (NOT an error) if the file is missing — `run_summary`
    falls through to the synthetic ``"other"`` bucket for every panic, which
    is still useful information.
    """
    if repo_root is None:
        return []
    path = repo_root / "qa" / "archetype-qa" / "panic-families.json"
    if not path.is_file():
        return []
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return []
    families_raw = data.get("families", []) if isinstance(data, dict) else []
    out: List[PanicFamily] = []
    for entry in families_raw:
        try:
            out.append(
                PanicFamily(
                    family_id=str(entry["family_id"]),
                    description=str(entry.get("description", "")),
                    pattern=re.compile(str(entry["pattern"])),
                    status=str(entry.get("status", "unknown")),
                    panic_site=entry.get("panic_site"),
                )
            )
        except (KeyError, re.error):
            continue
    return out


def match_family(panic_msg: str, families: Iterable[PanicFamily]) -> Optional[str]:
    """Return the first matching ``family_id`` for ``panic_msg``, or None.

    Patterns are tried in declaration order; the JSON file is the canonical
    authority for ordering when families overlap (more-specific patterns
    should appear first there).
    """
    for fam in families:
        if fam.pattern.search(panic_msg):
            return fam.family_id
    return None
