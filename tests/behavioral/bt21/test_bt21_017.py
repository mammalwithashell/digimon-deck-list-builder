"""Behavioral tests for BT21-017 Dimetromon (Lv.4 Red Digimon).

Card text:
[When Digivolving] If you have 1 or fewer Tamers, you may play 1
[Owen Dreadnought] from your hand without paying the cost.

Inherited Effect:
[Your Turn] [Once Per Turn] When your opponent's security stack is removed from,
gain 1 memory.

C# reference (BT21_017.cs):
- OnLoseSecurity inherited uses CanTriggerWhenLoseSecurity(hashtable, PlayerCondition)
  where PlayerCondition => player == card.Owner.Enemy
- Must only trigger when the OPPONENT loses security, not when the card owner does
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT21017Dimetromon:
    """Tests for BT21-017 Dimetromon."""

    # ── Inherited: OnLoseSecurity gain 1 memory ──────────────────────

    def test_inherited_gain_memory_on_opponent_security_loss(self, debug_runner):
        """Inherited effect should gain 1 memory when the opponent's security
        is removed (event_player is the opponent, i.e., NOT the card owner)."""
        runner = debug_runner(initial_memory=3)
        game = runner.game

        # Place Dimetromon as inherited source under a Lv.5 Digimon
        perm = runner.place_on_field(1, ["BT21-017", "ST1-07"])

        # Get the inherited OnLoseSecurity effect from Dimetromon (bottom card)
        dimetromon_card = perm.card_sources[0]  # bottom of stack = BT21-017
        effects = dimetromon_card.effect_list(None)
        lose_sec_effects = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity]
        assert len(lose_sec_effects) == 1, "Dimetromon should have exactly 1 OnLoseSecurity effect"

        effect = lose_sec_effects[0]
        assert effect.is_inherited_effect, "OnLoseSecurity effect must be inherited"

        memory_before = game.memory

        # Simulate opponent (player2) losing security -- event_player is the opponent
        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player2,  # opponent lost security
        }
        assert effect.can_use_condition(context), (
            "Condition should pass when opponent loses security during your turn"
        )

        effect.on_process_callback(context)
        assert game.memory == memory_before + 1, (
            "Should gain 1 memory when opponent's security is removed"
        )

    def test_inherited_no_trigger_on_own_security_loss(self, debug_runner):
        """Inherited effect must NOT trigger when the card owner's own security
        is removed. The C# checks player == card.Owner.Enemy."""
        runner = debug_runner(initial_memory=3)
        game = runner.game

        perm = runner.place_on_field(1, ["BT21-017", "ST1-07"])

        dimetromon_card = perm.card_sources[0]
        effects = dimetromon_card.effect_list(None)
        lose_sec_effect = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        # event_player is the card owner (player1) -- own security removed
        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player1,  # own security lost
        }
        assert not lose_sec_effect.can_use_condition(context), (
            "Condition must FAIL when the card owner's own security is removed. "
            "Card text says 'opponent's security stack', not 'your security stack'."
        )

    def test_inherited_no_trigger_on_opponent_turn(self, debug_runner):
        """Inherited effect requires [Your Turn] -- must not trigger on opponent's turn."""
        runner = debug_runner(initial_memory=3)
        game = runner.game

        perm = runner.place_on_field(1, ["BT21-017", "ST1-07"])

        dimetromon_card = perm.card_sources[0]
        effects = dimetromon_card.effect_list(None)
        lose_sec_effect = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        # Switch turns so it's player2's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player2,  # opponent lost security, but it's their turn
        }
        assert not lose_sec_effect.can_use_condition(context), (
            "Condition must FAIL on opponent's turn (requires [Your Turn])"
        )

    def test_inherited_once_per_turn(self, debug_runner):
        """Inherited effect is [Once Per Turn] -- should have max_count_per_turn set."""
        runner = debug_runner(initial_memory=3)

        perm = runner.place_on_field(1, ["BT21-017", "ST1-07"])

        dimetromon_card = perm.card_sources[0]
        effects = dimetromon_card.effect_list(None)
        lose_sec_effect = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        assert lose_sec_effect.max_count_per_turn == 1, (
            "Effect should be limited to once per turn"
        )

    def test_inherited_not_on_field_no_trigger(self, debug_runner):
        """If Dimetromon is not on the field (e.g., in hand), effect should not trigger."""
        runner = debug_runner(initial_memory=3)
        game = runner.game

        # Inject Dimetromon into hand (not on field)
        runner.inject_card(1, "BT21-017", "hand")

        # Get card from hand
        dimetromon_card = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT21-017":
                dimetromon_card = c
                break
        assert dimetromon_card is not None

        effects = dimetromon_card.effect_list(None)
        lose_sec_effects = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity]
        if lose_sec_effects:
            context = {
                'game': game,
                'player': game.player1,
                'event_player': game.player2,
            }
            assert not lose_sec_effects[0].can_use_condition(context), (
                "Effect must not trigger when card is not on the field"
            )

    # ── When Digivolving: play Owen Dreadnought ─────────────────────

    def test_when_digivolving_play_owen_from_hand(self, debug_runner):
        """[When Digivolving] should allow playing Owen Dreadnought from hand
        if 1 or fewer tamers on field."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place Dimetromon on field (as top card -- simulating it just digivolved)
        perm = runner.place_on_field(1, ["ST1-03", "BT21-017"])

        # Inject Owen Dreadnought (BT21-081) into hand
        runner.inject_card(1, "BT21-081", "hand")

        # Get the OnEnterFieldAnyone effect
        dimetromon_card = perm.top_card
        effects = dimetromon_card.effect_list(None)
        digivolve_effects = [e for e in effects if e.timing == EffectTiming.OnEnterFieldAnyone]
        assert len(digivolve_effects) >= 1, "Dimetromon should have a When Digivolving effect"

        effect = digivolve_effects[0]
        assert effect.is_when_digivolving, "Effect must be flagged as when_digivolving"
        assert effect.is_optional, "Effect must be optional"

    def test_when_digivolving_blocked_with_2_tamers(self, debug_runner):
        """[When Digivolving] condition should fail if owner has 2+ tamers on field."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place Dimetromon on field
        perm = runner.place_on_field(1, ["ST1-03", "BT21-017"])

        # Place 2 tamers on field
        runner.place_on_field(1, ["BT21-081"])  # Owen Dreadnought tamer
        runner.place_on_field(1, ["BT21-081"])  # Second Owen Dreadnought

        # Inject Owen into hand for the play target
        runner.inject_card(1, "BT21-081", "hand")

        dimetromon_card = perm.top_card
        effects = dimetromon_card.effect_list(None)
        digivolve_effect = [e for e in effects if e.timing == EffectTiming.OnEnterFieldAnyone][0]

        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
        }
        assert not digivolve_effect.can_use_condition(context), (
            "Condition should FAIL when owner has 2+ tamers on field"
        )

    def test_when_digivolving_allowed_with_1_tamer(self, debug_runner):
        """[When Digivolving] condition should pass if owner has exactly 1 tamer."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        perm = runner.place_on_field(1, ["ST1-03", "BT21-017"])

        # Place exactly 1 tamer
        runner.place_on_field(1, ["BT21-081"])

        # Inject Owen into hand
        runner.inject_card(1, "BT21-081", "hand")

        dimetromon_card = perm.top_card
        effects = dimetromon_card.effect_list(None)
        digivolve_effect = [e for e in effects if e.timing == EffectTiming.OnEnterFieldAnyone][0]

        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
        }
        assert digivolve_effect.can_use_condition(context), (
            "Condition should PASS when owner has exactly 1 tamer"
        )

    def test_when_digivolving_allowed_with_0_tamers(self, debug_runner):
        """[When Digivolving] condition should pass if owner has 0 tamers."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        perm = runner.place_on_field(1, ["ST1-03", "BT21-017"])

        # No tamers on field -- inject Owen into hand
        runner.inject_card(1, "BT21-081", "hand")

        dimetromon_card = perm.top_card
        effects = dimetromon_card.effect_list(None)
        digivolve_effect = [e for e in effects if e.timing == EffectTiming.OnEnterFieldAnyone][0]

        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
        }
        assert digivolve_effect.can_use_condition(context), (
            "Condition should PASS when owner has 0 tamers"
        )
