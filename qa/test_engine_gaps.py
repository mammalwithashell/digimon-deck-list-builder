"""Debug game verification tests for engine gap resolutions.

Tests the 6 engine gaps resolved on 2026-03-17 plus stability fixes.
Uses in-process engine directly — no server required for unit tests.
Greedy baselines also run in-process.
"""

import sys
import traceback
sys.path.insert(0, str(__import__('pathlib').Path(__file__).parent.parent))

from qa.debug_game_helper import run_greedy_baseline

from digimon_gym.engine.game import Game
from digimon_gym.engine.core.permanent import Permanent
from digimon_gym.engine.core.player import Player
from digimon_gym.engine.core.card_source import CardSource
from digimon_gym.engine.core.entity_base import CEntity_Base
from digimon_gym.engine.data.enums import GamePhase, EffectTiming, CardColor, CardKind
from digimon_gym.engine.interfaces.modifiers import ModifierType


def make_card(card_id="TEST-001", name="TestDigimon", kind=CardKind.Digimon,
              dp=5000, level=4, play_cost=5, traits=None, colors=None, owner=None):
    """Helper to create a CardSource with minimal setup."""
    entity = CEntity_Base()
    entity.card_id = card_id
    entity.card_name_eng = name
    entity.card_kind = kind
    entity.dp = dp
    entity.level = level
    entity.play_cost = play_cost
    entity.type_eng = traits or []
    entity.card_colors = colors or [CardColor.Red]
    cs = CardSource()
    cs.set_base_data(entity, owner)
    return cs


def make_game():
    """Create a properly initialized Game."""
    game = Game()
    game.start_game()
    return game


def test_1a_change_security_attack_wiring():
    """Test 1A: CHANGE_SECURITY_ATTACK modifier is wired into permanent.security_attack_modifier()."""
    print("\n=== Test 1A: CHANGE_SECURITY_ATTACK wiring ===")
    game = make_game()

    # Create a Digimon with static SA modifier from effect
    card = make_card("TEST-SA", "TestMon", owner=game.player1)
    perm = Permanent([card])
    perm._owner_game = game
    game.player1.battle_area.append(perm)

    # Baseline: no SA modifiers
    sa0 = perm.security_attack_modifier()
    print(f"  Baseline SA modifier: {sa0}")
    assert sa0 == 0, f"Expected 0 baseline, got {sa0}"

    # Register CHANGE_SECURITY_ATTACK +2 from registry
    game.register_modifier(
        perm, ModifierType.CHANGE_SECURITY_ATTACK,
        value_fn=lambda cur, t, c: cur + 2,
        expiry='end_of_turn',
    )
    sa1 = perm.security_attack_modifier()
    print(f"  After +2 registry modifier: {sa1}")
    assert sa1 == 2, f"Expected 2, got {sa1}"

    # Register another -1 from registry (should stack)
    game.register_modifier(
        perm, ModifierType.CHANGE_SECURITY_ATTACK,
        value_fn=lambda cur, t, c: cur - 1,
        expiry='end_of_turn',
    )
    sa2 = perm.security_attack_modifier()
    print(f"  After +2 and -1 registry modifiers: {sa2}")
    assert sa2 == 1, f"Expected 1, got {sa2}"

    # Verify temp SA modifier also still works alongside registry
    perm._temp_sa_modifier = 3
    sa3 = perm.security_attack_modifier()
    print(f"  After adding temp SA=3 + registry mods: {sa3}")
    # 3 (temp) + 2 - 1 (registry) = 4
    assert sa3 == 4, f"Expected 4 (3 temp + 2 - 1 registry), got {sa3}"

    print("  PASS: CHANGE_SECURITY_ATTACK registry wiring works (stacks with temp modifier)")
    return True


def test_1b_cannot_attack_player():
    """Test 1B: CANNOT_ATTACK_PLAYER modifier prevents security attacks."""
    print("\n=== Test 1B: CANNOT_ATTACK_PLAYER modifier ===")
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
    print("  Before modifier: can_attack_player=True (correct)")

    # Register CANNOT_ATTACK_PLAYER
    game.register_modifier(perm1, ModifierType.CANNOT_ATTACK_PLAYER, expiry='end_of_turn')

    assert perm1.can_attack_player() is False
    print("  After modifier: can_attack_player=False (correct)")

    # Build action mask and verify
    from digimon_gym.engine.game.action_mask import build_action_mask
    from digimon_gym.engine.game.constants import SECURITY_TARGET, TARGETS_PER_ATTACKER
    mask = build_action_mask(game, 1)

    sec_action = 100 + 0 * TARGETS_PER_ATTACKER + SECURITY_TARGET
    digimon_action = 100 + 0 * TARGETS_PER_ATTACKER + 0

    print(f"  Action mask[security attack ({sec_action})]: {mask[sec_action]}")
    print(f"  Action mask[digimon attack ({digimon_action})]: {mask[digimon_action]}")

    assert mask[sec_action] == 0.0, "Security attack should be MASKED OUT"
    assert mask[digimon_action] == 1.0, "Digimon attack should be available"
    print("  PASS: CANNOT_ATTACK_PLAYER correctly masks security but allows Digimon attacks")
    return True


def test_1c_is_own_effect_context():
    """Test 1C: is_own_effect in WhenRemoveField context."""
    print("\n=== Test 1C: is_own_effect in removal context ===")
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
    print("  Opponent deletion: is_opponent_effect=True, is_own_effect=False (correct)")

    # Also check WhenPermanentWouldBeDeleted
    wpwd_contexts = [(name, ctx) for name, ctx in captured
                      if name == 'WhenPermanentWouldBeDeleted']
    if wpwd_contexts:
        wpwd = wpwd_contexts[-1][1]
        assert wpwd['is_opponent_effect'] is True
        assert wpwd['is_own_effect'] is False
        print("  WhenPermanentWouldBeDeleted also has correct flags")

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
    print("  Own deletion: is_opponent_effect=False, is_own_effect=True (correct)")

    print("  PASS: is_own_effect correctly propagated in all removal contexts")
    return True


def test_1d_conditional_color_bypass():
    """Test 1D: Conditional color requirement bypass via _match_color_requirement_fn."""
    print("\n=== Test 1D: Conditional color requirement bypass ===")

    # Test: fn returns False (bypass)
    card = CardSource()
    card._match_color_requirement_fn = lambda: False
    assert card.match_color_requirement is False
    print("  fn=False → bypass (correct)")

    # Test: fn returns True (enforce)
    card._match_color_requirement_fn = lambda: True
    assert card.match_color_requirement is True
    print("  fn=True → enforce (correct)")

    # Test: no fn, falls back to static
    card2 = CardSource()
    assert card2.match_color_requirement is True
    card2._match_color_requirement = False
    assert card2.match_color_requirement is False
    print("  No fn, static=False → bypass (correct)")

    # Test: fn takes precedence over static
    card3 = CardSource()
    card3._match_color_requirement = False
    card3._match_color_requirement_fn = lambda: True
    assert card3.match_color_requirement is True
    print("  fn=True overrides static=False → enforce (correct)")

    # Test: fn with exception falls through to static
    card4 = CardSource()
    card4._match_color_requirement = False
    card4._match_color_requirement_fn = lambda: 1/0  # Will raise
    assert card4.match_color_requirement is False
    print("  fn raises → falls back to static=False (correct)")

    print("  PASS: _match_color_requirement_fn works correctly")
    return True


def test_2_may_attack():
    """Test 2: MAY_ATTACK enables attack without forcing (pass remains available)."""
    print("\n=== Test 2: MAY_ATTACK vs FORCE_ATTACK ===")
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

    from digimon_gym.engine.game.action_mask import build_action_mask
    from digimon_gym.engine.game.constants import SECURITY_TARGET, TARGETS_PER_ATTACKER

    # MAY_ATTACK: pass AND attack should both be available
    game.register_modifier(perm1, ModifierType.MAY_ATTACK, expiry='end_of_turn')
    mask_may = build_action_mask(game, 1)

    sec_action = 100 + 0 * TARGETS_PER_ATTACKER + SECURITY_TARGET
    assert mask_may[62] == 1.0, "Pass should be available with MAY_ATTACK"
    assert mask_may[sec_action] == 1.0, "Attack should be available with MAY_ATTACK"
    print("  MAY_ATTACK: pass=available, attack=available (correct)")

    # FORCE_ATTACK: only attack, no pass
    game.modifiers.clear_all()
    game.register_modifier(perm1, ModifierType.FORCE_ATTACK, expiry='end_of_turn')
    mask_force = build_action_mask(game, 1)

    assert mask_force[62] == 0.0, "Pass should NOT be available with FORCE_ATTACK"
    assert mask_force[sec_action] == 1.0, "Attack should be available with FORCE_ATTACK"
    print("  FORCE_ATTACK: pass=blocked, attack=available (correct)")

    print("  PASS: MAY_ATTACK enables optional attack (pass remains available)")
    return True


def test_4a_puppets_recursion_guard():
    """Test 4A: Puppets vs TS Olympos — no RecursionError."""
    print("\n=== Test 4A: Puppets recursion guard (10 greedy games) ===")
    results = []
    for i in range(10):
        r = run_greedy_baseline("Puppets", "TS Olympos")
        results.append(r)
        status = "OK" if r["completed"] else f"FAIL: {str(r['crash_error'])[:80] if r['crash_error'] else 'timeout'}"
        print(f"  Game {i+1}: {status} (turns={r['turns']}, steps={r['steps']})")

    completed = sum(1 for r in results if r["completed"])
    recursion_errors = [r for r in results if r["crash_error"] and "RecursionError" in str(r["crash_error"])]
    print(f"\n  Completed: {completed}/10, RecursionErrors: {len(recursion_errors)}")

    if recursion_errors:
        print("  FAIL: RecursionError still occurring")
        return False
    print("  PASS: No RecursionError in 10 games")
    return True


def test_smoke_jesmon():
    """Smoke: 5 Jesmon GX greedy games."""
    print("\n=== Smoke: Jesmon GX (5 games) ===")
    results = []
    for i in range(5):
        r = run_greedy_baseline("Jesmon GX (Gankoomon)", "TS Jupitermon")
        results.append(r)
        status = "OK" if r["completed"] else f"FAIL: {str(r['crash_error'])[:60] if r['crash_error'] else 'timeout'}"
        print(f"  Game {i+1}: {status} (turns={r['turns']})")
    completed = sum(1 for r in results if r["completed"])
    print(f"  Completed: {completed}/5")
    return completed >= 4


def test_smoke_ts_jupitermon():
    """Smoke: 5 TS Jupitermon greedy games."""
    print("\n=== Smoke: TS Jupitermon (5 games) ===")
    results = []
    for i in range(5):
        r = run_greedy_baseline("TS Jupitermon", "Medusamon")
        results.append(r)
        status = "OK" if r["completed"] else f"FAIL: {str(r['crash_error'])[:60] if r['crash_error'] else 'timeout'}"
        print(f"  Game {i+1}: {status} (turns={r['turns']})")
    completed = sum(1 for r in results if r["completed"])
    print(f"  Completed: {completed}/5")
    return completed >= 4


def test_smoke_medusamon():
    """Smoke: 5 Medusamon greedy games."""
    print("\n=== Smoke: Medusamon (5 games) ===")
    results = []
    for i in range(5):
        r = run_greedy_baseline("Medusamon", "Puppets")
        results.append(r)
        status = "OK" if r["completed"] else f"FAIL: {str(r['crash_error'])[:60] if r['crash_error'] else 'timeout'}"
        print(f"  Game {i+1}: {status} (turns={r['turns']})")
    completed = sum(1 for r in results if r["completed"])
    print(f"  Completed: {completed}/5")
    return completed >= 4


if __name__ == "__main__":
    tests = [
        ("1A: CHANGE_SECURITY_ATTACK wiring", test_1a_change_security_attack_wiring),
        ("1B: CANNOT_ATTACK_PLAYER", test_1b_cannot_attack_player),
        ("1C: is_own_effect context", test_1c_is_own_effect_context),
        ("1D: Conditional color bypass", test_1d_conditional_color_bypass),
        ("2: MAY_ATTACK", test_2_may_attack),
        ("4A: Puppets recursion guard", test_4a_puppets_recursion_guard),
        ("Smoke: Jesmon GX", test_smoke_jesmon),
        ("Smoke: TS Jupitermon", test_smoke_ts_jupitermon),
        ("Smoke: Medusamon", test_smoke_medusamon),
    ]

    results = {}
    for name, fn in tests:
        try:
            passed = fn()
            results[name] = "PASS" if passed else "FAIL"
        except Exception as e:
            results[name] = f"ERROR: {e}"
            traceback.print_exc()

    print("\n" + "=" * 60)
    print("RESULTS SUMMARY")
    print("=" * 60)
    for name, status in results.items():
        icon = "PASS" if status == "PASS" else "FAIL"
        print(f"  [{icon}] {name}: {status}")

    total = len(results)
    passed = sum(1 for s in results.values() if s == "PASS")
    print(f"\n  {passed}/{total} tests passed")
