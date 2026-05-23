"""``run_summary`` handler: header + evals + panics + console tail.

The pieces parsed from ``console.log``:

- **Header block** — the banner between two ``"=" * 60`` lines that
  ``pilot_training.py:979-1012`` emits at startup. Each interior line has the
  shape ``  <Key>: <value>``; we strip whitespace and lowercase-with-underscore
  the key for stable JSON access.
- **Evals** — preferred source is ``evals.jsonl`` (structured, written by the
  callback patch). Falls back to regex-parsing the ``[Eval @ N steps] ...``
  lines if the sidecar doesn't exist (pre-Phase-2 runs).
- **Panics** — every line matching :data:`panic_families.PANIC_LINE_RE`,
  bucketed by family via :func:`panic_families.match_family`. Unmatched panics
  roll up under the synthetic family id ``"other"``.
- **Console tail** — the last ~N lines of the file, for free-text context.
"""

from __future__ import annotations

import io
import json
import re
from collections import deque
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional

from .panic_families import PANIC_LINE_RE, PanicFamily, match_family


HEADER_BANNER = "=" * 60


HEADER_LINE_RE = re.compile(r"^\s{2,}(?P<key>[A-Za-z][A-Za-z0-9 _-]*?):\s+(?P<value>.*?)\s*$")
"""Matches one interior header line: ``  Key:   value`` with two-or-more
leading spaces (rules out body-text false positives that happen to contain a
colon)."""


EVAL_LINE_RE = re.compile(
    r"\[Eval @ (?P<step>[0-9_,]+) steps\] "
    r"Win rate: (?P<win_rate>[0-9.]+)% \| "
    r"Mean reward: (?P<mean_reward>-?[0-9.]+) \| "
    r"Games played: (?P<games_played>[0-9_,]+)"
)
"""Matches ``pilot_training.py:548-554``'s eval print exactly."""


def _normalize_header_key(raw: str) -> str:
    """``"Algorithm"`` → ``"algorithm"``; ``"Total steps"`` → ``"total_steps"``."""
    return re.sub(r"[^a-z0-9]+", "_", raw.strip().lower()).strip("_")


_HEADER_SCAN_LINES = 80
"""How many leading lines to scan for header rows. pilot_training emits ~15-20
header lines plus surrounding banners and the optional Gauntlet / Generalist
blocks — 80 is enough headroom and keeps us from accidentally consuming body
lines later in the file."""


def parse_header(console_log: Path) -> Dict[str, str]:
    """Parse the header block from ``console.log``.

    ``pilot_training.py:979-1012`` prints three ``"=" * 60`` banners (opening,
    after the title line, closing) sandwiching the field rows. Rather than
    count banners (brittle if future code adds or removes a separator), we
    scan the first ``_HEADER_SCAN_LINES`` lines for any row matching the
    distinctive ``"  <Key>: <value>"`` shape and collect every match.
    """
    if not console_log.is_file():
        return {}
    out: Dict[str, str] = {}
    try:
        with console_log.open("r", encoding="utf-8", errors="replace") as fh:
            for idx, line in enumerate(fh):
                if idx >= _HEADER_SCAN_LINES:
                    break
                # Skip banner lines outright so they can never accidentally match.
                if line.strip() == HEADER_BANNER:
                    continue
                m = HEADER_LINE_RE.match(line)
                if not m:
                    continue
                out[_normalize_header_key(m.group("key"))] = m.group("value")
    except OSError:
        return {}
    return out


def read_eval_sidecar(evals_jsonl: Path, tail: int) -> Optional[List[Dict[str, Any]]]:
    """Read the last ``tail`` rows from ``evals.jsonl``.

    Returns ``None`` (not an empty list) when the file is missing, to let the
    caller distinguish "no sidecar exists" (fallback to console regex) from
    "sidecar exists but is empty" (legitimate state during a brand-new run).
    """
    if not evals_jsonl.is_file():
        return None
    rows: deque[Dict[str, Any]] = deque(maxlen=max(1, tail))
    try:
        with evals_jsonl.open("r", encoding="utf-8") as fh:
            for raw in fh:
                raw = raw.strip()
                if not raw:
                    continue
                try:
                    rows.append(json.loads(raw))
                except json.JSONDecodeError:
                    continue
    except OSError:
        return []
    return list(rows)


def fallback_parse_eval_console(console_log: Path, tail: int) -> List[Dict[str, Any]]:
    """Regex-parse ``[Eval @ N steps]`` lines from ``console.log``.

    Used only when ``evals.jsonl`` is absent. Field coverage is a subset of
    the sidecar (no ``draw_rate`` / ``by_archetype`` / ``mean_terminal_score``
    etc. — the print is lossy by design).
    """
    if not console_log.is_file():
        return []
    rows: deque[Dict[str, Any]] = deque(maxlen=max(1, tail))
    try:
        with console_log.open("r", encoding="utf-8", errors="replace") as fh:
            for line in fh:
                m = EVAL_LINE_RE.search(line)
                if not m:
                    continue
                rows.append({
                    "step": int(m.group("step").replace(",", "").replace("_", "")),
                    "win_rate": float(m.group("win_rate")) / 100.0,
                    "mean_reward": float(m.group("mean_reward")),
                    "games_played": int(m.group("games_played").replace(",", "").replace("_", "")),
                    "wall_time": None,
                    "draw_rate": None,
                    "mean_terminal_score": None,
                    "mean_dense_reward": None,
                    "mean_eval_episode_length": None,
                })
    except OSError:
        return []
    return list(rows)


def count_panics(console_log: Path, families: Iterable[PanicFamily]) -> Dict[str, Any]:
    """Scan ``console.log`` for panic lines and bucket by family.

    Returns ``{"total": int, "by_family": {<family_id>: int, ..., "other": int}}``.
    The ``"other"`` bucket is present only when at least one panic failed to
    match any family — keeps the output tidy when nothing's amiss.
    """
    families = list(families)
    by_family: Dict[str, int] = {}
    total = 0
    if not console_log.is_file():
        return {"total": 0, "by_family": {}}
    try:
        with console_log.open("r", encoding="utf-8", errors="replace") as fh:
            for line in fh:
                m = PANIC_LINE_RE.match(line.rstrip("\r\n"))
                if not m:
                    continue
                total += 1
                fam = match_family(m.group("msg"), families) or "other"
                by_family[fam] = by_family.get(fam, 0) + 1
    except OSError:
        return {"total": 0, "by_family": {}}
    return {"total": total, "by_family": by_family}


def tail_console(console_log: Path, n: int = 50) -> List[str]:
    """Return the last ``n`` lines of ``console.log`` (newline-stripped)."""
    if not console_log.is_file():
        return []
    buf: deque[str] = deque(maxlen=max(1, n))
    try:
        with console_log.open("r", encoding="utf-8", errors="replace") as fh:
            for line in fh:
                buf.append(line.rstrip("\r\n"))
    except OSError:
        return []
    return list(buf)


def run_summary(run_dir: Path, tail_evals: int, families: Iterable[PanicFamily]) -> Dict[str, Any]:
    """Compose the full ``run_summary`` response per spec §run_summary tool."""
    console_log = run_dir / "console.log"
    sidecar = run_dir / "evals.jsonl"
    evals = read_eval_sidecar(sidecar, tail_evals)
    sidecar_present = evals is not None
    if not sidecar_present:
        evals = fallback_parse_eval_console(console_log, tail_evals)
    return {
        "ok": True,
        "header": parse_header(console_log),
        "evals": evals,
        "evals_source": "sidecar" if sidecar_present else "console",
        "panics": count_panics(console_log, families),
        "recent_console_tail": tail_console(console_log, 50),
    }
