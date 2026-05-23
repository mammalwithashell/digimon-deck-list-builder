"""``run_metric`` / ``run_tags`` handlers backed by TensorBoard's
``EventAccumulator``.

Caching strategy (design.md §Decision 1):

- One accumulator per run name.
- Reload on every call (``EventAccumulator.Reload()`` is cheap and picks up
  new events on active runs).
- Drop and rebuild the accumulator if the underlying ``MaskablePPO_*/``
  directory rotates (detected via the directory's inode/mtime as a cache key).
"""

from __future__ import annotations

import os
import threading
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Optional, Union

from tensorboard.backend.event_processing.event_accumulator import EventAccumulator


# Defaults give us ~all scalars we care about and skip the heavy stuff
# (histograms, images, audio) — pilot_training only writes scalars anyway.
_SIZE_GUIDANCE = {
    "scalars": 0,        # 0 = keep all
    "images": 1,
    "audio": 1,
    "histograms": 1,
    "compressedHistograms": 1,
    "tensors": 1,
}


@dataclass
class _CachedAccumulator:
    accumulator: EventAccumulator
    tb_subdir: Path
    tb_subdir_mtime: float


class AccumulatorCache:
    """One :class:`EventAccumulator` per logical run, invalidated by mtime."""

    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._entries: Dict[str, _CachedAccumulator] = {}

    def get(self, run_dir: Path) -> Optional[EventAccumulator]:
        tb_subdir = _find_tb_subdir(run_dir)
        if tb_subdir is None:
            return None
        try:
            mtime = tb_subdir.stat().st_mtime
        except OSError:
            return None
        key = str(run_dir.resolve())
        with self._lock:
            cached = self._entries.get(key)
            if cached is None or cached.tb_subdir != tb_subdir or cached.tb_subdir_mtime < mtime:
                acc = EventAccumulator(str(tb_subdir), size_guidance=_SIZE_GUIDANCE)
                cached = _CachedAccumulator(accumulator=acc, tb_subdir=tb_subdir, tb_subdir_mtime=mtime)
                self._entries[key] = cached
        cached.accumulator.Reload()
        # Update mtime after Reload to track any growth-during-call.
        try:
            cached.tb_subdir_mtime = tb_subdir.stat().st_mtime
        except OSError:
            pass
        return cached.accumulator


# Module-level singleton so the cache persists across tool calls.
_CACHE = AccumulatorCache()


def _find_tb_subdir(run_dir: Path) -> Optional[Path]:
    """Locate the deepest TB log dir under ``run_dir``.

    SB3 writes ``<run_dir>/<algorithm>_<n>/events.out.tfevents.*``. If
    multiple algorithm dirs exist (rare — only if the same dir was reused
    for both MaskablePPO and MaskableRecurrentPPO), pick the one with the
    most recently modified event file.
    """
    if not run_dir.is_dir():
        return None
    best: Optional[Path] = None
    best_mtime = -1.0
    for child in run_dir.iterdir():
        if not child.is_dir():
            continue
        tfevents = list(child.glob("events.out.tfevents.*"))
        if not tfevents:
            continue
        for ev in tfevents:
            try:
                mtime = ev.stat().st_mtime
            except OSError:
                continue
            if mtime > best_mtime:
                best_mtime = mtime
                best = child
    return best


def _scalars_for(accumulator: EventAccumulator, tag: str, since_step: Optional[int]) -> List[Dict[str, Any]]:
    try:
        events = accumulator.Scalars(tag)
    except KeyError:
        return []
    rows = []
    for ev in events:
        step = int(ev.step)
        if since_step is not None and step < since_step:
            continue
        rows.append({"step": step, "wall_time": float(ev.wall_time), "value": float(ev.value)})
    rows.sort(key=lambda r: r["step"])
    return rows


def run_tags(run_dir: Optional[Path]) -> Dict[str, Any]:
    if run_dir is None or not run_dir.is_dir():
        return {"ok": False, "error": "run directory not found"}
    accumulator = _CACHE.get(run_dir)
    if accumulator is None:
        return {"ok": False, "error": f"no TensorBoard event files found under {run_dir}"}
    return {"ok": True, "tags": sorted(accumulator.Tags().get("scalars", []))}


def run_metric(
    run_dir: Optional[Path],
    tag: Union[str, List[str]],
    since_step: Optional[int],
) -> Dict[str, Any]:
    if run_dir is None or not run_dir.is_dir():
        return {"ok": False, "error": "run directory not found"}
    accumulator = _CACHE.get(run_dir)
    if accumulator is None:
        return {"ok": False, "error": f"no TensorBoard event files found under {run_dir}"}
    if isinstance(tag, list):
        series = {t: _scalars_for(accumulator, t, since_step) for t in tag}
        return {"ok": True, "series": series}
    return {"ok": True, "tag": tag, "values": _scalars_for(accumulator, tag, since_step)}
