"""Behavioral tests for BT18-064 Mercurymon (Lv.4 Black Mineral/Mutant, DP 5000, Cost 5).

Card text:
  [On Play] [When Digivolving] Until the end of your opponent's turn, this
  Digimon can't be returned to the hand or deck by your opponent's effects.

  Inherited: [Opponent's Turn] This Digimon gets +2000 DP.

Key faithfulness points tested:
  - [On Play] grants CANNOT_RETURN_TO_HAND and CANNOT_RETURN_TO_DECK modifiers.
  - [When Digivolving] grants same modifiers.
  - Modifiers expire at 'end_of_opponent_turn'.
  - Inherited effect is +2000 DP on opponent's turn only.
  - Inherited effect uses _dp_modifier (continuous, not process-based).
  - Condition requires card to be on field.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming
from engine_py_legacy.engine.interfaces.modifiers import ModifierType

MERCURYMON = "BT18-064"
AGUMON = "ST1-03"
FILLER = [AGUMON] * 50


@pytest.mark.behavioral
class TestBT18064OnPlay:
    """Tests for BT18-064 On Play effect: return protection."""

    def test_has_on_play_effect(self, debug_runner):
        """Should have an On Play effect."""
        runner = debug_runner(deck1=[MERCURYMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=5)
        perm = runner.place_on_field(1, [MERCURYMON])

        top = perm.top_card
        effects = top.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        assert len(on_play) == 1, "Should have exactly 1 On Play effect"

    def test_has_when_digivolving_effect(self, debug_runner):
        """Should have a When Digivolving effect."""
        runner = debug_runner(deck1=[MERCURYMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=5)
        perm = runner.place_on_field(1, [MERCURYMON])

        top = perm.top_card
        effects = top.effect_list(None)
        when_digi = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        assert len(when_digi) == 1, "Should have exactly 1 When Digivolving effect"

    def test_on_play_grants_cannot_return_to_hand(self, debug_runner):
        """On Play should register CANNOT_RETURN_TO_HAND modifier."""
        runner = debug_runner(deck1=[MERCURYMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [MERCURYMON])

        # Trigger On Play effects
        runner.game.execute_effects(EffectTiming.OnEnterFieldAnyone, {
            "played_card": perm.top_card,
            "played_permanent": perm,
            "event_permanent": perm,
            "event_player": runner.game.player1,
            "is_effect_play": False,
        })
        runner.auto_resolve()

        hand_mods = runner.game.modifiers._modifiers.get(ModifierType.CANNOT_RETURN_TO_HAND, [])
        assert len(hand_mods) >= 1, "Should register CANNOT_RETURN_TO_HAND modifier"

    def test_on_play_grants_cannot_return_to_deck(self, debug_runner):
        """On Play should register CANNOT_RETURN_TO_DECK modifier."""
        runner = debug_runner(deck1=[MERCURYMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [MERCURYMON])

        runner.game.execute_effects(EffectTiming.OnEnterFieldAnyone, {
            "played_card": perm.top_card,
            "played_permanent": perm,
            "event_permanent": perm,
            "event_player": runner.game.player1,
            "is_effect_play": False,
        })
        runner.auto_resolve()

        deck_mods = runner.game.modifiers._modifiers.get(ModifierType.CANNOT_RETURN_TO_DECK, [])
        assert len(deck_mods) >= 1, "Should register CANNOT_RETURN_TO_DECK modifier"

    def test_modifier_expires_end_of_opponent_turn(self, debug_runner):
        """Modifiers should expire at 'end_of_opponent_turn'."""
        runner = debug_runner(deck1=[MERCURYMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [MERCURYMON])

        runner.game.execute_effects(EffectTiming.OnEnterFieldAnyone, {
            "played_card": perm.top_card,
            "played_permanent": perm,
            "event_permanent": perm,
            "event_player": runner.game.player1,
            "is_effect_play": False,
        })
        runner.auto_resolve()

        hand_mods = runner.game.modifiers._modifiers.get(ModifierType.CANNOT_RETURN_TO_HAND, [])
        deck_mods = runner.game.modifiers._modifiers.get(ModifierType.CANNOT_RETURN_TO_DECK, [])

        for mod in hand_mods:
            assert mod.expiry == 'end_of_opponent_turn', (
                f"CANNOT_RETURN_TO_HAND modifier should expire at end_of_opponent_turn, got {mod.expiry}"
            )
        for mod in deck_mods:
            assert mod.expiry == 'end_of_opponent_turn', (
                f"CANNOT_RETURN_TO_DECK modifier should expire at end_of_opponent_turn, got {mod.expiry}"
            )

    def test_when_digivolving_grants_same_protection(self, debug_runner):
        """When Digivolving should also grant return protection."""
        runner = debug_runner(deck1=[MERCURYMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=10)
        runner.set_phase("Main")

        # Stack: Agumon (Lv.3) -> Mercurymon (Lv.4)
        perm = runner.place_on_field(1, [AGUMON, MERCURYMON])

        # Fire WhenDigivolving timing
        runner.game.execute_effects(EffectTiming.WhenDigivolving, {
            "permanent": perm,
            "digivolved_permanent": perm,
            "player": runner.game.player1,
        })
        runner.auto_resolve()

        hand_mods = runner.game.modifiers._modifiers.get(ModifierType.CANNOT_RETURN_TO_HAND, [])
        deck_mods = runner.game.modifiers._modifiers.get(ModifierType.CANNOT_RETURN_TO_DECK, [])

        assert len(hand_mods) >= 1, "When Digivolving should register CANNOT_RETURN_TO_HAND"
        assert len(deck_mods) >= 1, "When Digivolving should register CANNOT_RETURN_TO_DECK"

    def test_condition_requires_field(self, debug_runner):
        """Condition should fail when Mercurymon is not on the field."""
        runner = debug_runner(deck1=[MERCURYMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=5)

        runner.inject_card(1, MERCURYMON, "hand")
        game = runner.game

        merc_hand = [
            c for c in game.player1.hand_cards
            if c.c_entity_base and c.c_entity_base.card_id == MERCURYMON
        ]
        assert len(merc_hand) >= 1

        card = merc_hand[0]
        effects = card.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        if on_play:
            assert not on_play[0].can_use_condition({}), (
                "Condition should fail when card is not on field"
            )


@pytest.mark.behavioral
class TestBT18064Inherited:
    """Tests for BT18-064 inherited: [Opponent's Turn] +2000 DP."""

    def test_has_inherited_dp_effect(self, debug_runner):
        """Should have an inherited effect with _dp_modifier = 2000."""
        runner = debug_runner(deck1=[MERCURYMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=5)
        perm = runner.place_on_field(1, [MERCURYMON])

        top = perm.top_card
        effects = top.effect_list(None)
        inherited_dp = [
            e for e in effects
            if e.is_inherited_effect and getattr(e, '_dp_modifier', 0) != 0
        ]
        assert len(inherited_dp) == 1, "Should have exactly 1 inherited DP effect"
        assert inherited_dp[0]._dp_modifier == 2000, "Should be +2000 DP"

    def test_inherited_dp_only_on_opponent_turn(self, debug_runner):
        """Inherited +2000 DP should only be active on opponent's turn."""
        runner = debug_runner(deck1=[MERCURYMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=5)

        # Place Mercurymon as inherited under another Digimon
        perm = runner.place_on_field(1, [MERCURYMON, "ST1-07"])  # Greymon on top
        game = runner.game

        bottom_card = perm.card_sources[0]  # Mercurymon
        effects = bottom_card.effect_list(None)
        inherited_dp = [
            e for e in effects
            if e.is_inherited_effect and getattr(e, '_dp_modifier', 0) != 0
        ][0]

        # On P1's turn (owner's turn) - should NOT be active
        assert game.player1.is_my_turn
        assert not inherited_dp.can_use_condition({}), (
            "Inherited DP effect should NOT be active on owner's turn"
        )

        # Switch to opponent's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        assert inherited_dp.can_use_condition({}), (
            "Inherited DP effect should be active on opponent's turn"
        )

    def test_inherited_dp_requires_field(self, debug_runner):
        """Inherited effect condition should fail when card is not on field."""
        runner = debug_runner(
            deck1=[MERCURYMON] * 4 + [AGUMON] * 46,
            deck2=FILLER,
            initial_memory=5,
            first_player=2,  # P2's turn = opponent's turn for P1
        )

        runner.inject_card(1, MERCURYMON, "hand")
        game = runner.game

        merc_hand = [
            c for c in game.player1.hand_cards
            if c.c_entity_base and c.c_entity_base.card_id == MERCURYMON
        ]
        assert len(merc_hand) >= 1

        card = merc_hand[0]
        effects = card.effect_list(None)
        inherited_dp = [
            e for e in effects
            if e.is_inherited_effect and getattr(e, '_dp_modifier', 0) != 0
        ]
        if inherited_dp:
            assert not inherited_dp[0].can_use_condition({}), (
                "Condition should fail when card is not on field"
            )
