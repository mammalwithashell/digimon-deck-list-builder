"""PvP perspective normalization of the EVENT stream (`normalize_events_for_player`).

Companion to ``test_state_filter_perspective.py``. The per-recipient STATE is
perspective-normalized (each viewer becomes ``player1``), but ``GameEvent``s carry
ABSOLUTE player ids (top-level ``player`` plus ``meta.target_player`` /
``meta.winner``). Broadcast unchanged, the joiner (seat 2) sees their own moves
labelled "Opponent" in the game log and event-driven animations render on the
wrong side. The fix mirrors the state filter: for seat 2 swap every absolute
player reference (1 ↔ 2); leave RELATIVE encodings (``source_slot`` /
``target_slot`` field indices, all non-player meta) untouched. Seat 1 is identity.

The authoritative event/meta shape is ``event_to_pydict`` in
``code/digimon-engine-py/src/lib.rs`` — ``target_player`` (Attack) and ``winner``
(GameOver) are the only absolute-player-id meta keys.
"""

from server.state_filter import normalize_events_for_player


def _play_event(player: int, *, seq: int = 1) -> dict:
    """A play event from `player`'s hand into their battle area (slot relative)."""
    return {
        "type": "play",
        "seq": seq,
        "player": player,
        "source_card_id": "BT1-010",
        "source_card_name": "Greymon",
        "source_slot": 0,
        "target_card_id": None,
        "target_card_name": None,
        "target_slot": None,
        "meta": {"cost_paid": 4, "cost_printed": 5, "via_alt_path": None},
    }


def _attack_event(player: int, target_player: int, *, seq: int = 2) -> dict:
    """An attack by `player`'s slot 1 against `target_player`'s slot 3."""
    return {
        "type": "attack",
        "seq": seq,
        "player": player,
        "source_card_id": "BT1-010",
        "source_card_name": "Greymon",
        "source_slot": 1,
        "target_card_id": "BT1-020",
        "target_card_name": "Tyrannomon",
        "target_slot": 3,
        "meta": {"target_player": target_player, "attacker_dp": 5000, "target_dp": 4000},
    }


def _game_over_event(winner, *, seq: int = 3) -> dict:
    """A game-over event. Engine leaves top-level `player` at the 0 default."""
    return {
        "type": "game_over",
        "seq": seq,
        "player": 0,
        "source_card_id": None,
        "source_card_name": None,
        "source_slot": None,
        "target_card_id": None,
        "target_card_name": None,
        "target_slot": None,
        "meta": {"winner": winner, "reason": "SecurityDepleted", "result": "win"},
    }


def _memory_event(player: int, *, seq: int = 4) -> dict:
    """A memory_change whose meta `total`/`delta` happen to be 1/2 — must NOT be
    mistaken for player ids and swapped."""
    return {
        "type": "memory_change",
        "seq": seq,
        "player": player,
        "source_card_id": None,
        "source_card_name": None,
        "source_slot": None,
        "target_card_id": None,
        "target_card_name": None,
        "target_slot": None,
        "meta": {"delta": 1, "total": 2},
    }


# ── Seat 2 (joiner): every absolute player id mirrored to the viewer's POV ──


def test_player2_event_player_swapped():
    # The joiner's own action (absolute player 2) must read as player 1 so the
    # frontend's `playerLabels{1:'You'}` labels it "You"; the opponent → 2.
    events = [_play_event(player=2), _play_event(player=1, seq=5)]
    out = normalize_events_for_player(events, player_id=2)
    assert out[0]["player"] == 1
    assert out[1]["player"] == 2


def test_player2_attack_target_player_swapped():
    out = normalize_events_for_player([_attack_event(player=1, target_player=2)], player_id=2)
    ev = out[0]
    assert ev["player"] == 2
    assert ev["meta"]["target_player"] == 1
    # Non-player meta untouched.
    assert ev["meta"]["attacker_dp"] == 5000
    assert ev["meta"]["target_dp"] == 4000


def test_player2_game_over_winner_swapped_and_none_preserved():
    out = normalize_events_for_player([_game_over_event(winner=2)], player_id=2)
    assert out[0]["meta"]["winner"] == 1
    # A null winner (draw) passes through unchanged.
    out_none = normalize_events_for_player([_game_over_event(winner=None)], player_id=2)
    assert out_none[0]["meta"]["winner"] is None


def test_player2_game_over_player_zero_default_preserved():
    # GameOver leaves top-level `player` at 0; a naive 1↔else swap would corrupt
    # it to 1. Non-seat values must pass through untouched.
    out = normalize_events_for_player([_game_over_event(winner=1)], player_id=2)
    assert out[0]["player"] == 0


def test_player2_slots_are_relative_and_untouched():
    # source_slot/target_slot are field indices relative to the (now-swapped)
    # player — like pendingAttack slots, they are NOT remapped.
    out = normalize_events_for_player([_attack_event(player=1, target_player=2)], player_id=2)
    assert out[0]["source_slot"] == 1
    assert out[0]["target_slot"] == 3


def test_player2_non_player_meta_not_swapped():
    # delta=1 / total=2 must survive verbatim — they are not player ids.
    out = normalize_events_for_player([_memory_event(player=1)], player_id=2)
    assert out[0]["player"] == 2
    assert out[0]["meta"] == {"delta": 1, "total": 2}


def test_player2_does_not_mutate_input():
    events = [_attack_event(player=1, target_player=2), _game_over_event(winner=2)]
    snapshot = [
        (e["player"], dict(e["meta"]))
        for e in events
    ]
    normalize_events_for_player(events, player_id=2)
    for ev, (orig_player, orig_meta) in zip(events, snapshot):
        assert ev["player"] == orig_player
        assert ev["meta"] == orig_meta


# ── Seat 1 (host): identity perspective ─────────────────────────────────────


def test_player1_perspective_is_identity():
    events = [
        _play_event(player=1),
        _attack_event(player=1, target_player=2),
        _game_over_event(winner=1),
    ]
    out = normalize_events_for_player(events, player_id=1)
    assert out[0]["player"] == 1
    assert out[1]["meta"]["target_player"] == 2
    assert out[2]["meta"]["winner"] == 1
