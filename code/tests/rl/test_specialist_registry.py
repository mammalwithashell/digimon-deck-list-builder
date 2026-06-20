"""Unit tests for the deck-keyed specialist registry (add-deck-specialist-league).

Covers: round-trip persistence, current/history upsert, round-pool emission
(mirror inclusion + per-entry deck + all-deck coverage), the layout-hash gate,
and history retention.
"""
from __future__ import annotations

import json

import pytest

from digimon_gym.agents.specialist_registry import (
    LeaguePoolEntry,
    Specialist,
    SpecialistRegistry,
    slugify_deck,
)

LAYOUT = "deadbeef"
OTHER_LAYOUT = "feedface"

DECKS = [
    "ST-1 Gaia Red",
    "ST-2 Cocytus Blue",
    "ST-4 Giga Green",
]


def _spec(deck: str, round: int = 0, layout: str = LAYOUT, path: str | None = None) -> Specialist:
    return Specialist(
        deck=deck,
        weights_path=path or f"models/specialists/{slugify_deck(deck)}/r{round}/final.zip",
        algorithm="mlp",
        observation_profile="standard_lite_deck_v2",
        tensor_layout_hash=layout,
        round=round,
    )


def test_slugify_and_name():
    assert slugify_deck("ST-1 Gaia Red") == "st-1"
    assert slugify_deck("ST-6 Venomous Violet") == "st-6"
    assert slugify_deck("Rocks") == "rocks"
    assert _spec("ST-4 Giga Green", round=3).name == "st-4@r3"


def test_round_trip_persistence(tmp_path):
    reg = SpecialistRegistry(tmp_path / "registry.json")
    for d in DECKS:
        reg.set_current(_spec(d, round=0))
    reg.save()

    reloaded = SpecialistRegistry.load(tmp_path / "registry.json")
    assert sorted(reloaded.decks()) == sorted(DECKS)
    assert len(reloaded.history) == len(DECKS)
    assert reloaded.get("ST-1 Gaia Red").round == 0
    # on-disk shape is the documented {version, current, history}
    raw = json.loads((tmp_path / "registry.json").read_text())
    assert raw["version"] == 1
    assert set(raw["current"]) == set(DECKS)


def test_set_current_upserts_and_keeps_history(tmp_path):
    reg = SpecialistRegistry(tmp_path / "r.json")
    reg.set_current(_spec("ST-1 Gaia Red", round=0))
    reg.set_current(_spec("ST-1 Gaia Red", round=1))
    # current points at the newest round; history keeps both
    assert reg.get("ST-1 Gaia Red").round == 1
    assert len(reg.history) == 2


def test_emit_round_pool_includes_mirror_and_all_decks(tmp_path):
    reg = SpecialistRegistry(tmp_path / "r.json")
    for d in DECKS:
        reg.set_current(_spec(d, round=0))

    pool = reg.emit_round_pool("ST-1 Gaia Red", LAYOUT, keep_last_per_deck=1)
    pooled_decks = {e.deck for e in pool}
    # every deck is represented, INCLUDING the training deck (the mirror)
    assert pooled_decks == set(DECKS)
    assert "ST-1 Gaia Red" in pooled_decks
    # every entry carries a deck + a policy path (the (policy, deck) coupling)
    assert all(isinstance(e, LeaguePoolEntry) and e.deck and e.weights_path for e in pool)


def test_emit_round_pool_can_exclude_mirror(tmp_path):
    reg = SpecialistRegistry(tmp_path / "r.json")
    for d in DECKS:
        reg.set_current(_spec(d, round=0))
    pool = reg.emit_round_pool("ST-1 Gaia Red", LAYOUT, include_mirror=False)
    assert "ST-1 Gaia Red" not in {e.deck for e in pool}
    assert {e.deck for e in pool} == set(DECKS) - {"ST-1 Gaia Red"}


def test_layout_hash_gate_excludes_incompatible(tmp_path):
    reg = SpecialistRegistry(tmp_path / "r.json")
    reg.set_current(_spec("ST-1 Gaia Red", round=0, layout=LAYOUT))
    reg.set_current(_spec("ST-2 Cocytus Blue", round=0, layout=OTHER_LAYOUT))

    assert {s.deck for s in reg.compatible(LAYOUT)} == {"ST-1 Gaia Red"}
    pool = reg.emit_round_pool("ST-1 Gaia Red", LAYOUT)
    # the OTHER_LAYOUT specialist must not appear in a LAYOUT pool
    assert all(e.deck != "ST-2 Cocytus Blue" for e in pool)


def test_snapshots_for_keeps_last_n_newest(tmp_path):
    reg = SpecialistRegistry(tmp_path / "r.json")
    for rnd in range(4):
        reg.set_current(_spec("ST-4 Giga Green", round=rnd))
    snaps = reg.snapshots_for("ST-4 Giga Green", LAYOUT, keep_last=2)
    assert [s.round for s in snaps] == [3, 2]  # newest first, capped at 2


def test_write_round_pool_manifest_and_empty_guard(tmp_path):
    reg = SpecialistRegistry(tmp_path / "r.json")
    for d in DECKS:
        reg.set_current(_spec(d, round=0))

    out = reg.write_round_pool(tmp_path / "pool_st1.json", "ST-1 Gaia Red", LAYOUT)
    manifest = json.loads(out.read_text())
    assert manifest["for_deck"] == "ST-1 Gaia Red"
    assert {e["deck"] for e in manifest["entries"]} == set(DECKS)
    assert all("weights_path" in e and "deck" in e for e in manifest["entries"])

    # empty pool (no compatible snapshots) is an explicit error, not a silent pass
    empty = SpecialistRegistry(tmp_path / "empty.json")
    with pytest.raises(ValueError, match="empty round pool"):
        empty.write_round_pool(tmp_path / "p.json", "ST-1 Gaia Red", LAYOUT)
