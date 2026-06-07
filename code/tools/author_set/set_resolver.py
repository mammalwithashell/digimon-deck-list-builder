"""Resolve a release-set prefix to its distinct card-ID list (Phase 1).

A release set (e.g. ``BT17``, ``EX12``, ``ST3``) is identified by its card-ID
prefix. ``cards.json`` is keyed by distinct card ID, so alternate-art printings
(which share an ID but differ in tcgplayer_id/rarity on the API) are already
collapsed to one entry per ID — resolving is a prefix filter.

Exact-prefix matching is required: ``BT1`` must not match ``BT10``. We anchor on
``^{PREFIX}-`` so the hyphen delimits the set token from the number.
"""

from __future__ import annotations

import json
import re
from typing import Optional


def normalize_prefix(prefix: str) -> str:
    """Uppercase + strip a set prefix. ``bt17`` -> ``BT17``, ``ex12 `` -> ``EX12``."""
    return prefix.strip().upper()


def resolve_set(prefix: str, cards: Optional[dict] = None) -> list[str]:
    """Return the sorted distinct card IDs for a set prefix from ``cards.json``.

    Args:
        prefix: Set prefix (case-insensitive), e.g. ``"BT17"`` or ``"ex12"``.
        cards: Optional pre-loaded cards dict (keyed by card_id). If ``None``,
            loads from ``data_paths.CARDS_JSON``.

    Returns:
        Sorted list of distinct card IDs whose set token exactly equals
        ``prefix``. Empty if the set is absent locally.
    """
    pre = normalize_prefix(prefix)
    if cards is None:
        cards = _load_cards()
    pat = re.compile(rf"^{re.escape(pre)}-\d")
    return sorted(cid for cid in cards if pat.match(str(cid)))


def resolve_set_from_api_rows(prefix: str, api_rows: list[dict]) -> list[str]:
    """Return sorted distinct card IDs for ``prefix`` from raw digimoncard.io rows.

    The API returns one row per printing (alternate arts share an ``id``), so we
    dedupe by ``id`` and filter to the exact set prefix — mirroring the dedupe in
    ``ingest_cards.fetch_set_by_card_prefix``.
    """
    pre = normalize_prefix(prefix)
    pat = re.compile(rf"^{re.escape(pre)}-\d")
    return sorted({str(r["id"]) for r in api_rows if pat.match(str(r.get("id", "")))})


def _load_cards() -> dict:
    from data_paths import CARDS_JSON  # local import: keeps module importable headless

    with open(CARDS_JSON, encoding="utf-8") as f:
        data = json.load(f)
    if isinstance(data, list):  # legacy array format
        return {c["card_id"]: c for c in data}
    return data
