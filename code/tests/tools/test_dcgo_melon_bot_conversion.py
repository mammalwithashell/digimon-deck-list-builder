"""Regression from a real Wine bot recording: the AI bypasses queued play packets."""
import copy
import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def converter():
    spec = importlib.util.spec_from_file_location(
        "dcgo_convert_raw", ROOT / "code/dcgo-melon-recorder/convert_raw.py"
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def recorded_rows():
    path = ROOT / "qa/dcgo-mod-recordings/cloud/2026-11-14/cloud-bot-fixed-raw.jsonl"
    return [json.loads(line) for line in path.read_text().splitlines()]


def test_actual_bot_paid_plays_survive_conversion():
    module = converter()
    rows, inferred = module.convert(recorded_rows(), module.load_cards(ROOT / "data/cards.json"))
    plays = [r for r in rows if r.get("card_id")]
    assert [(r["actor"], r["card_id"], r["action_id"]) for r in plays] == [
        (1, "ST1-04", 414),  # Dracomon digivolves onto the breeding egg.
        (1, "ST1-16", 0),    # Gaia Force, now in the first hand slot.
        (1, "ST1-10", 0),    # Phoenixmon on the next bot turn.
    ]
    assert any("HATCH" in note for note in inferred)


def test_unqueued_paid_pvp_effect_is_still_not_a_main_phase_play():
    module = converter()
    raw = copy.deepcopy(recorded_rows())
    for row in raw:
        if "is_ai" in row:
            row["is_ai"] = False
    rows, inferred = module.convert(raw, module.load_cards(ROOT / "data/cards.json"))
    assert not [r for r in rows if r.get("card_id")]
    assert not inferred


def test_free_bot_effect_play_is_not_an_extra_main_phase_action():
    module = converter()
    raw = copy.deepcopy(recorded_rows())
    for row in raw:
        if row["method"] == "PlayCardClass.PlayCard":
            row["pay_cost"] = False
    rows, _ = module.convert(raw, module.load_cards(ROOT / "data/cards.json"))
    assert not [r for r in rows if r.get("card_id")]
