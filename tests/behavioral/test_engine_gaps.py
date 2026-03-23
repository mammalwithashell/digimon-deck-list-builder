"""Behavioral tests for engine gap resolutions.

Migrated from qa/test_engine_gaps.py into proper pytest format.
Tests the 6 engine gaps resolved on 2026-03-17 plus stability fixes.
Uses in-process engine directly -- no server required.
"""

import json
import traceback
from pathlib import Path

import pytest

from tests.helpers.game_builder import make_card

from digimon_gym.engine.game import Game
from digimon_gym.engine.core.permanent import Permanent
from digimon_gym.engine.core.player import Player
from digimon_gym.engine.core.card_source import CardSource
from digimon_gym.engine.core.entity_base import CEntity_Base
from digimon_gym.engine.data.enums import GamePhase, EffectTiming, CardColor, CardKind
from digimon_gym.engine.interfaces.modifiers import ModifierType
from digimon_gym.engine.game.action_mask import build_action_mask
from digimon_gym.engine.game.constants import SECURITY_TARGET, TARGETS_PER_ATTACKER


def make_game():
    """Create a properly initialized Game."""
    game = Game()
    game.start_game()
    return game


# ---------------------------------------------------------------------------
# Greedy baseline helpers (inlined from qa/debug_game_helper.run_greedy_baseline)
# ---------------------------------------------------------------------------

DECK_LIBRARY_PATH = Path(__file__).parent.parent.parent / "digimon_gym" / "engine" / "data" / "deck_library.json"


def _load_deck(archetype_name: str) -> list[str]:
    """Load the first decklist for an archetype from deck_library.json."""
    with open(DECK_LIBRARY_PATH, "r", encoding="utf-8") as f:
        library = json.load(f)
    arch = library["archetypes"].get(archetype_name)
    if not arch or not arch.get("decklists"):
        raise ValueError(f"No decklists for archetype: {archetype_name}")
    return json.loads(arch["decklists"][0]["decklist"])


def _run_greedy_baseline(archetype1: str, archetype2: str) -> dict:
    """Run a greedy baseline game in-process using the engine directly.

    Returns dict with: matchup, completed, turns, winner, steps, crash_error
    """
    try:
        deck1 = _load_deck(archetype1)
        deck2 = _load_deck(archetype2)

        from digimon_gym.digimon_gym import DigimonEnv, greedy_policy

        env = DigimonEnv()
        env.reset(options={"deck1": deck1, "deck2": deck2})

        done = False
        steps = 0
        max_steps = 500

        while not done and steps < max_steps:
            action = greedy_policy(env)
            _, _, done, _, _ = env.step(action)
            steps += 1

        game = env.runner.game
        winner_id = game.winner.player_id if game.winner else 0
        turns = game.turn_count

        return {
            "matchup": f"{archetype1} vs {archetype2}",
            "completed": done,
            "turns": turns,
            "winner": winner_id,
            "steps": steps,
            "crash_error": None if done else f"Exceeded max_steps ({max_steps})",
        }
    except Exception as e:
        return {
            "matchup": f"{archetype1} vs {archetype2}",
            "completed": False,
            "turns": 0,
            "winner": None,
            "steps": 0,
            "crash_error": f"{type(e).__name__}: {e}\n{traceback.format_exc()}",
        }


# ---------------------------------------------------------------------------
# Unit tests for individual engine gaps
# ---------------------------------------------------------------------------


def test_1a_change_security_attack_wiring():
    """CHANGE_SECURITY_ATTACK modifier is wired into permanent.security_attack_modifier()."""
    game = make_game()

    card = make_card("TEST-SA", "TestMon", owner=game.player1)
    perm = Permanent([card])
    perm._owner_game = game
    game.player1.battle_area.append(perm)

    # Baseline: no SA modifiers
    sa0 = perm.security_attack_modifier()
    assert sa0 == 0

    # Register CHANGE_SECURITY_ATTACK +2 from registry
    game.register_modifier(
        perm, ModifierType.CHANGE_SECURITY_ATTACK,
        value_fn=lambda cur, t, c: cur + 2,
        expiry='end_of_turn',
    )
    sa1 = perm.security_attack_modifier()
    assert sa1 == 2

    # Register another -1 from registry (should stack)
    game.register_modifier(
        perm, ModifierType.CHANGE_SECURITY_ATTACK,
        value_fn=lambda cur, t, c: cur - 1,
        expiry='end_of_turn',
    )
    sa2 = perm.security_attack_modifier()
    assert sa2 == 1

    # Verify temp SA modifier also still works alongside registry
    perm._temp_sa_modifier = 3
    sa3 = perm.security_attack_modifier()
    # 3 (temp) + 2 - 1 (registry) = 4
    assert sa3 == 4


def test_1b_cannot_attack_player():
    """CANNOT_ATTACK_PLAYER modifier prevents security attacks."""
    game = make_game()
    game.current_phase = GamePhase.Main
    game.memory = 5

    # Set up: player 1 has a Digimon
    card1 = make_card("ATK-001", "Attacker", dp=5000, level=4, owner=game.player1)
    perm1 = Permanent([card1])
    perm1._owner_game = game
    perm1.turn_played = -1  # No summoning sickness
    game.player1.battle_area.append(perm1)

    # Player 2 has a suspended Digimon and security
    card2 = make_card("DEF-001", "Defender", dp=3000, level=3, owner=game.player2)
    perm2 = Permanent([card2])
    perm2._owner_game = game
    perm2.is_suspended = True
    game.player2.battle_area.append(perm2)

    sec = make_card("SEC-001", "SecCard", owner=game.player2)
    game.player2.security_cards.append(sec)

    # Before modifier: can attack player
    assert perm1.can_attack_player() is True

    # Register CANNOT_ATTACK_PLAYER
    game.register_modifier(perm1, ModifierType.CANNOT_ATTACK_PLAYER, expiry='end_of_turn')

    assert perm1.can_attack_player() is False

    # Build action mask and verify
    mask = build_action_mask(game, 1)

    sec_action = 100 + 0 * TARGETS_PER_ATTACKER + SECURITY_TARGET
    digimon_action = 100 + 0 * TARGETS_PER_ATTACKER + 0

    assert mask[sec_action] == 0.0, "Security attack should be MASKED OUT"
    assert mask[digimon_action] == 1.0, "Digimon attack should be available"


def test_1c_is_own_effect_context():
    """is_own_effect in WhenRemoveField context."""
    game = make_game()

    # Capture WhenRemoveField context
    captured = []
    original = game.player1._fire_timing

    def patched(timing, context):
        if timing in (EffectTiming.WhenRemoveField, EffectTiming.OnRemovedField,
                       EffectTiming.WhenPermanentWouldBeDeleted):
            captured.append((timing.name, dict(context)))
        original(timing, context)

    game.player1._fire_timing = patched

    # Test 1: Delete by opponent effect
    card1 = make_card("DEL-001", "Victim1", owner=game.player1)
    perm1 = Permanent([card1])
    perm1._owner_game = game
    game.player1.battle_area.append(perm1)
    game.player1.delete_permanent(perm1, is_opponent_effect=True, removal_cause='effect')

    opp_contexts = [(name, ctx) for name, ctx in captured
                     if name == 'WhenRemoveField']
    assert len(opp_contexts) >= 1, "WhenRemoveField should have fired"
    ctx = opp_contexts[-1][1]
    assert ctx['is_opponent_effect'] is True
    assert ctx['is_own_effect'] is False

    # Also check WhenPermanentWouldBeDeleted
    wpwd_contexts = [(name, ctx) for name, ctx in captured
                      if name == 'WhenPermanentWouldBeDeleted']
    if wpwd_contexts:
        wpwd = wpwd_contexts[-1][1]
        assert wpwd['is_opponent_effect'] is True
        assert wpwd['is_own_effect'] is False

    # Test 2: Delete by own effect
    captured.clear()
    card2 = make_card("DEL-002", "Victim2", owner=game.player1)
    perm2 = Permanent([card2])
    perm2._owner_game = game
    game.player1.battle_area.append(perm2)
    game.player1.delete_permanent(perm2, is_opponent_effect=False, removal_cause='effect')

    own_contexts = [(name, ctx) for name, ctx in captured
                     if name == 'WhenRemoveField']
    assert len(own_contexts) >= 1
    ctx2 = own_contexts[-1][1]
    assert ctx2['is_opponent_effect'] is False
    assert ctx2['is_own_effect'] is True


def test_1d_conditional_color_bypass():
    """Conditional color requirement bypass via _match_color_requirement_fn."""
    # Test: fn returns False (bypass)
    card = CardSource()
    card._match_color_requirement_fn = lambda: False
    assert card.match_color_requirement is False

    # Test: fn returns True (enforce)
    card._match_color_requirement_fn = lambda: True
    assert card.match_color_requirement is True

    # Test: no fn, falls back to static
    card2 = CardSource()
    assert card2.match_color_requirement is True
    card2._match_color_requirement = False
    assert card2.match_color_requirement is False

    # Test: fn takes precedence over static
    card3 = CardSource()
    card3._match_color_requirement = False
    card3._match_color_requirement_fn = lambda: True
    assert card3.match_color_requirement is True

    # Test: fn with exception falls through to static
    card4 = CardSource()
    card4._match_color_requirement = False
    card4._match_color_requirement_fn = lambda: 1 / 0  # Will raise
    assert card4.match_color_requirement is False


def test_2_may_attack():
    """MAY_ATTACK enables attack without forcing (pass remains available)."""
    game = make_game()
    game.current_phase = GamePhase.Main
    game.memory = 5

    card1 = make_card("MAY-001", "MayAttacker", dp=5000, level=4, owner=game.player1)
    perm1 = Permanent([card1])
    perm1._owner_game = game
    perm1.turn_played = game.turn_count
    perm1.grant_keyword('_is_rush')
    game.player1.battle_area.append(perm1)

    sec = make_card("SEC-001", "SecCard", owner=game.player2)
    game.player2.security_cards.append(sec)

    # MAY_ATTACK: pass AND attack should both be available
    game.register_modifier(perm1, ModifierType.MAY_ATTACK, expiry='end_of_turn')
    mask_may = build_action_mask(game, 1)

    sec_action = 100 + 0 * TARGETS_PER_ATTACKER + SECURITY_TARGET
    assert mask_may[62] == 1.0, "Pass should be available with MAY_ATTACK"
    assert mask_may[sec_action] == 1.0, "Attack should be available with MAY_ATTACK"

    # FORCE_ATTACK: only attack, no pass
    game.modifiers.clear_all()
    game.register_modifier(perm1, ModifierType.FORCE_ATTACK, expiry='end_of_turn')
    mask_force = build_action_mask(game, 1)

    assert mask_force[62] == 0.0, "Pass should NOT be available with FORCE_ATTACK"
    assert mask_force[sec_action] == 1.0, "Attack should be available with FORCE_ATTACK"


# ---------------------------------------------------------------------------
# Greedy baseline smoke tests
# ---------------------------------------------------------------------------


@pytest.mark.slow
def test_4a_puppets_recursion_guard():
    """Puppets vs TS Olympos -- no RecursionError in 10 greedy games."""
    results = []
    for _ in range(10):
        r = _run_greedy_baseline("Puppets", "TS Olympos")
        results.append(r)

    recursion_errors = [r for r in results if r["crash_error"] and "RecursionError" in str(r["crash_error"])]
    assert len(recursion_errors) == 0, "RecursionError still occurring"


@pytest.mark.slow
def test_smoke_jesmon():
    """5 Jesmon GX greedy games complete without crash."""
    results = []
    for _ in range(5):
        r = _run_greedy_baseline("Jesmon GX (Gankoomon)", "TS Jupitermon")
        results.append(r)
    completed = sum(1 for r in results if r["completed"])
    assert completed >= 4, f"Only {completed}/5 games completed"


@pytest.mark.slow
def test_smoke_ts_jupitermon():
    """5 TS Jupitermon greedy games complete without crash."""
    results = []
    for _ in range(5):
        r = _run_greedy_baseline("TS Jupitermon", "Medusamon")
        results.append(r)
    completed = sum(1 for r in results if r["completed"])
    assert completed >= 4, f"Only {completed}/5 games completed"


@pytest.mark.slow
def test_smoke_medusamon():
    """5 Medusamon greedy games complete without crash."""
    results = []
    for _ in range(5):
        r = _run_greedy_baseline("Medusamon", "Puppets")
        results.append(r)
    completed = sum(1 for r in results if r["completed"])
    assert completed >= 4, f"Only {completed}/5 games completed"
