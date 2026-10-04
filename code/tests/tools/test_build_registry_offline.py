"""Tests for tools.build_registry --offline: append-only index assignment without the API.

Offline mode used to rebuild norm_ids only, writing norm_id 0.0 for any card that had
no index and leaving it unindexed -- the state all 77 EX12 cards were ingested in.
"""
from __future__ import annotations

import json
import sys

from tools import build_registry


def _run_offline(tmp_path, monkeypatch, cards: dict) -> dict:
    path = tmp_path / "cards.json"
    path.write_text(json.dumps(cards, indent=2), encoding="utf-8")
    monkeypatch.setattr(build_registry, "CARDS_JSON", str(path))
    monkeypatch.setattr(sys, "argv", ["build_registry.py", "--offline"])
    build_registry.main()
    return json.loads(path.read_text(encoding="utf-8"))


def test_offline_indexes_unindexed_cards_after_the_highest_existing_index(tmp_path, monkeypatch):
    out = _run_offline(tmp_path, monkeypatch, {
        "BT1-002": {"index": 7, "norm_id": 7 / 20000, "card_id": "BT1-002"},
        "EX12-002": {"card_id": "EX12-002"},
        "BT1-001": {"index": 3, "norm_id": 3 / 20000, "card_id": "BT1-001"},
        "EX12-001": {"card_id": "EX12-001"},
    })

    # Existing indices never move; new ones follow the highest, in natural card order.
    assert {cid: card.get("index") for cid, card in out.items()} == {
        "BT1-002": 7, "EX12-002": 9, "BT1-001": 3, "EX12-001": 8,
    }
    assert out["EX12-001"]["norm_id"] == 8 / 20000
    # Same entry shape as every indexed card: index and norm_id lead.
    assert list(out["EX12-001"])[:3] == ["index", "norm_id", "card_id"]
    assert list(out) == ["BT1-002", "EX12-002", "BT1-001", "EX12-001"]
