"""Path resolution for ``runs/`` and ``models/``.

Mirrors ``code/digimon-engine-mcp/src/main.rs::default_cards_json_path`` — if
no explicit override is supplied, walk up from the current working directory
looking for a sibling named ``runs/`` or ``models/`` (up to ``MAX_ANCESTOR_DEPTH``
levels above cwd, so a user invoking the server from deep inside the repo
still resolves the right root).
"""

from __future__ import annotations

from pathlib import Path
from typing import Optional

MAX_ANCESTOR_DEPTH = 6


def _walk_for(dirname: str) -> Optional[Path]:
    cur = Path.cwd().resolve()
    for _ in range(MAX_ANCESTOR_DEPTH + 1):
        candidate = cur / dirname
        if candidate.is_dir():
            return candidate
        parent = cur.parent
        if parent == cur:
            break
        cur = parent
    return None


def resolve_runs_dir(override: Optional[Path]) -> Optional[Path]:
    """Resolve the ``runs/`` directory.

    Returns the override (resolved + existence-checked) if supplied, otherwise
    walks ancestors of ``Path.cwd()`` looking for ``./runs``. Returns ``None``
    when nothing is found — caller is responsible for surfacing a friendly
    error to the MCP client.
    """
    if override is not None:
        resolved = override.expanduser().resolve()
        return resolved if resolved.is_dir() else None
    return _walk_for("runs")


def resolve_models_dir(override: Optional[Path]) -> Optional[Path]:
    """Resolve the ``models/`` directory. Same semantics as :func:`resolve_runs_dir`."""
    if override is not None:
        resolved = override.expanduser().resolve()
        return resolved if resolved.is_dir() else None
    return _walk_for("models")


def resolve_repo_root(runs_dir: Optional[Path], models_dir: Optional[Path]) -> Optional[Path]:
    """Best-effort repo-root inference for ``qa/archetype-qa/panic-families.json`` lookup.

    Prefers the parent of ``runs_dir`` (the usual layout — ``runs/`` and ``qa/``
    sit at the same level inside the repo). Falls back to ``models_dir``'s parent.
    """
    if runs_dir is not None:
        return runs_dir.parent
    if models_dir is not None:
        return models_dir.parent
    return None


_MODEL_RUN_MARKERS = ("recordings", "checkpoints", "deck_pool_snapshot.json")


def _has_model_run_markers(p: Path) -> bool:
    """A directory looks like a model-run if any of its expected child markers exist."""
    return any((p / marker).exists() for marker in _MODEL_RUN_MARKERS)


def resolve_model_run_dir(
    models_dir: Optional[Path],
    name: str,
) -> Optional[tuple]:
    """Resolve ``<models-dir>/<name>/...`` to the directory holding model artifacts.

    Per design.md §Decision 8: ``<models-dir>/<name>/`` may either contain
    ``recordings/`` / ``checkpoints/`` / ``deck_pool_snapshot.json`` directly,
    or wrap a timestamped subdirectory (e.g. ``pilot_ppo_<timestamp>/``) that
    holds them. Returns ``(resolved_dir, model_run_id)`` or ``None``.

    The ``model_run_id`` reflects which subdirectory was chosen:

    - If ``<models-dir>/<name>/`` itself has markers, ``model_run_id == name``.
    - Otherwise the most-recently-modified child subdirectory is chosen and
      its name becomes ``model_run_id``.
    """
    if models_dir is None or not models_dir.is_dir():
        return None
    base = models_dir / name
    if not base.is_dir():
        return None
    if _has_model_run_markers(base):
        return (base, name)
    # Descend into the most-recently-modified marker-bearing child.
    candidates = []
    for child in base.iterdir():
        if not child.is_dir():
            continue
        if not _has_model_run_markers(child):
            continue
        try:
            mtime = child.stat().st_mtime
        except OSError:
            continue
        candidates.append((mtime, child))
    if not candidates:
        return None
    candidates.sort(reverse=True)
    chosen = candidates[0][1]
    return (chosen, chosen.name)
