"""Tests for tools.ingest_cards dropping ids the digimoncard.io API serves that are not cards.

The API serves RB1-10 beside RB1-010 Siriusmon: a two-digit number in a set
whose numbers are three digits, rarity "Unknown", no set name, and an older
wording ("unsuspend this Digimon" where the card says "you may unsuspend").
The official Bandai DB has no RB1-10. A set re-ingest replaces every card with
the set's prefix by the API's response, so without a filter it would come back.
"""
from __future__ import annotations

import io
import json

from tools import ingest_cards

RB1_010 = {
    "id": "RB1-010", "name": "Siriusmon", "type": "Digimon", "level": 6, "play_cost": 11,
    "dp": 11000, "color": "Red", "rarity": "SR", "form": "Mega", "attribute": "Vaccine",
    "digi_type": "Light Dragon", "evolution_cost": 3, "evolution_color": "Red", "evolution_level": 5,
    "xros_req": "[Digivolve] Lv.5 w/[Gammamon] in text: Cost 3",
    "main_effect": "[Your Turn] [Once Per Turn] When an opponent's Digimon is deleted, "
                   "you may unsuspend this Digimon.",
    "source_effect": "",
}
RB1_10 = dict(RB1_010, id="RB1-10", rarity="Unknown", form=None, evolution_color=None,
              evolution_level=None, xros_req="",
              main_effect="[Your Turn] (Once Per Turn) When an opponent's Digimon is deleted, "
                          "unsuspend this Digimon.")


def _serve(monkeypatch, rows):
    payload = json.dumps(rows).encode()
    monkeypatch.setattr(ingest_cards.urllib.request, "urlopen",
                        lambda request, timeout=30: io.BytesIO(payload))


def test_fetch_set_drops_the_rb1_10_duplicate_of_siriusmon(monkeypatch):
    _serve(monkeypatch, [RB1_010, RB1_10])

    assert [c["card_id"] for c in ingest_cards.fetch_set_by_card_prefix("RB1")] == ["RB1-010"]
