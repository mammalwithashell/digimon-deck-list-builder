"""Behavioral tests for BT15-008 Muchomon (Lv.3 Red, Cost 3).

Card text (from cards.json):
[Your Turn] [Once Per Turn] When one of your red Digimon attacks a player,
    <Draw 1> (Draw 1 card from your deck.)

C# reference (BT15_008.cs):
- Timing: EffectTiming.OnAllyAttack (fires for any ally attacking, not self)
- PermanentCondition: checks the attacking permanent is a red Digimon on
    owner's battle area
- CanUseCondition: IsExistOnBattleArea, IsOwnerTurn,
    CanTriggerOnPermanentAttack(hashtable, PermanentCondition),
    DefendingPermanent == null (attacks PLAYER, not Digimon)
- Action: Draw 1

Key faithfulness checks:
1. Must use OnAllyAttack timing (triggers for ANY of your Digimon, not just self)
2. Must check that the attacking Digimon is RED
3. Must check that the attack target is a PLAYER (not a Digimon)
4. Once Per Turn
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT15008Muchomon:
    """Tests for BT15-008 Muchomon."""

    def _get_draw_effect(self, perm):
        """Get the draw 1 on-attack effect from Muchomon's top card."""
        top = perm.top_card
        effects = top.effect_list(None) if top else []
        on_attack = [e for e in effects if getattr(e, 'is_on_attack', False)]
        assert len(on_attack) >= 1, "Muchomon should have at least 1 on-attack effect"
        return on_attack[0]

    # ------------------------------------------------------------------
    # Effect structure
    # ------------------------------------------------------------------

    def test_effect_exists_with_on_attack(self, debug_runner):
        """Should have an on-attack effect."""
        runner = debug_runner(
            deck1=["BT15-008"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT15-008"])
        effect = self._get_draw_effect(perm)
        assert effect is not None

    def test_effect_once_per_turn(self, debug_runner):
        """The effect should be Once Per Turn."""
        runner = debug_runner(
            deck1=["BT15-008"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT15-008"])
        effect = self._get_draw_effect(perm)
        assert effect.max_count_per_turn == 1, "Should be Once Per Turn"

    # ------------------------------------------------------------------
    # CRITICAL: Timing must be OnAllyAttack (not OnUseAttack)
    # ------------------------------------------------------------------

    def test_timing_is_ally_attack_not_self_attack(self, debug_runner):
        """CRITICAL: Effect must use OnAllyAttack timing so it triggers when
        ANY of your red Digimon attacks, not just when Muchomon itself attacks.

        C# reference uses EffectTiming.OnAllyAttack with PermanentCondition
        that checks the attacking permanent is red and on owner's battle area.

        Bug: Current script uses OnUseAttack which only fires for self-attacks.
        """
        runner = debug_runner(
            deck1=["BT15-008"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT15-008"])
        effect = self._get_draw_effect(perm)

        # The C# uses OnAllyAttack timing
        assert effect.timing == EffectTiming.OnAllyAttack, (
            f"Effect should use OnAllyAttack timing (triggers for any ally), "
            f"but uses {effect.timing}. C# ref: timing == EffectTiming.OnAllyAttack"
        )

    # ------------------------------------------------------------------
    # CRITICAL: Must check attacking Digimon is RED
    # ------------------------------------------------------------------

    def test_condition_requires_red_attacker(self, debug_runner):
        """Condition must check that the attacking Digimon is red.

        C# PermanentCondition checks: permanent.TopCard.CardColors.Contains(CardColor.Red)
        A non-red Digimon attacking should not trigger the effect.

        Bug: Current script has no color check in condition.
        """
        runner = debug_runner(
            deck1=["BT15-008"] * 4 + ["ST2-04"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm_muchomon = runner.place_on_field(1, ["BT15-008"])
        # Place a BLUE Digimon (ST2-04 Bearmon)
        perm_blue = runner.place_on_field(1, ["ST2-04"])
        game = runner.game

        effect = self._get_draw_effect(perm_muchomon)

        # Simulate context where a blue Digimon is attacking
        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm_muchomon,
            'attacker': perm_blue,
        }
        result = effect.can_use_condition(context)
        assert not result, (
            "Condition should FAIL when the attacking Digimon is not red. "
            "C# checks permanent.TopCard.CardColors.Contains(CardColor.Red)"
        )

    def test_condition_passes_red_attacker(self, debug_runner):
        """Condition should pass when a red Digimon attacks."""
        runner = debug_runner(
            deck1=["BT15-008"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm_muchomon = runner.place_on_field(1, ["BT15-008"])
        # ST1-03 Agumon is Red
        perm_red = runner.place_on_field(1, ["ST1-03"])
        game = runner.game

        effect = self._get_draw_effect(perm_muchomon)

        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm_muchomon,
            'attacker': perm_red,
        }
        result = effect.can_use_condition(context)
        assert result, "Condition should PASS when a red Digimon attacks"

    # ------------------------------------------------------------------
    # CRITICAL: Must check attack target is PLAYER (not Digimon)
    # ------------------------------------------------------------------

    def test_condition_requires_player_attack_target(self, debug_runner):
        """Condition must check that the attack target is a player, not a Digimon.

        C# reference: GManager.instance.attackProcess.DefendingPermanent == null
        When DefendingPermanent is null, it means the attack is against the player.

        Bug: Current script has no check for attack target.
        """
        runner = debug_runner(
            deck1=["BT15-008"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm_muchomon = runner.place_on_field(1, ["BT15-008"])
        perm_attacker = runner.place_on_field(1, ["ST1-03"])
        perm_defender = runner.place_on_field(2, ["ST1-03"])
        game = runner.game

        effect = self._get_draw_effect(perm_muchomon)

        # When attacking a Digimon (defender exists), should NOT trigger
        # The engine tracks this via pending_attack.original_target
        from digimon_gym.engine.game.constants import PendingAttack
        game.pending_attack = PendingAttack(
            attacker=perm_attacker,
            original_target=perm_defender,
            effective_target=perm_defender,
        )

        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm_muchomon,
            'attacker': perm_attacker,
        }
        result = effect.can_use_condition(context)
        assert not result, (
            "Condition should FAIL when attacking a Digimon (not a player). "
            "C# checks DefendingPermanent == null"
        )

    # ------------------------------------------------------------------
    # Condition: requires field and your turn
    # ------------------------------------------------------------------

    def test_condition_requires_field(self, debug_runner):
        """Condition should fail when Muchomon is not on the field."""
        runner = debug_runner(
            deck1=["BT15-008"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT15-008"])
        effect = self._get_draw_effect(perm)

        runner.game.player1.battle_area.remove(perm)
        result = effect.can_use_condition({})
        assert not result, "Condition should fail when not on field"

    def test_condition_requires_your_turn(self, debug_runner):
        """Condition should fail on opponent's turn."""
        runner = debug_runner(
            deck1=["BT15-008"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT15-008"])
        game = runner.game
        effect = self._get_draw_effect(perm)

        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        result = effect.can_use_condition({
            'game': game,
            'player': game.player1,
            'permanent': perm,
        })
        assert not result, "Condition should fail on opponent's turn"

    # ------------------------------------------------------------------
    # Process: Draw 1
    # ------------------------------------------------------------------

    def test_process_draws_one_card(self, debug_runner):
        """Process should draw exactly 1 card."""
        runner = debug_runner(
            deck1=["BT15-008"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT15-008"])
        game = runner.game
        effect = self._get_draw_effect(perm)

        hand_before = len(game.player1.hand_cards)
        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert len(game.player1.hand_cards) == hand_before + 1, \
            "Should draw exactly 1 card"
