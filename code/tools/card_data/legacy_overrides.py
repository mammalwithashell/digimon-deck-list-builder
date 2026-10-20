"""Read-only compatibility adapter for the legacy whole-field overrides.

The reviewed migration from ``data/card_overrides.json`` is complete. The file
remains readable only for historical ABI reports and explicit legacy callers;
the committed canonical generator does not apply it. This module intentionally
exposes no writer. New or changed corrections belong in reviewed
``card_corrections.json``.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any, Mapping, MutableMapping

from .errors import ValidationError


def load_legacy_overrides(
    path: str | os.PathLike[str] | None = None,
) -> dict[str, Any]:
    """Load and minimally validate the frozen legacy override document."""

    if path is None:
        from data_paths import CARD_OVERRIDES

        path = CARD_OVERRIDES
    source = Path(path)
    if not source.exists():
        return {}
    try:
        payload = json.loads(source.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValidationError(
            "failed to read legacy card overrides",
            path=source,
            details={"error": str(exc)},
        ) from exc
    if not isinstance(payload, dict):
        raise ValidationError(
            "legacy card override document root must be an object",
            path=source,
        )
    for card_id, fields in payload.items():
        if card_id.startswith("_"):
            continue
        if not isinstance(fields, dict):
            raise ValidationError(
                "legacy card override entry must be an object",
                card_id=card_id,
                path=source,
            )
    return payload


def apply_legacy_overrides(
    cards: MutableMapping[str, MutableMapping[str, Any]],
    overrides: Mapping[str, Any] | None = None,
    *,
    path: str | os.PathLike[str] | None = None,
) -> int:
    """Apply frozen legacy values for compatibility without writing them."""

    if overrides is None:
        overrides = load_legacy_overrides(path)
    applied = 0
    for card_id, patch in overrides.items():
        if card_id.startswith("_"):
            continue
        if card_id not in cards:
            print(f"  WARNING: override for unknown card_id {card_id!r} skipped")
            continue
        if not isinstance(patch, Mapping):
            raise ValidationError(
                "legacy card override entry must be an object",
                card_id=card_id,
            )
        cards[card_id].update(patch)
        applied += 1
    return applied
