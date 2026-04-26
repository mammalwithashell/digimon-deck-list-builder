"""Behavioral tests for EX11-020 Hanimon (Lv.3, Yellow, Puppet/LIBERATOR).

Card text:
  Alt-digi: From [Kyaromon] for cost 0

  [On Deletion] If deleted other than in battle, you may play 1 [Shoemon]
      from your hand without paying the cost.

  --- Inherited ---
  [Opponent's Turn][Once Per Turn] When one of your opponent's Digimon attacks,
      by deleting 1 of your other Digimon, end that attack.

C# reference: EX11_020.cs
  - On Deletion uses CanTriggerOnDeletion(hashtable, card) && !IsByBattle(hashtable)
  - Shoemon filter uses EqualsCardName("Shoemon") -- exact match
  - Inherited uses EffectTiming.OnAllyAttack (not OnUseAttack)
  - Inherited hash: "StopAttack_EX11-020"
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX11020OnDeletion:
    """Tests for EX11-020 Hanimon [On Deletion] effect."""

    def test_on_deletion_by_effect_enters_selection(self, debug_runner):
        """When Hanimon is deleted by an effect (not battle), the On Deletion
        effect should fire and enter a selection phase to play Shoemon."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place Hanimon on P1's field
        perm = runner.place_on_field(1, ["EX11-020"])

        # Add a Shoemon to P1's hand
        runner.inject_card(1, "EX11-019", "hand")  # EX11-019 is Shoemon

        # Delete by effect (not battle) -- should trigger On Deletion
        game.player1.delete_permanent(perm, is_battle=False)

        # The effect should have entered a selection phase for playing Shoemon
        assert game.pending_selection is not None, (
            "On Deletion should enter selection phase for playing Shoemon"
        )

        # Resolve the selection to play Shoemon
        hand_before = len(game.player1.hand_cards)
        valid = game.pending_selection.valid_indices
        assert len(valid) >= 1, "Should have at least 1 valid selection (Shoemon)"
        game.decode_action(valid[0], game.current_player_id)

        # Shoemon should be played from hand
        assert len(game.player1.hand_cards) == hand_before - 1, (
            "Shoemon should be played from hand"
        )

    def test_on_deletion_in_battle_does_not_trigger(self, debug_runner):
        """When Hanimon is deleted in battle, the On Deletion effect should
        NOT fire (card says 'If deleted other than in battle')."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place Hanimon on P1's field
        perm = runner.place_on_field(1, ["EX11-020"])

        # Add a Shoemon to P1's hand
        runner.inject_card(1, "EX11-019", "hand")

        hand_before = len(game.player1.hand_cards)

        # Delete in battle -- should NOT trigger On Deletion
        game.player1.delete_permanent(perm, is_battle=True)

        # No selection phase and hand unchanged
        assert game.pending_selection is None, (
            "On Deletion should NOT enter selection when deleted in battle"
        )
        assert len(game.player1.hand_cards) == hand_before, (
            "Shoemon should NOT be played when Hanimon is deleted in battle"
        )

    def test_shoemon_filter_exact_match_no_shoe_shoemon(self, debug_runner):
        """The play filter should only match [Shoemon] exactly, not [ShoeShoemon]."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place Hanimon on P1's field
        perm = runner.place_on_field(1, ["EX11-020"])

        # Add only ShoeShoemon (NOT Shoemon) to hand
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT22-032", "hand")  # ShoeShoemon

        hand_before = len(game.player1.hand_cards)

        # Delete by effect -- On Deletion fires but should NOT find valid target
        game.player1.delete_permanent(perm, is_battle=False)

        # No selection phase entered (no valid Shoemon targets)
        assert game.pending_selection is None, (
            "Should NOT enter selection when only ShoeShoemon is in hand -- "
            "filter must be exact match for [Shoemon]"
        )
        assert len(game.player1.hand_cards) == hand_before, (
            "ShoeShoemon should NOT be played"
        )

    def test_on_deletion_condition_checks_is_battle_context(self, debug_runner):
        """Verify the On Deletion condition function checks is_battle from context."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        perm = runner.place_on_field(1, ["EX11-020"])
        card = perm.card_sources[0]

        # Get the On Deletion effect
        effects = card.effect_list(EffectTiming.OnDestroyedAnyone)
        on_del = [e for e in effects if e.is_on_deletion]
        assert len(on_del) == 1, "Should have exactly 1 On Deletion effect"

        effect = on_del[0]

        # With is_battle=False -- condition should pass
        ctx_no_battle = {
            "game": game, "player": game.player1, "permanent": perm,
            "card": card, "is_battle": False, "removal_cause": "effect",
        }
        assert effect.can_use_condition(ctx_no_battle), (
            "On Deletion condition should pass when is_battle=False"
        )

        # With is_battle=True -- condition should fail
        ctx_battle = {
            "game": game, "player": game.player1, "permanent": perm,
            "card": card, "is_battle": True, "removal_cause": "battle",
        }
        assert not effect.can_use_condition(ctx_battle), (
            "On Deletion condition should fail when is_battle=True"
        )


@pytest.mark.behavioral
class TestEX11020InheritedStopAttack:
    """Tests for EX11-020 inherited [Opponent's Turn] end attack effect."""

    def test_inherited_uses_on_ally_attack_timing(self, debug_runner):
        """The inherited effect must use OnAllyAttack timing (not OnUseAttack),
        so it fires on other permanents when any Digimon attacks."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place a Lv.4 digivolved over Hanimon on P1's field
        perm = runner.place_on_field(1, ["EX11-020", "ST1-05"], turn_played=-1)

        # Find inherited effect from Hanimon (bottom card)
        hanimon_card = perm.card_sources[0]  # Hanimon is the bottom card
        effects = hanimon_card.effect_list(EffectTiming.OnAllyAttack)
        inherited = [e for e in effects if e.is_inherited_effect and e.is_on_attack]

        assert len(inherited) >= 1, (
            "Hanimon should have an inherited effect on OnAllyAttack timing"
        )

    def test_inherited_does_not_use_on_use_attack(self, debug_runner):
        """The inherited effect must NOT use OnUseAttack timing."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        perm = runner.place_on_field(1, ["EX11-020", "ST1-05"])

        hanimon_card = perm.card_sources[0]
        # effect_list ignores timing param, so filter by .timing attribute
        effects = hanimon_card.effect_list(None)
        inherited_on_use_attack = [
            e for e in effects
            if e.is_inherited_effect and e.is_on_attack
            and e.timing == EffectTiming.OnUseAttack
        ]

        assert len(inherited_on_use_attack) == 0, (
            "Hanimon should NOT have inherited effects with OnUseAttack timing"
        )

    def test_inherited_hash_is_unique(self, debug_runner):
        """The inherited effect hash must be unique to EX11-020, not collide
        with other cards (e.g., EX9-027)."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        perm = runner.place_on_field(1, ["EX11-020", "ST1-05"])

        hanimon_card = perm.card_sources[0]
        effects = hanimon_card.effect_list(EffectTiming.OnAllyAttack)
        inherited = [e for e in effects if e.is_inherited_effect and e.is_on_attack]

        assert len(inherited) >= 1
        hash_str = inherited[0].hash_string
        assert hash_str == "StopAttack_EX11-020", (
            f"Hash string should be 'StopAttack_EX11-020', got '{hash_str}'"
        )

    def test_inherited_only_on_opponent_turn(self, debug_runner):
        """The inherited effect condition should only pass on the opponent's turn."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # P1's digimon with Hanimon inherited
        perm = runner.place_on_field(1, ["EX11-020", "ST1-05"], turn_played=-1)
        # Need another digimon to delete
        runner.place_on_field(1, ["ST1-03"], turn_played=-1)

        hanimon_card = perm.card_sources[0]
        effects = hanimon_card.effect_list(EffectTiming.OnAllyAttack)
        inherited = [e for e in effects if e.is_inherited_effect and e.is_on_attack]
        assert len(inherited) >= 1

        effect = inherited[0]

        # Currently P1's turn -- condition should fail
        assert game.player1.is_my_turn, "Should be P1's turn"
        assert not effect.can_use_condition({}), (
            "Inherited condition should fail on own turn"
        )
