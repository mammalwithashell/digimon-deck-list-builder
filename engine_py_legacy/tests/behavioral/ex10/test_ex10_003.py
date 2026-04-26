"""Behavioral tests for EX10-003 Tumblemon (Lv.2, Black DigiEgg, Rock/LIBERATOR).

Card text:
Inherited Effect [Opponent's Turn] [Once Per Turn] When one of your opponent's
Digimon attacks, by trashing 3 [Mineral] or [Rock] trait cards from this
Digimon's digivolution cards, end that attack.

Key cards for test setups:
- EX10-003 Tumblemon: Lv.2, Rock trait (provides inherited effect)
- BT2-054 Gotsumon:   Lv.3, Rock trait
- BT4-065 Gotsumon:   Lv.3, Rock trait
- BT4-066 Golemon:    Lv.4, Mineral trait (top card of test stacks)
- ST1-03 Agumon:       Lv.3, Reptile trait (no Mineral/Rock — as a non-qualifying source)
"""

import pytest
from engine_py_legacy.engine.data.enums import GamePhase


# Digivolution stack: bottom-to-top order per place_on_field convention.
# EX10-003 (Rock) | BT2-054 (Rock) | BT4-065 (Rock) | BT4-066 (Mineral, top)
# Qualifying sources under top: EX10-003, BT2-054, BT4-065 = 3 Mineral/Rock
STACK_3_QUALIFYING = ["EX10-003", "BT2-054", "BT4-065", "BT4-066"]

# Only 2 qualifying sources under top (ST1-03 replaces one Rock)
STACK_2_QUALIFYING = ["EX10-003", "BT2-054", "ST1-03", "BT4-066"]

# 4 qualifying sources: add an extra Mineral card
STACK_4_QUALIFYING = ["EX10-003", "BT2-054", "BT4-065", "BT4-066", "BT4-066"]


def _get_inherited_effect(perm):
    """Find the EX10-003 inherited 'end attack' effect on a permanent."""
    from engine_py_legacy.engine.data.enums import EffectTiming
    for eff in perm.effect_list(EffectTiming.NoTiming):
        if eff.is_inherited_effect and 'EX10-003' in eff.effect_name:
            return eff
    return None


@pytest.mark.behavioral
class TestEX10003Tumblemon:
    """Tests for EX10-003 Tumblemon inherited effect."""

    # ── Effect structure tests ──────────────────────────────────────

    def test_is_inherited_effect(self, debug_runner):
        """The effect must be flagged as inherited so it fires from digivolution stack."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, STACK_3_QUALIFYING)
        eff = _get_inherited_effect(perm)
        assert eff is not None, "Should find inherited effect on permanent"
        assert eff.is_inherited_effect is True

    def test_uses_on_ally_attack_timing(self, debug_runner):
        """The effect must use OnAllyAttack timing (not OnUseAttack) so it fires
        from the defender's side when the opponent attacks.

        execute_effects(OnAllyAttack) scans both battle areas in combat.py
        _start_attack; OnUseAttack only fires for the attacker itself.
        """
        from engine_py_legacy.engine.data.enums import EffectTiming
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, STACK_3_QUALIFYING)
        eff = _get_inherited_effect(perm)
        assert eff is not None
        assert eff.timing == EffectTiming.OnAllyAttack, \
            f"Timing should be OnAllyAttack, got {eff.timing}"

    def test_is_optional(self, debug_runner):
        """The effect should be optional ('by trashing' = player can choose not to)."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, STACK_3_QUALIFYING)
        eff = _get_inherited_effect(perm)
        assert eff is not None
        assert eff.is_optional is True

    def test_once_per_turn(self, debug_runner):
        """The effect should be once per turn."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, STACK_3_QUALIFYING)
        eff = _get_inherited_effect(perm)
        assert eff is not None
        assert eff.max_count_per_turn == 1

    # ── Condition tests ─────────────────────────────────────────────

    def test_condition_passes_with_3_qualifying_sources(self, debug_runner):
        """Condition should pass when there are 3+ Mineral/Rock sources under top."""
        runner = debug_runner(initial_memory=0)
        # Player 2 is the one with EX10-003 (defender)
        perm = runner.place_on_field(2, STACK_3_QUALIFYING)
        game = runner.game

        # Set it to be player 1's turn (opponent's turn for player 2)
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        eff = _get_inherited_effect(perm)
        assert eff is not None
        eff.set_effect_source_permanent(perm)
        ctx = {
            'game': game, 'player': game.player2, 'permanent': perm,
            'card': eff.effect_source_card,
        }
        assert eff.can_use_condition(ctx) is True

    def test_condition_fails_with_fewer_than_3(self, debug_runner):
        """Condition should fail when fewer than 3 Mineral/Rock sources exist."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(2, STACK_2_QUALIFYING)
        game = runner.game

        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        eff = _get_inherited_effect(perm)
        assert eff is not None
        eff.set_effect_source_permanent(perm)
        ctx = {
            'game': game, 'player': game.player2, 'permanent': perm,
            'card': eff.effect_source_card,
        }
        assert eff.can_use_condition(ctx) is False

    def test_condition_fails_on_own_turn(self, debug_runner):
        """Condition should fail when it's the owner's own turn."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, STACK_3_QUALIFYING)
        game = runner.game

        # Player 1's turn (own turn for the perm owner)
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        eff = _get_inherited_effect(perm)
        assert eff is not None
        eff.set_effect_source_permanent(perm)
        ctx = {
            'game': game, 'player': game.player1, 'permanent': perm,
            'card': eff.effect_source_card,
        }
        assert eff.can_use_condition(ctx) is False

    # ── Process / selection tests ───────────────────────────────────

    def test_process_opens_source_selection(self, debug_runner):
        """Process callback should open a SelectSource phase for player to choose cards."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(2, STACK_3_QUALIFYING)
        game = runner.game

        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        eff = _get_inherited_effect(perm)
        assert eff is not None
        eff.set_effect_source_permanent(perm)

        ctx = {
            'game': game, 'player': game.player2, 'permanent': perm,
            'card': eff.effect_source_card,
        }
        eff.on_process_callback(ctx)

        # Should have created a source selection for player 2
        assert game.pending_selection is not None, \
            "Process should create a pending selection for source card choice"
        assert game.current_phase == GamePhase.SelectSource, \
            "Should enter SelectSource phase"
        assert game.pending_selection.selecting_player is game.player2

    def test_process_trashes_player_selected_cards(self, debug_runner):
        """Player should select exactly which 3 Mineral/Rock cards to trash."""
        runner = debug_runner(initial_memory=0)
        # Use 4 qualifying so player has choice
        stack = ["EX10-003", "BT2-054", "BT4-065", "BT4-066", "BT4-066"]
        # bottom-to-top: [EX10-003(0), BT2-054(1), BT4-065(2), BT4-066(3), BT4-066(4=top)]
        perm = runner.place_on_field(2, stack)
        game = runner.game

        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        # Set up a pending attack so force_end_attack has something to end
        from engine_py_legacy.engine.game.constants import PendingAttack
        attacker = runner.place_on_field(1, ["ST1-03"])
        game.pending_attack = PendingAttack(
            attacker=attacker,
            original_target=game.player2,
            effective_target=game.player2,
        )

        # Record initial state
        initial_source_count = len(perm.card_sources)
        initial_trash_count = len(game.player2.trash_cards)

        eff = _get_inherited_effect(perm)
        assert eff is not None
        eff.set_effect_source_permanent(perm)

        ctx = {
            'game': game, 'player': game.player2, 'permanent': perm,
            'card': eff.effect_source_card,
        }
        eff.on_process_callback(ctx)

        # Find field index and compute base for source actions
        from engine_py_legacy.engine.game.constants import SOURCES_PER_FIELD
        field_idx = game.player2.battle_area.index(perm)
        base = 2000 + field_idx * SOURCES_PER_FIELD

        # Select 3 cards one at a time (choose specific indices)
        # Sources: [EX10-003(0), BT2-054(1), BT4-065(2), BT4-066(3), BT4-066(4=top)]
        # We'll pick index 0 (EX10-003), then index 1 (BT2-054), then index 2 (BT4-065)
        for step in range(3):
            assert game.pending_selection is not None, \
                f"Selection {step+1}/3 should be pending"
            assert game.current_phase == GamePhase.SelectSource
            # Pick the first valid index
            valid = game.pending_selection.valid_indices
            assert len(valid) > 0, f"Should have valid indices for selection {step+1}"
            game.decode_action(valid[0], game.player2.player_id)

        # After 3 selections, cards should be trashed
        assert len(perm.card_sources) == initial_source_count - 3, \
            "3 cards should have been removed from digivolution stack"
        assert len(game.player2.trash_cards) == initial_trash_count + 3, \
            "3 cards should be in trash"
        # Attack should have been ended
        assert game.pending_attack.is_end_attack is True, \
            "force_end_attack should have been called"

    def test_does_not_auto_select(self, debug_runner):
        """With 4 qualifying sources, the player must choose (not auto-selected)."""
        runner = debug_runner(initial_memory=0)
        # 4 qualifying under top: player has real choice
        stack = ["EX10-003", "BT2-054", "BT4-065", "BT4-066", "BT4-066"]
        perm = runner.place_on_field(2, stack)
        game = runner.game

        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        from engine_py_legacy.engine.game.constants import PendingAttack
        attacker = runner.place_on_field(1, ["ST1-03"])
        game.pending_attack = PendingAttack(
            attacker=attacker,
            original_target=game.player2,
            effective_target=game.player2,
        )

        eff = _get_inherited_effect(perm)
        assert eff is not None
        eff.set_effect_source_permanent(perm)

        ctx = {
            'game': game, 'player': game.player2, 'permanent': perm,
            'card': eff.effect_source_card,
        }
        eff.on_process_callback(ctx)

        # With 4 qualifying sources, first selection should offer 4 choices
        assert game.pending_selection is not None
        valid_count = len(game.pending_selection.valid_indices)
        assert valid_count == 4, \
            f"Should offer 4 valid sources to choose from, got {valid_count}"

    def test_no_card_sources_remove_antipattern(self):
        """The script must NOT use perm.card_sources.remove() directly.
        It should use request_selection + player-driven source selection."""
        import inspect
        from engine_py_legacy.engine.data.scripts.ex10.ex10_003 import EX10_003

        source = inspect.getsource(EX10_003)
        # Should not have direct list removal (anti-pattern from old code)
        assert 'card_sources.remove(c)' not in source, \
            "Should not auto-remove from card_sources in a loop"
        # Should use request_selection / SelectSource for player choice
        assert 'SelectSource' in source, \
            "Should use SelectSource phase for player-driven selection"
        assert 'request_selection' in source, \
            "Should use game.request_selection() for player choice"

    def test_condition_excludes_top_card_from_count(self, debug_runner):
        """The condition should not count the top card as a qualifying source,
        even if the top card has Mineral/Rock trait."""
        runner = debug_runner(initial_memory=0)
        # Stack where top card is Mineral but only 2 sources below are Mineral/Rock
        # BT4-066 (Mineral) is top, under: EX10-003 (Rock), BT2-054 (Rock), ST1-03 (Reptile)
        # Qualifying under top: 2 (EX10-003 + BT2-054), not enough even though top is Mineral
        stack = ["EX10-003", "BT2-054", "ST1-03", "BT4-066"]
        perm = runner.place_on_field(2, stack)
        game = runner.game

        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        eff = _get_inherited_effect(perm)
        assert eff is not None
        eff.set_effect_source_permanent(perm)
        ctx = {
            'game': game, 'player': game.player2, 'permanent': perm,
            'card': eff.effect_source_card,
        }
        # Top card is Mineral but only 2 qualifying under it => condition fails
        assert eff.can_use_condition(ctx) is False

    def test_full_attack_flow_ends_attack(self, debug_runner):
        """Integration test: opponent attacks, effect fires via execute_effects(OnAllyAttack),
        player selects 3 sources, attack is ended via force_end_attack()."""
        from engine_py_legacy.engine.data.enums import EffectTiming
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(2, STACK_3_QUALIFYING)
        game = runner.game

        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        from engine_py_legacy.engine.game.constants import PendingAttack
        attacker = runner.place_on_field(1, ["ST1-03"])
        game.pending_attack = PendingAttack(
            attacker=attacker,
            original_target=game.player2,
            effective_target=game.player2,
        )

        # Fire OnAllyAttack effects (what combat.py _start_attack does)
        game.execute_effects(EffectTiming.OnAllyAttack, {"attacker": attacker})

        # The effect should have created a source selection
        assert game.pending_selection is not None, \
            "OnAllyAttack effect should trigger source selection"
        assert game.current_phase == GamePhase.SelectSource

        # Resolve all 3 selections
        for step in range(3):
            assert game.pending_selection is not None, \
                f"Selection {step+1}/3 should be pending"
            valid = game.pending_selection.valid_indices
            assert len(valid) > 0
            game.decode_action(valid[0], game.player2.player_id)

        # Attack should be flagged as ended
        assert game.pending_attack.is_end_attack is True, \
            "After trashing 3 sources, the attack should be force-ended"

        # The 3 sources should be in player 2's trash
        assert len(game.player2.trash_cards) >= 3, \
            "Trashed sources should be in player's trash"

    def test_only_mineral_rock_sources_offered(self, debug_runner):
        """Source selection should only offer Mineral/Rock trait cards, not other sources."""
        runner = debug_runner(initial_memory=0)
        # Stack with 3 qualifying + 1 non-qualifying under top
        stack = ["EX10-003", "ST1-03", "BT2-054", "BT4-065", "BT4-066"]
        # Sources: [EX10-003(0,Rock), ST1-03(1,Reptile), BT2-054(2,Rock), BT4-065(3,Rock), BT4-066(4,top/Mineral)]
        perm = runner.place_on_field(2, stack)
        game = runner.game

        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        eff = _get_inherited_effect(perm)
        assert eff is not None
        eff.set_effect_source_permanent(perm)

        ctx = {
            'game': game, 'player': game.player2, 'permanent': perm,
            'card': eff.effect_source_card,
        }
        eff.on_process_callback(ctx)

        # Should offer only the 3 Mineral/Rock sources, not ST1-03
        assert game.pending_selection is not None
        valid = game.pending_selection.valid_indices

        from engine_py_legacy.engine.game.constants import SOURCES_PER_FIELD
        field_idx = game.player2.battle_area.index(perm)
        base = 2000 + field_idx * SOURCES_PER_FIELD

        # ST1-03 is at index 1, so action_id = base+1 should NOT be valid
        st1_action = base + 1
        assert st1_action not in valid, \
            "ST1-03 (Reptile) should NOT be offered as a valid source to trash"
        assert len(valid) == 3, \
            "Should offer exactly 3 valid sources (Rock/Mineral only)"
