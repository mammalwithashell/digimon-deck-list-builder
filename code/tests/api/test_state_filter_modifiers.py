"""WebSocket state filtering must treat the inspector's new battle-area
`modifiers` field as public data (battle permanents are public), while still
redacting opponent hand/security. Guards the `add-permanent-stack-inspector`
serialization surface against accidental redaction.
"""

from server.state_filter import (
    filter_state_for_player,
    filter_state_for_spectator,
)


def _permanent_with_modifiers():
    return {
        "topCardId": "BT24-018",
        "keywords": ["blocker"],
        "keywordBreakdown": {"innate": ["blocker"], "gained": []},
        "securityAttackModifier": 2,
        "modifiers": [
            {"type": "CannotBeDestroyed", "value": 0, "expiry": "Permanent", "sourceCardId": None},
            {"type": "ChangeDp", "value": 3000, "expiry": "EndOfTurn", "sourceCardId": "BT24-018"},
        ],
        "dpBreakdown": {"base": 4000, "sources": [], "temporary": 3000, "aura": 0, "total": 7000},
        "sources": [{"cardId": "SRC", "inheritedEffectText": "Gain 1 memory."}],
        "inheritedEffects": [{"cardId": "SRC", "text": "Gain 1 memory."}],
    }


def _full_state():
    perm = _permanent_with_modifiers()
    return {
        "player1": {
            "handIds": ["A", "B"],
            "handCards": [{"cardId": "A"}, {"cardId": "B"}],
            "securityIds": ["S1"],
            "battleArea": [perm],
        },
        "player2": {
            "handIds": ["X"],
            "handCards": [{"cardId": "X"}],
            "securityIds": ["S2"],
            "battleArea": [perm],
        },
    }


def test_modifiers_survive_player_filter_for_both_seats():
    state = _full_state()
    filtered = filter_state_for_player(state, player_id=1)

    # Own and opponent battle-area permanents keep their modifiers/runtime state.
    for key in ("player1", "player2"):
        perm = filtered[key]["battleArea"][0]
        assert perm["modifiers"][0]["type"] == "CannotBeDestroyed"
        assert perm["modifiers"][1]["value"] == 3000
        assert perm["keywordBreakdown"]["innate"] == ["blocker"]
        assert perm["dpBreakdown"]["total"] == 7000

    # Opponent hand/security still redacted; own hand preserved.
    assert filtered["player1"]["handIds"] == ["A", "B"]
    assert filtered["player1"]["securityIds"] == []
    assert filtered["player2"]["handIds"] == []
    assert filtered["player2"]["securityIds"] == []


def test_modifiers_survive_spectator_filter():
    state = _full_state()
    filtered = filter_state_for_spectator(state, spectator_mode="hidden")
    perm = filtered["player1"]["battleArea"][0]
    assert perm["modifiers"][0]["type"] == "CannotBeDestroyed"
    # Both hands redacted for spectators.
    assert filtered["player1"]["handIds"] == []
    assert filtered["player2"]["handIds"] == []


def test_redaction_over_rust_to_ui_json():
    """Integration: the relocated filter runs unchanged over the REAL Rust
    `RustHeadlessGame.to_ui_json()` dict — opponent hand/security are redacted,
    counts and public zones survive, the viewer keeps their own hand.

    Locks the Rust-output contract: the filter is allowlist-by-omission, so a
    new hidden key in `to_ui_json` would NOT be redacted and must be added to
    `_redact_player`. This asserts the keys that exist today.
    """
    import pytest

    digimon_engine = pytest.importorskip("digimon_engine")

    deck = ["ST1-01"] * 5 + ["ST1-03"] * 45
    game = digimon_engine.RustHeadlessGame(
        deck, deck, seed=1, observation_profile="standard_lite_v2"
    )
    full = game.to_ui_json()

    filtered = filter_state_for_player(full, player_id=1)

    # Opponent (player2) hand + security card data is fully redacted.
    assert filtered["player2"]["handIds"] == []
    assert filtered["player2"]["handCards"] == []
    assert filtered["player2"]["securityIds"] == []
    # Counts survive redaction (they live in separate keys).
    assert filtered["player2"]["handCount"] == full["player2"]["handCount"]
    assert filtered["player2"]["securityCount"] == full["player2"]["securityCount"]

    # The viewer keeps their own hand but never sees their own (face-down) security.
    assert filtered["player1"]["handIds"] == full["player1"]["handIds"]
    assert filtered["player1"]["handCards"] == full["player1"]["handCards"]
    assert filtered["player1"]["securityIds"] == []

    # Public zones pass through untouched for both players.
    for key in ("player1", "player2"):
        assert filtered[key]["battleArea"] == full[key]["battleArea"]
    assert filtered["memoryGauge"] == full["memoryGauge"]
