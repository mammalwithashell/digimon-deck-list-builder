"""convert_raw.py round-7 fixes (round-6 review, finding 5).

A played card's hand-listing index is dropped only when the listing NAMES a card at the slot and that card is not the
played one. A listing with no `card_id` (an older recorder build) or a row with a hand but no `cards` says nothing
about which card the slot holds, so the old behaviour stands: the index is kept, and the queued-play gate
(`queued_play_indexes`) decides whether the row is a main-phase play. Before the fix the index was dropped for those
rows, the play fell through as a positional play past the gate, and the queued index stayed in the set.

Synthetic raw rows only: no recording (they carry private deck order) is read.
"""
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
_spec = importlib.util.spec_from_file_location(
    "dcgo_convert_raw_review_r7_under_test", ROOT / "code" / "dcgo-melon-recorder" / "convert_raw.py")
convert_raw = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(convert_raw)

YOU = 1
TAMER = "ST24-13"


def _game(*rows):
    deck = ["ST1-03"] * 10
    return [
        {"method": "Snapshot.SetRedraw", "t": "t", "you": YOU, "is_ai": False, "players": [
            {"id": 0, "hand": deck[:5], "library": deck[5:], "egg_library": []},
            {"id": 1, "hand": deck[:5], "library": deck[5:], "egg_library": []},
        ]},
        {"method": "TurnStateMachine.SetRedraw", "args": [0, False]},
        {"method": "TurnStateMachine.SetRedraw", "args": [1, False]},
        {"method": "TurnStateMachine.SetBreedingPhase", "tp": YOU, "turn": 1, "args": [YOU, False]},
        *rows,
        {"method": "TurnStateMachine.Surrender", "tp": YOU, "turn": 1, "args": [YOU]},
    ]


def _be32(n):
    n &= 0xFFFFFFFF
    return [(n >> 24) & 0xFF, (n >> 16) & 0xFF, (n >> 8) & 0xFF, n & 0xFF]


def _queue_play(card_index, frame):
    return {"method": "TurnStateMachine.QueueMainPhaseAction_Internal", "tp": YOU, "turn": 1,
            "args": [YOU, convert_raw.PLAY_PACKET_ID, _be32(card_index) + _be32(frame) + _be32(0) + _be32(-1) + _be32(0)],
            "play": {"card_index": card_index}}


def _play(cards, slot, listing):
    return {"method": "PlayCardClass.PlayCard", "tp": YOU, "turn": 1, "cards": cards, "pay_cost": True,
            "target": None, "seq": 99,
            "hand": {"player": YOU, "slot": slot, "cards": listing}}


def _main(out):
    """The main-phase play rows (the mulligans, breeding pass and closing concede are other sources)."""
    return [r for r in out if r.get("type") == "action" and r.get("source") == "main_phase"]


def test_a_listing_without_card_ids_keeps_the_played_index():
    # An older recorder build lists the hand without card ids. The listing says nothing against the played card, so
    # the slot's index (65) is kept: it is the queued play's index, and the play carries it.
    out, notes = convert_raw.convert(
        _game(_queue_play(65, 10), _play([TAMER], 1, [{"card_index": 58}, {"card_index": 65}])), cards={})
    plays = [r for r in out if r.get("type") == "action" and r.get("card_id") == TAMER]
    assert [p.get("card_index") for p in plays] == [65], plays
    assert not any("not the played card" in n for n in notes), notes


def test_a_row_with_a_hand_but_no_cards_is_still_gated_on_the_queued_play():
    # No `cards` (so no played card to compare with) and no queued play packet: the old behaviour skips the row at the
    # queued-play gate. Dropping the index first sent it past the gate as a positional play.
    out, notes = convert_raw.convert(
        _game(_play([], 1, [{"card_id": "ST24-07", "card_index": 58}, {"card_id": TAMER, "card_index": 65}])), cards={})
    assert not _main(out), _main(out)
    assert not any("hand slot" in n and "not the played card" in n for n in notes), notes


def test_the_queued_index_is_consumed_by_the_play_it_queued():
    # One queued play (index 65), then TWO rows with a hand but no `cards` listing 65 at the slot: the first is the
    # queued play (its index leaves the queued set), the second has no queued play left and is skipped. Dropping the
    # index let both through as positional plays and left 65 queued.
    listing = [{"card_id": "ST24-07", "card_index": 58}, {"card_id": TAMER, "card_index": 65}]
    out, _ = convert_raw.convert(_game(_queue_play(65, 10), _play([], 1, listing), _play([], 1, listing)), cards={})
    assert len(_main(out)) == 1, _main(out)


def test_a_listing_naming_another_card_still_drops_the_index():
    # Guard (green on the old code): the listed card at the slot is named and is not the played card.
    out, notes = convert_raw.convert(
        _game(_queue_play(58, 10), _play([TAMER], 0, [{"card_id": "ST24-07", "card_index": 58},
                                                      {"card_id": TAMER, "card_index": 65}])), cards={})
    plays = [r for r in out if r.get("type") == "action" and r.get("card_id") == TAMER]
    assert all("card_index" not in p for p in plays), plays
    assert any("seq 99" in n and "hand slot" in n for n in notes), notes
