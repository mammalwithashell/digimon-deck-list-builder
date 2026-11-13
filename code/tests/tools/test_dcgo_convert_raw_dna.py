"""convert_raw.py: a DNA (jogress) digivolve becomes DNA_DIGIVOLVE_START + hand slot.

DCGO asks the two material Digimon (`SelectPermanentEffect`, the playing seat, own side) BEFORE it queues the play;
the queued `QueueMainPhaseAction_Internal` play packet's bytes then carry a non-empty `JogressEvoRootsFrameIDs` list
(card index, frame, int count, ids - big-endian). The engine starts DNA from the hand card and asks the materials
afterwards, so the converter writes the DNA action AHEAD of the two material rows and renumbers the steps. A play
packet with an empty jogress list onto an occupied frame stays an ordinary digivolve (400 + hand * 15 + field).

Synthetic raw rows only: no recording (they carry private deck order) is read.
"""
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
_spec = importlib.util.spec_from_file_location(
    "dcgo_convert_raw_under_test", ROOT / "code" / "dcgo-melon-recorder" / "convert_raw.py")
convert_raw = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(convert_raw)

YOU = 0


def _be32(n):
    return [(n >> 24) & 0xFF, (n >> 16) & 0xFF, (n >> 8) & 0xFF, n & 0xFF]


def _play_bytes(card_index, frame, roots):
    """PlayCardAction payload: card index, frame, then int[] JogressEvoRootsFrameIDs (count + ids)."""
    return _be32(card_index) + _be32(frame) + _be32(len(roots)) + [b for r in roots for b in _be32(r)]


def _snapshot():
    deck = ["ST1-03"] * 10
    return {"method": "Snapshot.SetRedraw", "t": "t", "you": YOU, "is_ai": False, "players": [
        {"id": 0, "hand": deck[:5], "library": deck[5:], "egg_library": []},
        {"id": 1, "hand": deck[:5], "library": deck[5:], "egg_library": []},
    ]}


def _hand(slot_of_played, played_index):
    cards = [{"card_index": 100 + i} for i in range(max(5, slot_of_played + 1))]
    cards[slot_of_played] = {"card_index": played_index}
    return {"player": YOU, "slot": slot_of_played, "cards": cards}


def _material(frame):
    return {"method": "SelectPermanentEffect.SetTargetFrames", "tp": YOU, "turn": 1, "args": [YOU, [True], [frame]]}


def _queue_play(card_index, frame, roots):
    return {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": YOU, "turn": 1,
            "args": [YOU, convert_raw.PLAY_PACKET_ID, _play_bytes(card_index, frame, roots)],
            "play": {"card_index": card_index}}


def _resolve_play(slot, played_index, card, **target):
    row = {"method": "PlayCardClass.PlayCard", "tp": YOU, "turn": 1, "cards": [card],
           "hand": _hand(slot, played_index)}
    row.update(target)
    return row


def _raw_game(*main_phase):
    return [
        _snapshot(),
        {"method": "TurnStateMachine.SetRedraw", "args": [0, False]},
        {"method": "TurnStateMachine.SetRedraw", "args": [1, False]},
        {"method": "TurnStateMachine.SetBreedingPhase", "tp": YOU, "turn": 1, "args": [YOU, False]},
        *main_phase,
        {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": YOU, "turn": 1,
         "args": [YOU, convert_raw.PASS_PACKET_ID, []]},
        {"method": "TurnStateMachine.Surrender", "tp": 1, "turn": 2, "args": [1]},
    ]


def _decisions(out):
    return [r for r in out if r["type"] in ("action", "selection")]


def test_dna_play_becomes_dna_action_ahead_of_its_two_material_picks():
    out, _ = convert_raw.convert(_raw_game(
        _material(0),
        _material(1),
        _queue_play(card_index=77, frame=0, roots=[3, 4]),
        _resolve_play(slot=2, played_index=77, card="BT1-084"),
    ), cards={})
    rows = _decisions(out)
    kinds = [(r["type"], r.get("action_id", r.get("prompt"))) for r in rows]
    assert kinds == [
        ("action", convert_raw.ACTION_KEEP),
        ("action", convert_raw.ACTION_KEEP),
        ("action", convert_raw.ACTION_PASS),  # breeding
        ("action", convert_raw.DNA_DIGIVOLVE_START + 2),  # 65: DNA from hand slot 2, before the materials
        ("selection", "SelectPermanentEffect"),
        ("selection", "SelectPermanentEffect"),
        ("action", convert_raw.ACTION_PASS),  # end of turn
        ("action", convert_raw.ACTION_CONCEDE),
    ], kinds
    dna = rows[3]
    assert dna["action_id"] == 65 and dna["actor"] == YOU and dna["card_id"] == "BT1-084"
    assert dna["source"] == "main_phase" and dna["phase"] == "Main"
    # The material picks keep their own order (first material, then second).
    assert [r["targets"] for r in rows[4:6]] == [[{"player": YOU, "frame": 0}], [{"player": YOU, "frame": 1}]]
    # Steps stay contiguous from 0 after the move, and game_end counts them all.
    assert [r["step"] for r in rows] == list(range(len(rows)))
    assert out[-1] == {"type": "game_end", "winner": 0, "reason": "surrender", "total_steps": len(rows)}


def test_play_packet_with_empty_jogress_list_stays_an_ordinary_digivolve():
    out, _ = convert_raw.convert(_raw_game(
        _queue_play(card_index=78, frame=1, roots=[]),
        _resolve_play(slot=3, played_index=78, card="BT1-030", target="perm", target_slot=1),
    ), cards={})
    rows = _decisions(out)
    plays = [r for r in rows if r["type"] == "action" and r.get("card_id") == "BT1-030"]
    assert len(plays) == 1
    assert plays[0]["action_id"] == convert_raw.DIGIVOLVE_START + 3 * convert_raw.FIELDS_PER_HAND + 1  # 446
    assert not any(convert_raw.DNA_DIGIVOLVE_START <= r.get("action_id", -1) < convert_raw.DNA_DIGIVOLVE_END
                   for r in rows if r["type"] == "action" and r.get("source") == "main_phase")
    assert [r["step"] for r in rows] == list(range(len(rows)))


def test_dna_flag_is_per_card_and_does_not_leak_to_a_later_digivolve():
    out, _ = convert_raw.convert(_raw_game(
        _material(0),
        _material(1),
        _queue_play(card_index=77, frame=0, roots=[3, 4]),
        _resolve_play(slot=2, played_index=77, card="BT1-084"),
        _queue_play(card_index=78, frame=0, roots=[]),
        _resolve_play(slot=1, played_index=78, card="BT1-030", target="perm", target_slot=0),
    ), cards={})
    rows = _decisions(out)
    ids = {r.get("card_id"): r["action_id"] for r in rows if r["type"] == "action" and r.get("card_id")}
    assert ids == {"BT1-084": 65, "BT1-030": convert_raw.DIGIVOLVE_START + 1 * convert_raw.FIELDS_PER_HAND + 0}
    assert [r["step"] for r in rows] == list(range(len(rows)))


def test_unresolved_dna_attempt_does_not_flag_a_later_ordinary_digivolve_of_the_same_card():
    # QueueMainPhaseAction logs ATTEMPTS (CLAUDE.md rule 27): a queued DNA packet DCGO never resolves must not make the
    # same card's later ordinary digivolve (empty jogress list, the latest packet for that card) a DNA action.
    out, _ = convert_raw.convert(_raw_game(
        _queue_play(card_index=77, frame=0, roots=[3, 4]),  # attempted, never resolved
        _queue_play(card_index=77, frame=1, roots=[]),
        _resolve_play(slot=2, played_index=77, card="BT1-084", target="perm", target_slot=1),
    ), cards={})
    rows = _decisions(out)
    plays = [r for r in rows if r["type"] == "action" and r.get("card_id") == "BT1-084"]
    assert len(plays) == 1
    assert plays[0]["action_id"] == convert_raw.DIGIVOLVE_START + 2 * convert_raw.FIELDS_PER_HAND + 1  # 431
    assert not any(convert_raw.DNA_DIGIVOLVE_START <= r.get("action_id", -1) < convert_raw.DNA_DIGIVOLVE_END
                   for r in rows if r["type"] == "action" and r.get("source") == "main_phase")
    assert [r["step"] for r in rows] == list(range(len(rows)))


def test_material_picks_of_an_unresolved_dna_attempt_stay_where_dcgo_logged_them():
    # The two material picks of a DNA attempt DCGO never resolved are not moved behind the later ordinary digivolve
    # (they belong to no DNA action). They stay in recorded order; the replay meets them with no prompt parked.
    out, _ = convert_raw.convert(_raw_game(
        _material(0),
        _material(1),
        _queue_play(card_index=77, frame=0, roots=[3, 4]),  # attempted, never resolved
        _queue_play(card_index=77, frame=1, roots=[]),
        _resolve_play(slot=2, played_index=77, card="BT1-084", target="perm", target_slot=1),
    ), cards={})
    rows = _decisions(out)
    kinds = [(r["type"], r.get("action_id", r.get("prompt"))) for r in rows]
    assert kinds == [
        ("action", convert_raw.ACTION_KEEP),
        ("action", convert_raw.ACTION_KEEP),
        ("action", convert_raw.ACTION_PASS),  # breeding
        ("selection", "SelectPermanentEffect"),
        ("selection", "SelectPermanentEffect"),
        ("action", convert_raw.DIGIVOLVE_START + 2 * convert_raw.FIELDS_PER_HAND + 1),
        ("action", convert_raw.ACTION_PASS),  # end of turn
        ("action", convert_raw.ACTION_CONCEDE),
    ], kinds
    assert [r["step"] for r in rows] == list(range(len(rows)))


REMOTE = 1 - YOU


def _remote_queue_play(card_index, frame, roots):
    row = _queue_play(card_index, frame, roots)
    row["tp"] = REMOTE
    row["args"][0] = REMOTE
    return row


def _remote_resolve_dna(slot, played_index, card, target_slot, board):
    """PlayCardClass.PlayCard of the REMOTE seat's DNA play: the mod logs the resolution-time target_slot of the
    permanent in the play's TargetFrameID (GetBattleAreaPermanents order) and both battle areas."""
    hand = _hand(slot, played_index)
    hand["player"] = REMOTE
    return {"method": "PlayCardClass.PlayCard", "tp": REMOTE, "turn": 2, "cards": [card], "pay_cost": True,
            "hand": hand, "target": {"card_id": board[target_slot], "card_index": 500},
            "target_slot": target_slot, "target_breeding": False, "boards": [["ST1-03"], list(board)]}


def _remote_turn(*main_phase):
    for row in main_phase:
        row["turn"] = 2
    return [
        _snapshot(),
        {"method": "TurnStateMachine.SetRedraw", "args": [0, False]},
        {"method": "TurnStateMachine.SetRedraw", "args": [1, False]},
        {"method": "TurnStateMachine.SetBreedingPhase", "tp": REMOTE, "turn": 2, "args": [REMOTE, False]},
        *main_phase,
        {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": REMOTE, "turn": 2,
         "args": [REMOTE, convert_raw.PASS_PACKET_ID, []]},
        {"method": "TurnStateMachine.Surrender", "tp": YOU, "turn": 3, "args": [YOU]},
    ]


def test_remote_seat_dna_without_picks_gets_material_rows_synthesized_from_the_jogress_roots():
    # raw-20261113T025510Z g3 seq 363/364: the REMOTE seat (P0, the recorder ran on P1) DNA-digivolves into
    # Angewomon (EX12-044) from frames [3, 4]. The remote client never runs the root selection, so no
    # SelectPermanentEffect row exists. The play's TargetFrameID 4 holds the battle-area permanent at compact
    # slot 1, so exactly one occupied battle frame lies below frame 4 - frame 3 - and the roots are slots [0, 1]
    # (DCGO frame ids index FieldPermanents, GetBattleAreaPermanents walks them in id order: Player.cs:621-640).
    board = ["EX12-042", "EX12-010", "EX12-066", "EX12-073"]
    out, _ = convert_raw.convert(_remote_turn(
        _remote_queue_play(card_index=10, frame=4, roots=[3, 4]),
        _remote_resolve_dna(slot=9, played_index=10, card="EX12-044", target_slot=1, board=board),
    ), cards={})
    rows = _decisions(out)
    kinds = [(r["type"], r.get("action_id", r.get("prompt"))) for r in rows]
    assert kinds == [
        ("action", convert_raw.ACTION_KEEP),
        ("action", convert_raw.ACTION_KEEP),
        ("action", convert_raw.ACTION_PASS),  # breeding
        ("action", convert_raw.DNA_DIGIVOLVE_START + 9),  # 72
        ("selection", "SelectPermanentEffect"),
        ("selection", "SelectPermanentEffect"),
        ("action", convert_raw.ACTION_PASS),
        ("action", convert_raw.ACTION_CONCEDE),
    ], kinds
    first, second = rows[4], rows[5]
    assert first["targets"] == [{"player": REMOTE, "frame": 0}]  # frame 3 -> Gatomon (EX12-042)
    assert second["targets"] == [{"player": REMOTE, "frame": 1}]  # frame 4 -> Greymon (EX12-010)
    for pick in (first, second):
        assert pick["actor"] == REMOTE and pick["synthesized"] == "jogress_roots"
        assert pick["board_p1"] == board  # resolution-time battle area: the replay remaps slots by identity
    assert [r["step"] for r in rows] == list(range(len(rows)))
    assert out[-1]["type"] == "game_end"


def test_remote_seat_dna_roots_in_root_order_after_the_anchor_frame():
    # roots [4, 5] with TargetFrameID 4 at slot 1: frame 5 is the very next frame, so it is slot 2.
    board = ["A", "B", "C", "D"]
    out, _ = convert_raw.convert(_remote_turn(
        _remote_queue_play(card_index=15, frame=4, roots=[5, 4]),
        _remote_resolve_dna(slot=3, played_index=15, card="EX12-032", target_slot=1, board=board),
    ), cards={})
    picks = [r for r in _decisions(out) if r.get("prompt") == "SelectPermanentEffect"]
    assert [p["targets"][0]["frame"] for p in picks] == [2, 1]  # DCGO's root order is the pick order


def test_remote_seat_dna_whose_roots_cannot_be_placed_stops_instead_of_guessing():
    # TargetFrameID 4 at slot 2: two of frames 0-3 are occupied, so frame 1 is slot 0 or slot 1 - not certain.
    out, _ = convert_raw.convert(_remote_turn(
        _remote_queue_play(card_index=10, frame=4, roots=[1, 4]),
        _remote_resolve_dna(slot=9, played_index=10, card="EX12-044", target_slot=2, board=["A", "B", "C", "D"]),
    ), cards={}, allow_truncated=True)
    assert "jogress roots" in convert_raw.LAST_TRUNCATION
    assert not any(r["type"] == "action" and r.get("card_id") == "EX12-044" for r in out)
    assert not any(r["type"] == "game_end" for r in out)


def test_remote_seat_dna_without_a_target_slot_stops_instead_of_guessing():
    resolve = _remote_resolve_dna(slot=9, played_index=10, card="EX12-044", target_slot=1, board=["A", "B"])
    del resolve["target_slot"]
    out, _ = convert_raw.convert(_remote_turn(
        _remote_queue_play(card_index=10, frame=4, roots=[3, 4]),
        resolve,
    ), cards={}, allow_truncated=True)
    assert "jogress roots" in convert_raw.LAST_TRUNCATION
    assert not any(r["type"] == "action" and r.get("card_id") == "EX12-044" for r in out)


def test_dna_with_a_single_material_pick_is_never_emitted():
    out, _ = convert_raw.convert(_raw_game(
        _material(1),
        _queue_play(card_index=77, frame=0, roots=[3, 4]),
        _resolve_play(slot=2, played_index=77, card="BT1-084"),
    ), cards={}, allow_truncated=True)
    assert convert_raw.LAST_TRUNCATION is not None and "pick" in convert_raw.LAST_TRUNCATION
    assert not any(r["type"] == "action" and r.get("card_id") == "BT1-084" for r in out)


def test_recorded_material_picks_that_contradict_the_jogress_roots_stop_the_conversion():
    # The local seat's two picks are checked against the packet's roots when the roots place with certainty:
    # roots [3, 4] at TargetFrameID 4 / slot 1 are slots [0, 1], so picks [1, 0] are not this DNA's materials.
    resolve = _resolve_play(slot=2, played_index=77, card="BT1-084", target={"card_id": "X", "card_index": 5},
                            target_slot=1, target_breeding=False, boards=[["A", "B", "C"], []])
    out, _ = convert_raw.convert(_raw_game(
        _material(1),
        _material(0),
        _queue_play(card_index=77, frame=4, roots=[3, 4]),
        resolve,
    ), cards={}, allow_truncated=True)
    assert "contradict" in convert_raw.LAST_TRUNCATION
    assert not any(r["type"] == "action" and r.get("card_id") == "BT1-084" for r in out)


def test_recorded_material_picks_that_match_the_jogress_roots_are_kept():
    # Regression guard: the same check passes the four real local-seat DNAs (raw-20261113T025510Z g1/g2/g4).
    resolve = _resolve_play(slot=2, played_index=77, card="BT1-084", target={"card_id": "X", "card_index": 5},
                            target_slot=1, target_breeding=False, boards=[["A", "B", "C"], []])
    out, _ = convert_raw.convert(_raw_game(
        _material(1),
        _material(0),
        _queue_play(card_index=77, frame=4, roots=[4, 3]),
        resolve,
    ), cards={})
    rows = _decisions(out)
    picks = [r for r in rows if r.get("prompt") == "SelectPermanentEffect"]
    assert [p["targets"][0]["frame"] for p in picks] == [1, 0]
    assert not any("synthesized" in p for p in picks)
    assert out[-1]["type"] == "game_end"


def test_bot_dna_gets_its_inferred_breeding_ahead_of_the_action_and_its_material_picks():
    # AI game: the bot's breeding decision is synthesised at its first play of the turn. For a DNA play that
    # first play's material picks are already written, so the breeding row must go ahead of them (and of the
    # DNA action), not between the picks and the action.
    bot = 1 - YOU
    snap = _snapshot()
    snap["is_ai"] = True

    def material(frame):
        return {"method": "SelectPermanentEffect.SetTargetFrames", "tp": bot, "turn": 2, "args": [bot, [True], [frame]]}

    hand = _hand(4, 88)
    hand["player"] = bot
    rows = [
        snap,
        {"method": "TurnStateMachine.SetRedraw", "args": [0, False]},
        {"method": "TurnStateMachine.SetRedraw", "args": [1, False]},
        material(0),
        material(1),
        {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": bot, "turn": 2,
         "args": [bot, convert_raw.PLAY_PACKET_ID, _play_bytes(88, 0, [0, 1])], "play": {"card_index": 88}},
        {"method": "PlayCardClass.PlayCard", "tp": bot, "turn": 2, "cards": ["BT1-084"], "hand": hand},
        {"method": "TurnStateMachine.Surrender", "tp": bot, "turn": 2, "args": [YOU]},
    ]
    out, _ = convert_raw.convert(rows, cards={})
    decisions = _decisions(out)
    kinds = [(r["type"], r.get("action_id", r.get("prompt"))) for r in decisions]
    assert kinds == [
        ("action", convert_raw.ACTION_KEEP),
        ("action", convert_raw.ACTION_KEEP),
        ("action", convert_raw.ACTION_PASS),  # the bot's inferred breeding PASS
        ("action", convert_raw.DNA_DIGIVOLVE_START + 4),
        ("selection", "SelectPermanentEffect"),
        ("selection", "SelectPermanentEffect"),
        ("action", convert_raw.ACTION_CONCEDE),
    ], kinds
    assert decisions[2]["inferred"] is True and decisions[2]["phase"] == "Breeding"
    assert [r["step"] for r in decisions] == list(range(len(decisions)))


def test_play_packet_without_a_readable_jogress_list_stops_instead_of_guessing():
    # The latest packet for card 77 decides; when its jogress list cannot be read the play's kind (DNA or not) is
    # unknown, so the converter truncates there instead of treating it as an ordinary play.
    unreadable = _queue_play(card_index=77, frame=0, roots=[])
    unreadable["args"][2] = unreadable["args"][2][:8]  # card index + frame only
    out, _ = convert_raw.convert(_raw_game(
        _queue_play(card_index=77, frame=0, roots=[3, 4]),
        unreadable,
        _resolve_play(slot=2, played_index=77, card="BT1-084", target="perm", target_slot=0),
    ), cards={}, allow_truncated=True)
    assert "no readable jogress list" in convert_raw.LAST_TRUNCATION
    assert not any(r["type"] == "action" and r.get("card_id") == "BT1-084" for r in out)
    assert not any(r["type"] == "game_end" for r in out)
