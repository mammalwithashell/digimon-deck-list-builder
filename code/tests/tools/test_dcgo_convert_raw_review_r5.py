"""convert_raw.py: round-4 review findings on the converter (review lane, round 5).

1. An unstamped "use this optional effect?" answer (OptionalSkill.SetUseOptional) is kept only for a seat whose answers
   are never stamped: a human seat, or the remote seat of a PvP game without the other client's stamp file. The mod
   stamps every menu answer of an agent-played seat in order, -1 meaning the engine asked nothing (Core.cs, the
   `engine_action` stamp: AppendStamp / ReadStamp), so an agent seat's unstamped answer is a DCGO prompt the engine
   folded, never a decision to replay.
5. A doBreeding whose breeding-area occupancy is unknown is a HATCH only when a Digi-Egg is seen in that area before an
   effect could have put one there: an effect's play into the breeding area goes through PlayPermanentClass, which the
   mod does not hook (raw-20261113T025510Z#g5 seq 834: P1's field board ends with Pinamon (BT20-004), which no logged
   row put there).
6. A DNA's two material picks are the seat's two own-side picks IMMEDIATELY before its play packet, in the same turn
   (TurnStateMachine.cs:2969-2983 queues the play from the root selection's end coroutine); a pick further back - an
   earlier turn's, or one with another row after it - is not a material.
9. A Burst Digivolve's Tamer pick is the seat's own-side pick right before the packet only when the picked permanent
   is a Tamer of that seat and the pick was made after the seat's previous action (SelectBurstDigivolutionEffect.cs:
   107-190 offers the seat's own Tamers; a DNA's material picks, which the converter writes AFTER the DNA action, are
   not a later play's Tamer pick).

Synthetic raw rows only: no recording (they carry private deck order) is read.
"""
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
_spec = importlib.util.spec_from_file_location(
    "dcgo_convert_raw_review_r5_under_test", ROOT / "code" / "dcgo-melon-recorder" / "convert_raw.py")
convert_raw = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(convert_raw)

P0, P1 = 0, 1


def _be32(n):
    n &= 0xFFFFFFFF
    return [(n >> 24) & 0xFF, (n >> 16) & 0xFF, (n >> 8) & 0xFF, n & 0xFF]


def _snapshot(you, is_ai=False):
    deck = ["ST1-03"] * 10
    return {"method": "Snapshot.SetRedraw", "t": "t", "you": you, "is_ai": is_ai, "players": [
        {"id": 0, "hand": deck[:5], "library": deck[5:], "egg_library": []},
        {"id": 1, "hand": deck[:5], "library": deck[5:], "egg_library": []},
    ]}


def _game(you, *rows, is_ai=False):
    return [
        _snapshot(you, is_ai),
        {"method": "TurnStateMachine.SetRedraw", "args": [0, False]},
        {"method": "TurnStateMachine.SetRedraw", "args": [1, False]},
        *rows,
        {"method": "TurnStateMachine.Surrender", "tp": you, "turn": 9, "args": [you]},
    ]


def _decisions(out):
    return [r for r in out if r["type"] in ("action", "selection")]


# ── 1. unstamped optional answers ────────────────────────────────────────────────────────────────────────────────


def _breed(seat, turn, do=False):
    return {"method": "TurnStateMachine.SetBreedingPhase", "tp": seat, "turn": turn, "args": [seat, do]}


def _optional(seat, use, turn=1, **extra):
    row = {"method": "OptionalSkill.SetUseOptional", "tp": P0, "turn": turn, "args": [seat, use]}
    row.update(extra)
    return row


def _bools(out):
    return [(r["actor"], r["bool_value"], r.get("engine_action"), bool(r.get("unstamped")))
            for r in out if r.get("type") == "selection" and r.get("prompt") == "generic_bool"]


def test_agent_seats_unstamped_true_is_dropped_but_its_unstamped_false_is_kept():
    # P0 is agent-played: its answers carry engine_action stamps. Core.cs stamps an agent's DECLINE as 62 and leaves only
    # its "Use" answer unstamped, so an unstamped TRUE is the agent's own "Use" the engine asked nothing about (dropped,
    # noted with its seq) while an unstamped FALSE did not come from the agent (DCGO's AI, a StallWatch timeout or a
    # human; round 6) and is kept for the replay to judge. P1 (no stamps anywhere: human, or the remote seat without a
    # stamp file) keeps its unstamped answer.
    out, notes = convert_raw.convert(_game(
        P0,
        _breed(P0, 1),
        _optional(P0, True, engine_action=30),
        _optional(P0, False),
        _optional(P0, True, seq=77),
        _optional(P1, False),
    ), cards={})
    assert _bools(out) == [(P0, True, 30, False), (P0, False, None, True), (P1, False, None, True)], _bools(out)
    assert [n for n in notes if "dropped" in n and "seq 77" in n], notes
    steps = [r["step"] for r in _decisions(out)]
    assert steps == list(range(len(steps)))


def test_agent_seat_is_known_from_a_stamp_on_any_menu_row():
    # The stamp that marks P0 as agent-played is on a later generic_int: the whole raw decides, not the rows so far.
    # Its unstamped TRUE is dropped; (its unstamped false would be kept, see the test above).
    out, _ = convert_raw.convert(_game(
        P0,
        _breed(P0, 1),
        _optional(P0, True),
        {"method": "UserSelectionManager.SetIntForPlayer", "tp": P0, "turn": 1, "args": [P0, 1], "engine_action": 31},
    ), cards={})
    assert _bools(out) == [], _bools(out)


def test_human_seats_unstamped_optional_answers_are_kept():
    # Regression guard: no stamp anywhere (a human playing both clients, or the remote seat): both seats' unstamped
    # answers stay, marked unstamped.
    out, _ = convert_raw.convert(_game(P0, _breed(P0, 1), _optional(P0, False), _optional(P1, True)), cards={})
    assert _bools(out) == [(P0, False, None, True), (P1, True, None, True)], _bools(out)


# ── 5. unknown breeding occupancy and an egg that may have arrived unlogged ──────────────────────────────────────

EGG_CARDS = {"P-176": {"card_kind": 3, "level": 2, "play_cost": 0}}


def _pass(seat, turn):
    return {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": seat, "turn": turn,
            "args": [seat, convert_raw.PASS_PACKET_ID, []]}


def _digivolve_breeding(actor, turn, card, battle_p0, battle_p1, target):
    """An effect's digivolution onto the turn player's breeding card (pay_cost false), with both battle boards."""
    return {"method": "PlayCardClass.PlayCard", "tp": actor, "turn": turn, "cards": [card], "pay_cost": False,
            "breeding_area": False, "target": {"card_id": target, "card_index": 1}, "target_slot": -1,
            "target_breeding": True, "hand": {"player": actor, "slot": 0, "cards": [{"card_id": card, "card_index": 9}]},
            "boards": [list(battle_p0), list(battle_p1)]}


def _field_pick(actor, turn, board_p0, board_p1, frame=0):
    return {"method": "SelectPermanentEffect.SetTargetFrames", "tp": turn % 2, "turn": turn,
            "args": [actor, [actor == turn % 2], [frame]], "boards": [list(board_p0), list(board_p1)]}


def _attack(tp, turn, battle_p0, battle_p1):
    return {"method": "TurnStateMachine.SetAttackingPermaent", "tp": tp, "turn": turn, "args": [0, -1],
            "boards": [list(battle_p0), list(battle_p1)]}


def _unknown_occupancy_then(*after_decision):
    """P0's breeding Ryudamon (BT20-010) may have moved to the battle area (the boards cannot tell), so P0's turn-4
    doBreeding is HATCH or MOVE by an occupancy the converter does not know."""
    return _game(
        P1,
        _breed(P0, 2, do=True),
        _digivolve_breeding(P0, 2, "BT20-010", ["BT20-087"], [], target="EGG"),
        _pass(P0, 2),
        _breed(P1, 3),
        _field_pick(P0, 3, ["BT20-087", "BT20-010"], ["ST24-04"]),
        _attack(P1, 3, ["BT20-087", "BT20-010"], ["ST24-04"]),
        _pass(P1, 3),
        _breed(P0, 4, do=True),
        *after_decision,
        _pass(P0, 4),
    )


def _breeding_actions(out, actor):
    return [r["action_id"] for r in out if r["type"] == "action" and r.get("phase") == "Breeding"
            and r["actor"] == actor]


def test_egg_seen_after_an_effect_resolved_does_not_prove_the_hatch():
    # An effect resolved (P0 answered its "use?") before the Digi-Egg was seen in P0's breeding area: that effect may
    # have played the egg there through PlayPermanentClass, which the mod does not log. HATCH is not inferred.
    out, _ = convert_raw.convert(_unknown_occupancy_then(
        _optional(P0, True, turn=4),
        _digivolve_breeding(P0, 4, "BT20-011", ["BT20-087", "BT20-010"], [], target="P-176"),
    ), cards=dict(EGG_CARDS), allow_truncated=True)
    assert convert_raw.LAST_TRUNCATION is not None and "breeding" in convert_raw.LAST_TRUNCATION, \
        convert_raw.LAST_TRUNCATION
    assert "unlogged" in convert_raw.LAST_TRUNCATION, convert_raw.LAST_TRUNCATION
    assert _breeding_actions(out, P0) == [convert_raw.ACTION_HATCH]
    assert not any(r["type"] == "game_end" for r in out)


def test_egg_on_a_field_board_before_any_effect_proves_the_hatch():
    # The first row after the decision is a field board ending with the Digi-Egg (the breeding frame is the last
    # FieldPermanents entry, Player.cs:41-54): nothing could have put it there but this decision.
    out, inferred = convert_raw.convert(_unknown_occupancy_then(
        _field_pick(P0, 4, ["BT20-087", "BT20-010", "P-176"], []),
    ), cards=dict(EGG_CARDS))
    assert _breeding_actions(out, P0) == [convert_raw.ACTION_HATCH, convert_raw.ACTION_HATCH]
    assert any("P-176" in note for note in inferred), inferred


def test_egg_on_a_field_board_after_an_effect_resolved_stops():
    out, _ = convert_raw.convert(_unknown_occupancy_then(
        {"method": "MultipleSkills.SetTargetSkill", "tp": P0, "turn": 4, "args": [P0, 0]},
        _field_pick(P0, 4, ["BT20-087", "BT20-010", "P-176"], []),
    ), cards=dict(EGG_CARDS), allow_truncated=True)
    assert convert_raw.LAST_TRUNCATION is not None and "unlogged" in convert_raw.LAST_TRUNCATION, \
        convert_raw.LAST_TRUNCATION
    assert _breeding_actions(out, P0) == [convert_raw.ACTION_HATCH]


# ── 6. DNA material picks: contiguous, right before the packet, same turn ────────────────────────────────────────


def _dna_bytes(card_index, frame, roots):
    return _be32(card_index) + _be32(frame) + _be32(len(roots)) + [b for r in roots for b in _be32(r)]


def _own_pick(seat, tp, turn, frame, boards=None):
    """`seat`'s pick of its OWN permanent at `frame` while `tp` is the turn player."""
    row = {"method": "SelectPermanentEffect.SetTargetFrames", "tp": tp, "turn": turn,
           "args": [seat, [seat == tp], [frame]]}
    if boards is not None:
        row["boards"] = boards
    return row


def _queue(seat, turn, card_index, payload):
    return {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": seat, "turn": turn,
            "args": [seat, convert_raw.PLAY_PACKET_ID, payload], "play": {"card_index": card_index}}


def _resolve(seat, turn, slot, played_index, card, **extra):
    cards = [{"card_id": "ST1-03", "card_index": 100 + i} for i in range(5)]
    cards[slot] = {"card_id": card, "card_index": played_index}
    row = {"method": "PlayCardClass.PlayCard", "tp": seat, "turn": turn, "cards": [card], "pay_cost": True,
           "hand": {"player": seat, "slot": slot, "cards": cards}}
    row.update(extra)
    return row


def test_bot_dna_does_not_take_own_side_picks_from_the_previous_turn():
    # AI game: the bot (P1) answered two own-side picks during P0's turn 1 (a defensive effect); on its turn 2 it
    # queues a DNA. Those picks are not right before the packet (P0's pass is between them), so they are not the DNA's
    # materials: they stay where DCGO logged them, and the DNA has none.
    out, _ = convert_raw.convert(_game(
        P0,
        _breed(P0, 1),
        _own_pick(P1, P0, 1, 0),
        _own_pick(P1, P0, 1, 1),
        _pass(P0, 1),
        _queue(P1, 2, 77, _dna_bytes(77, 0, [3, 4])),
        _resolve(P1, 2, 2, 77, "BT1-084"),
        is_ai=True,
    ), cards={}, allow_truncated=True)
    rows = _decisions(out)
    kinds = [(r["type"], r.get("actor"), r.get("action_id", r.get("prompt"))) for r in rows]
    picks = [i for i, k in enumerate(kinds) if k == ("selection", P1, "SelectPermanentEffect")]
    p0_pass = kinds.index(("action", P0, convert_raw.ACTION_PASS), kinds.index(("action", P0, convert_raw.ACTION_PASS)) + 1)
    assert picks and all(i < p0_pass for i in picks), kinds
    assert convert_raw.LAST_TRUNCATION is not None and "no material picks" in convert_raw.LAST_TRUNCATION, \
        convert_raw.LAST_TRUNCATION


def test_dna_picks_with_a_row_between_them_and_the_packet_are_not_materials():
    # P0 picked two own permanents, then answered a count, then queued the DNA: the picks are not the selection the
    # play was queued from (that ends in the packet itself, TurnStateMachine.cs:2969-2983).
    out, _ = convert_raw.convert(_game(
        P0,
        _breed(P0, 1),
        _own_pick(P0, P0, 1, 0),
        _own_pick(P0, P0, 1, 1),
        {"method": "SelectCountEffect.SetCount", "tp": P0, "turn": 1, "args": [P0, 1]},
        _queue(P0, 1, 77, _dna_bytes(77, 0, [3, 4])),
        _resolve(P0, 1, 2, 77, "BT1-084"),
    ), cards={}, allow_truncated=True)
    assert convert_raw.LAST_TRUNCATION is not None and "no material picks" in convert_raw.LAST_TRUNCATION, \
        convert_raw.LAST_TRUNCATION


# ── 9. the Burst Digivolve's Tamer pick ──────────────────────────────────────────────────────────────────────────

TAMER_CARDS = {"ST24-13": {"card_kind": 1}, "ST24-07": {"card_kind": 0, "level": 6}}
BOARDS = [["ST24-07", "ST24-12", "ST24-13", "ST24-13"], ["BT20-056"]]


def _burst_bytes(card_index, frame, burst_frame):
    return _be32(card_index) + _be32(frame) + _be32(0) + _be32(burst_frame) + _be32(0)


def _board_pick(seat, frame, turn=1):
    return {"method": "SelectPermanentEffect.SetTargetFrames", "tp": seat, "turn": turn,
            "args": [seat, [True], [frame]], "boards": BOARDS}


def _resolve_burst(seat, slot, played_index, card, target_slot, turn=1):
    return _resolve(seat, turn, slot, played_index, card, target="perm", target_slot=target_slot, boards=BOARDS)


def test_burst_pick_of_a_non_tamer_stops():
    # The own-side pick right before the burst packet names ShineGreymon (ST24-07), not a Tamer: it is some other
    # selection, and the Tamer pick is not on this client.
    out, _ = convert_raw.convert(_game(
        P0,
        _breed(P0, 1),
        _board_pick(P0, 0),
        _queue(P0, 1, 82, _burst_bytes(82, 4, 11)),
        _resolve_burst(P0, 2, 82, "BT25-104", 0),
    ), cards=dict(TAMER_CARDS), allow_truncated=True)
    assert "Burst" in (convert_raw.LAST_TRUNCATION or ""), convert_raw.LAST_TRUNCATION
    assert not any(r.get("mechanic") == "burst" for r in out)


def test_burst_does_not_take_a_dna_material_pick_written_after_the_dna_action():
    # P0's DNA wrote its action AHEAD of its two material picks, so the last row before the burst packet is a
    # material pick - made BEFORE P0's previous action (the DNA). It is not the burst's Tamer pick.
    out, _ = convert_raw.convert(_game(
        P0,
        _breed(P0, 1),
        # (both picks name a ST24-13 of P0's, so only their place before P0's DNA action tells them apart)
        _own_pick(P0, P0, 1, 2, boards=BOARDS),
        _own_pick(P0, P0, 1, 3, boards=BOARDS),
        _queue(P0, 1, 77, _dna_bytes(77, 0, [3, 4])),
        _resolve(P0, 1, 1, 77, "BT1-084"),
        _queue(P0, 1, 82, _burst_bytes(82, 4, 11)),
        _resolve_burst(P0, 2, 82, "BT25-104", 0),
    ), cards=dict(TAMER_CARDS), allow_truncated=True)
    assert "Burst" in (convert_raw.LAST_TRUNCATION or ""), convert_raw.LAST_TRUNCATION
    assert not any(r.get("mechanic") == "burst" for r in out)


def test_burst_pick_of_an_own_tamer_after_the_previous_action_is_kept():
    # Regression guard: the pick names a Marcus Damon & Thomas H. Norstein (ST24-13) of P0, made after P0's breeding
    # decision: the burst's Tamer pick, written after the digivolve action.
    out, _ = convert_raw.convert(_game(
        P0,
        _breed(P0, 1),
        _board_pick(P0, 3),
        _queue(P0, 1, 82, _burst_bytes(82, 4, 11)),
        _resolve_burst(P0, 2, 82, "BT25-104", 0),
    ), cards=dict(TAMER_CARDS))
    rows = _decisions(out)
    burst = [r for r in rows if r.get("mechanic") == "burst"]
    assert len(burst) == 1 and burst[0]["targets"][0]["frame"] == 3
    at = rows.index(burst[0])
    assert rows[at - 1]["type"] == "action" and rows[at - 1].get("card_id") == "BT25-104"
