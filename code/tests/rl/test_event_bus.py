"""Pure-Python tests for `RewardEventBus.derive()`.

Tests use synthetic `rl_state` dicts and event dicts (matching the
PyO3 binding's `event_to_pydict` shape) — no engine instantiation
required.
"""

from __future__ import annotations

from typing import Any, Dict, List

import pytest

from digimon_gym.agents.reward.event_bus import RewardEventBus
from digimon_gym.agents.reward.occurrences import (
    Digivolved,
    DnaDigivolved,
    MemoryShifted,
    OppDeleted,
    OwnDeleted,
    PlayedCard,
    SecurityLost,
    SecurityRemoved,
    StepElapsed,
    TerminalOutcome,
)


def _empty_state(**overrides: Any) -> Dict[str, Any]:
    """Build a minimal rl_state dict with sensible defaults."""
    state = {
        "game_over": False,
        "winner_id": None,
        "terminal_reason": None,
        "p1_security": 5,
        "p2_security": 5,
        "p1_digivolutions": 0,
        "p2_digivolutions": 0,
        "p1_dna_digivolutions": 0,
        "p2_dna_digivolutions": 0,
    }
    state.update(overrides)
    return state


def _play_event(
    *,
    seq: int = 1,
    player: int = 1,
    card_id: str = "X",
    cost_paid: int = 0,
    cost_printed: int = 0,
    via_alt_path: Any = None,
) -> Dict[str, Any]:
    return {
        "type": "Play",
        "seq": seq,
        "player": player,
        "source_card_id": card_id,
        "source_slot": 0,
        "target_card_id": None,
        "target_slot": None,
        "meta": {
            "cost_paid": cost_paid,
            "cost_printed": cost_printed,
            "via_alt_path": via_alt_path,
        },
    }


def _digivolve_event(
    *,
    seq: int = 1,
    player: int = 1,
    top_card_id: str = "Y",
    from_stack_top: str = "X",
    was_dna: bool = False,
    was_blast_dna: bool = False,
    memory_paid: int = 0,
) -> Dict[str, Any]:
    return {
        "type": "Digivolve",
        "seq": seq,
        "player": player,
        "source_card_id": top_card_id,
        "source_slot": 0,
        "meta": {
            "from_stack_top": from_stack_top,
            "was_dna": was_dna,
            "was_blast_dna": was_blast_dna,
            "memory_paid": memory_paid,
        },
    }


def _trash_event(*, seq: int = 1, player: int = 2, card_id: str = "Z") -> Dict[str, Any]:
    return {
        "type": "Trash",
        "seq": seq,
        "player": player,
        "source_card_id": card_id,
        "meta": {},
    }


def _memory_event(*, seq: int = 1, player: int = 1, delta: int = 0) -> Dict[str, Any]:
    return {
        "type": "MemoryChange",
        "seq": seq,
        "player": player,
        "meta": {"delta": delta, "total": 0},
    }


# ---- Always-on emissions -------------------------------------------------


def test_derive_always_emits_exactly_one_step_elapsed():
    bus = RewardEventBus()
    occ = bus.derive(_empty_state(), [], 0)
    step_count = sum(1 for o in occ if isinstance(o, StepElapsed))
    assert step_count == 1


def test_reset_clears_prev_rl_state():
    bus = RewardEventBus()
    # Step 1: establish prev
    bus.derive(_empty_state(p2_security=5), [], 0)
    # Step 2 (without reset): security drop emits SecurityRemoved
    occ = bus.derive(_empty_state(p2_security=3), [], 1)
    assert any(isinstance(o, SecurityRemoved) and o.count == 2 for o in occ)
    # Reset, then step 3 with same drop: NO SecurityRemoved (no prev to delta).
    bus.reset()
    occ = bus.derive(_empty_state(p2_security=3), [], 2)
    assert not any(isinstance(o, SecurityRemoved) for o in occ)


# ---- Security delta ------------------------------------------------------


def test_security_removed_fires_on_drop_in_p2_security():
    bus = RewardEventBus()
    bus.derive(_empty_state(p2_security=5), [], 0)
    occ = bus.derive(_empty_state(p2_security=3), [], 1)
    removed = [o for o in occ if isinstance(o, SecurityRemoved)]
    assert len(removed) == 1
    assert removed[0].count == 2


def test_security_lost_fires_on_drop_in_p1_security():
    bus = RewardEventBus()
    bus.derive(_empty_state(p1_security=5), [], 0)
    occ = bus.derive(_empty_state(p1_security=4), [], 1)
    lost = [o for o in occ if isinstance(o, SecurityLost)]
    assert len(lost) == 1
    assert lost[0].count == 1


def test_security_neither_fires_when_counts_unchanged():
    bus = RewardEventBus()
    bus.derive(_empty_state(p1_security=5, p2_security=5), [], 0)
    occ = bus.derive(_empty_state(p1_security=5, p2_security=5), [], 1)
    assert not any(isinstance(o, (SecurityRemoved, SecurityLost)) for o in occ)


def test_security_gain_does_not_fire_either_side():
    # Security going UP (rare — return-to-security effects) shouldn't emit.
    bus = RewardEventBus()
    bus.derive(_empty_state(p1_security=4, p2_security=4), [], 0)
    occ = bus.derive(_empty_state(p1_security=5, p2_security=5), [], 1)
    assert not any(isinstance(o, (SecurityRemoved, SecurityLost)) for o in occ)


def test_first_step_has_no_security_delta_emission():
    # Without a prev_rl_state, no delta is possible — the bus must not
    # synthesize false deltas.
    bus = RewardEventBus()
    occ = bus.derive(_empty_state(p1_security=5, p2_security=5), [], 0)
    assert not any(isinstance(o, (SecurityRemoved, SecurityLost)) for o in occ)


# ---- Terminal outcome ----------------------------------------------------


def test_terminal_outcome_fires_only_when_game_over_true():
    bus = RewardEventBus()
    occ = bus.derive(_empty_state(game_over=False), [], 10)
    assert not any(isinstance(o, TerminalOutcome) for o in occ)


def test_terminal_outcome_carries_winner_step_reason():
    bus = RewardEventBus()
    occ = bus.derive(
        _empty_state(
            game_over=True, winner_id=1, terminal_reason="security_attack",
        ),
        [],
        150,
    )
    terminals = [o for o in occ if isinstance(o, TerminalOutcome)]
    assert len(terminals) == 1
    assert terminals[0].winner_id == 1
    assert terminals[0].step_count == 150
    assert terminals[0].reason == "security_attack"


def test_terminal_outcome_winner_none_is_draw():
    bus = RewardEventBus()
    occ = bus.derive(
        _empty_state(game_over=True, winner_id=None, terminal_reason="step_limit"),
        [],
        300,
    )
    terminals = [o for o in occ if isinstance(o, TerminalOutcome)]
    assert terminals[0].winner_id is None


# ---- PlayedCard from GameEvent::Play -------------------------------------


def test_played_card_forwards_cost_and_alt_path_fields():
    bus = RewardEventBus()
    occ = bus.derive(
        _empty_state(),
        [_play_event(card_id="BT17-078", cost_paid=0, cost_printed=9,
                     via_alt_path="blast_dna_digivolve")],
        0,
    )
    plays = [o for o in occ if isinstance(o, PlayedCard)]
    assert len(plays) == 1
    assert plays[0].card_id == "BT17-078"
    assert plays[0].cost_paid == 0
    assert plays[0].cost_printed == 9
    assert plays[0].via_alt_path == "blast_dna_digivolve"


def test_played_card_handles_none_via_alt_path():
    bus = RewardEventBus()
    occ = bus.derive(
        _empty_state(),
        [_play_event(card_id="X", cost_paid=3, cost_printed=3, via_alt_path=None)],
        0,
    )
    plays = [o for o in occ if isinstance(o, PlayedCard)]
    assert plays[0].via_alt_path is None


def test_multiple_plays_in_one_step_each_emit_played_card():
    bus = RewardEventBus()
    occ = bus.derive(
        _empty_state(),
        [_play_event(seq=1, card_id="A"), _play_event(seq=2, card_id="B")],
        0,
    )
    plays = [o for o in occ if isinstance(o, PlayedCard)]
    assert [p.card_id for p in plays] == ["A", "B"]


# ---- Digivolved from GameEvent::Digivolve --------------------------------


def test_digivolved_forwards_was_dna_and_memory_paid():
    bus = RewardEventBus()
    occ = bus.derive(
        _empty_state(),
        [_digivolve_event(top_card_id="EVO", was_dna=False, memory_paid=3)],
        0,
    )
    digs = [o for o in occ if isinstance(o, Digivolved)]
    assert len(digs) == 1
    assert digs[0].top_card_id == "EVO"
    assert digs[0].was_dna is False
    assert digs[0].memory_paid == 3


def test_dna_digivolve_event_also_emits_dna_digivolved_marker():
    bus = RewardEventBus()
    occ = bus.derive(
        _empty_state(),
        [_digivolve_event(top_card_id="OMN", was_dna=True, memory_paid=0)],
        0,
    )
    digs = [o for o in occ if isinstance(o, Digivolved)]
    dna_markers = [o for o in occ if isinstance(o, DnaDigivolved)]
    assert len(digs) == 1
    assert digs[0].was_dna is True
    # Per spec decision 5: BOTH Digivolved and DnaDigivolved fire on DNA.
    assert len(dna_markers) == 1
    assert dna_markers[0].player == 1


def test_non_dna_digivolve_does_not_emit_dna_digivolved_marker():
    bus = RewardEventBus()
    occ = bus.derive(_empty_state(), [_digivolve_event(was_dna=False)], 0)
    assert not any(isinstance(o, DnaDigivolved) for o in occ)


def test_digivolved_enrichment_uses_card_data_lookup():
    def lookup(card_id: str):
        if card_id == "OMN":
            return (7, ["Royal Knight", "Holy"])
        return None

    bus = RewardEventBus(card_data_lookup=lookup)
    occ = bus.derive(
        _empty_state(),
        [_digivolve_event(top_card_id="OMN", was_dna=True)],
        0,
    )
    digs = [o for o in occ if isinstance(o, Digivolved)]
    assert digs[0].result_level == 7
    assert digs[0].result_traits == ["Royal Knight", "Holy"]


def test_digivolved_enrichment_fails_closed_when_lookup_returns_none():
    # Default lookup returns None for everything.
    bus = RewardEventBus()
    occ = bus.derive(
        _empty_state(),
        [_digivolve_event(top_card_id="UNKNOWN", was_dna=True)],
        0,
    )
    digs = [o for o in occ if isinstance(o, Digivolved)]
    assert digs[0].result_level is None
    assert digs[0].result_traits == []


# ---- MemoryShifted aggregation -------------------------------------------


def test_memory_shifted_aggregates_all_deltas_in_step_into_one_occurrence():
    bus = RewardEventBus()
    occ = bus.derive(
        _empty_state(),
        [
            _memory_event(seq=1, delta=3),
            _memory_event(seq=2, delta=-1),
            _memory_event(seq=3, delta=2),
        ],
        0,
    )
    shifts = [o for o in occ if isinstance(o, MemoryShifted)]
    assert len(shifts) == 1
    assert shifts[0].delta == 4  # 3 - 1 + 2


def test_memory_shifted_not_emitted_when_net_delta_zero():
    bus = RewardEventBus()
    occ = bus.derive(
        _empty_state(),
        [_memory_event(seq=1, delta=2), _memory_event(seq=2, delta=-2)],
        0,
    )
    assert not any(isinstance(o, MemoryShifted) for o in occ)


# ---- OppDeleted / OwnDeleted from Trash (v1 limitation: no zone filter) -


def test_trash_event_with_player_2_emits_opp_deleted():
    bus = RewardEventBus()
    occ = bus.derive(_empty_state(), [_trash_event(player=2, card_id="X")], 0)
    deletes = [o for o in occ if isinstance(o, OppDeleted)]
    assert len(deletes) == 1
    assert deletes[0].card_id == "X"


def test_trash_event_with_player_1_emits_own_deleted():
    bus = RewardEventBus()
    occ = bus.derive(_empty_state(), [_trash_event(player=1, card_id="Y")], 0)
    own = [o for o in occ if isinstance(o, OwnDeleted)]
    assert len(own) == 1


def test_trash_for_player_0_neither_side():
    # Defensive: an event with player=0 (sentinel — no player) emits
    # neither side.
    bus = RewardEventBus()
    occ = bus.derive(_empty_state(), [_trash_event(player=0)], 0)
    assert not any(isinstance(o, (OppDeleted, OwnDeleted)) for o in occ)


# ---- prev_rl_state lifecycle ---------------------------------------------


def test_prev_rl_state_advances_after_each_derive():
    bus = RewardEventBus()
    # Step 1: p2 starts at 5
    bus.derive(_empty_state(p2_security=5), [], 0)
    # Step 2: drop to 4 → fires +1
    occ = bus.derive(_empty_state(p2_security=4), [], 1)
    assert any(isinstance(o, SecurityRemoved) and o.count == 1 for o in occ)
    # Step 3: stays at 4 → no fire (prev=4 from step 2)
    occ = bus.derive(_empty_state(p2_security=4), [], 2)
    assert not any(isinstance(o, SecurityRemoved) for o in occ)
    # Step 4: drop to 2 → fires +2 (prev still 4 from step 3)
    occ = bus.derive(_empty_state(p2_security=2), [], 3)
    assert any(isinstance(o, SecurityRemoved) and o.count == 2 for o in occ)
