"""Tests for code/digimon_gym/agents/mulligan_log.py."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from digimon_gym.agents.mulligan_log import (
    _derive_lvl_counts,
    _derive_has_tamer,
)


def test_derive_lvl_counts_counts_each_level_bucket():
    # ST1-03 is a level-3 Digimon; ST1-01 is a level-2 (egg).
    # Pick known-existing ids from data/cards.json.
    counts = _derive_lvl_counts(["ST1-03", "ST1-03", "ST1-01", "ST1-03", "ST1-01"])
    # Only levels 3-7 are bucketed.
    assert counts["3"] == 3
    assert counts["4"] == 0
    assert counts["5"] == 0
    assert counts["6"] == 0
    assert counts["7"] == 0


def test_derive_lvl_counts_handles_unknown_ids():
    counts = _derive_lvl_counts(["NOT-A-REAL-CARD", "ST1-03"])
    assert counts["3"] == 1  # unknown id contributes 0 to every bucket


def test_derive_has_tamer_returns_false_when_no_tamer():
    # ST1-03 is a Digimon, not a Tamer.
    assert _derive_has_tamer(["ST1-03", "ST1-03"]) is False


def test_derive_has_tamer_returns_true_when_any_card_is_tamer():
    # Look up any Tamer-typed card from cards.json. If cards.json has no
    # tamer at all (extremely unlikely), skip rather than fail spuriously.
    from data_paths import CARDS_JSON
    cards = json.loads(Path(CARDS_JSON).read_text(encoding="utf-8"))
    # cards.json encodes type as card_kind int: 1 = Tamer
    tamer_ids = [cid for cid, c in cards.items() if c.get("card_kind") == 1]
    if not tamer_ids:
        pytest.skip("No Tamer cards in cards.json — cannot exercise has_tamer=True path")
    assert _derive_has_tamer([tamer_ids[0], "ST1-03"]) is True
