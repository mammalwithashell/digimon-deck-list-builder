"""PvP perspective normalization in `filter_state_for_player`.

Each player must receive a state where THEY are `player1` (the frontend renders
`player1` at the bottom/self slot). Without this, the joiner (seat 2) sees the
host's board at the bottom and their own board on top — the action mask is
perspective-relative, so seat 2 was fully mis-rendered. The fix: the per-recipient
filter also swaps players and remaps every ABSOLUTE player reference
(currentPlayer / winner / revealedCards.owner / selectingPlayer / zoneOwner) and
negates the `memoryGauge` (which `to_ui_json` emits from player1's perspective).
Relative encodings (mask OWN/ENEMY ranges, pendingAttack slots) are untouched.
"""

from server.state_filter import filter_state_for_player


def _full_state():
    return {
        "currentPlayer": 1,
        "memoryGauge": 3,
        "winner": None,
        "revealedCards": [
            {"cardId": "R1", "owner": 1},
            {"cardId": "R2", "owner": 2},
        ],
        "pendingSelection": {
            "kind": "OwnField",
            "selectingPlayer": 1,
            "zoneOwner": 2,
            "validIndices": [100],
        },
        "pendingAttack": {"attackerSlot": 0, "targetSlot": 14},
        "player1": {
            "id": 1,
            "handIds": ["P1A", "P1B"],
            "handCards": [{"cardId": "P1A"}, {"cardId": "P1B"}],
            "securityIds": ["P1S"],
            "handCount": 2,
            "battleArea": [],
        },
        "player2": {
            "id": 2,
            "handIds": ["P2X"],
            "handCards": [{"cardId": "P2X"}],
            "securityIds": ["P2S"],
            "handCount": 1,
            "battleArea": [],
        },
    }


# ── Seat 2 (joiner): normalized so the viewer becomes player1 ──────────────


def test_player2_viewer_becomes_player1_with_own_hand():
    filtered = filter_state_for_player(_full_state(), player_id=2)
    # Viewer (original player2) now occupies the player1 (self/bottom) slot,
    # keeps their own hand, but never sees their own face-down security.
    assert filtered["player1"]["handIds"] == ["P2X"]
    assert filtered["player1"]["securityIds"] == []
    assert filtered["player1"]["id"] == 1


def test_player2_opponent_is_redacted_in_player2_slot():
    filtered = filter_state_for_player(_full_state(), player_id=2)
    # Opponent (original player1) now occupies the player2 (top) slot, fully redacted.
    assert filtered["player2"]["handIds"] == []
    assert filtered["player2"]["handCards"] == []
    assert filtered["player2"]["securityIds"] == []
    assert filtered["player2"]["id"] == 2


def test_player2_current_player_remapped():
    # Original turn player is seat 1 → the opponent from the viewer's POV → 2.
    filtered = filter_state_for_player(_full_state(), player_id=2)
    assert filtered["currentPlayer"] == 2


def test_player2_memory_gauge_negated():
    # memoryGauge is emitted from player1's perspective; seat 2 sees it negated.
    filtered = filter_state_for_player(_full_state(), player_id=2)
    assert filtered["memoryGauge"] == -3


def test_player2_revealed_card_owners_remapped():
    filtered = filter_state_for_player(_full_state(), player_id=2)
    owners = {rc["cardId"]: rc["owner"] for rc in filtered["revealedCards"]}
    assert owners == {"R1": 2, "R2": 1}


def test_player2_pending_selection_player_fields_remapped():
    filtered = filter_state_for_player(_full_state(), player_id=2)
    ps = filtered["pendingSelection"]
    assert ps["selectingPlayer"] == 2
    assert ps["zoneOwner"] == 1
    # Relative encodings are untouched.
    assert ps["kind"] == "OwnField"
    assert ps["validIndices"] == [100]


def test_player2_winner_remapped_and_none_preserved():
    s = _full_state()
    s["winner"] = 2
    assert filter_state_for_player(s, player_id=2)["winner"] == 1
    s["winner"] = None
    assert filter_state_for_player(s, player_id=2)["winner"] is None


def test_player2_does_not_mutate_input():
    s = _full_state()
    filter_state_for_player(s, player_id=2)
    assert s["currentPlayer"] == 1
    assert s["memoryGauge"] == 3
    assert s["player1"]["handIds"] == ["P1A", "P1B"]


# ── Seat 1 (host): identity perspective, unchanged behavior ────────────────


def test_player1_perspective_is_identity():
    filtered = filter_state_for_player(_full_state(), player_id=1)
    assert filtered["player1"]["handIds"] == ["P1A", "P1B"]
    assert filtered["player1"]["securityIds"] == []
    assert filtered["player2"]["handIds"] == []
    assert filtered["currentPlayer"] == 1
    assert filtered["memoryGauge"] == 3
    assert {rc["cardId"]: rc["owner"] for rc in filtered["revealedCards"]} == {"R1": 1, "R2": 2}
    assert filtered["pendingSelection"]["selectingPlayer"] == 1


# ── Integration: normalization runs over the REAL Rust to_ui_json ──────────


def test_normalization_over_rust_to_ui_json():
    """Locks the seat-2 fix against the real engine's UI dict shape (mirrors
    test_redaction_over_rust_to_ui_json, but for the perspective swap)."""
    import pytest

    digimon_engine = pytest.importorskip("digimon_engine")

    deck = ["ST1-01"] * 5 + ["ST1-03"] * 45
    game = digimon_engine.RustHeadlessGame(
        deck, deck, seed=2, observation_profile="standard_lite_v2"
    )
    full = game.to_ui_json()

    # Seat 2 view: the joiner now occupies the player1 (self) slot with their
    # own real hand; the opponent (orig player1) is redacted in player2.
    p2 = filter_state_for_player(full, player_id=2)
    assert p2["player1"]["id"] == 1
    assert p2["player1"]["handIds"] == full["player2"]["handIds"]
    assert p2["player1"]["handCount"] == full["player2"]["handCount"]
    assert p2["player2"]["id"] == 2
    assert p2["player2"]["handIds"] == []
    assert p2["player2"]["handCount"] == full["player1"]["handCount"]
    # Absolute refs mirrored to the viewer's perspective.
    assert p2["currentPlayer"] == (2 if full["currentPlayer"] == 1 else 1)
    assert p2["memoryGauge"] == -full["memoryGauge"]

    # Seat 1 view is the identity (regression guard).
    p1 = filter_state_for_player(full, player_id=1)
    assert p1["currentPlayer"] == full["currentPlayer"]
    assert p1["memoryGauge"] == full["memoryGauge"]
    assert p1["player1"]["handIds"] == full["player1"]["handIds"]
