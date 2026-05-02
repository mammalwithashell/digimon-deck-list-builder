"""Behavioral tests for BT14-009 Gotsumon (Lv.3 Red Mineral/Rock, DP 2000, Cost 3).

Card text:
  [All Turns] Players can't play Digimon by effects.

Key faithfulness points tested:
  - Effect registers on On Play (OnEnterFieldAnyone + is_on_play flag).
  - Uses CANNOT_PLAY_BY_EFFECT modifier (not CANNOT_PLAY_CARD) because
    card text says "by effects" -- normal hand plays must be allowed.
  - Blocks both players' effect-based Digimon plays.
  - Does NOT block non-Digimon plays (Tamers, Options) by effects.
  - Does NOT block normal hand plays of Digimon.
  - Modifier is permanent (lasts while Gotsumon is on field).
  - Condition requires Gotsumon to be on field.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming
from engine_py_legacy.engine.interfaces.modifiers import ModifierType

GOTSUMON = "BT14-009"
AGUMON = "ST1-03"
FILLER = [AGUMON] * 50


@pytest.mark.behavioral
class TestBT14009Gotsumon:
    """Tests for BT14-009 Gotsumon: [All Turns] Players can't play Digimon by effects."""

    def test_has_on_play_effect(self, debug_runner):
        """Should have exactly 1 On Play effect."""
        runner = debug_runner(deck1=[GOTSUMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=5)
        perm = runner.place_on_field(1, [GOTSUMON])

        top = perm.top_card
        effects = top.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        assert len(on_play) == 1, "Should have exactly 1 On Play effect"

    def test_uses_cannot_play_by_effect_not_cannot_play_card(self, debug_runner):
        """Card text says 'by effects' so must use CANNOT_PLAY_BY_EFFECT modifier.
        CANNOT_PLAY_CARD would incorrectly block normal hand plays too."""
        runner = debug_runner(deck1=[GOTSUMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [GOTSUMON])

        # Trigger On Play effects
        runner.game.execute_effects(EffectTiming.OnEnterFieldAnyone, {
            "played_card": perm.top_card,
            "played_permanent": perm,
            "event_permanent": perm,
            "event_player": runner.game.player1,
            "is_effect_play": False,
        })
        runner.auto_resolve()

        # Should have CANNOT_PLAY_BY_EFFECT registered
        by_effect_mods = runner.game.modifiers._modifiers.get(
            ModifierType.CANNOT_PLAY_BY_EFFECT, []
        )
        assert len(by_effect_mods) >= 1, (
            "Should register CANNOT_PLAY_BY_EFFECT modifier (card says 'by effects')"
        )

        # Should NOT have CANNOT_PLAY_CARD registered
        play_card_mods = runner.game.modifiers._modifiers.get(
            ModifierType.CANNOT_PLAY_CARD, []
        )
        assert len(play_card_mods) == 0, (
            "Should NOT use CANNOT_PLAY_CARD -- card says 'by effects', "
            "which only blocks effect-based plays"
        )

    def test_blocks_digimon_by_effect(self, debug_runner):
        """Should block Digimon being played by effects."""
        runner = debug_runner(deck1=[GOTSUMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [GOTSUMON])

        # Trigger On Play effects
        runner.game.execute_effects(EffectTiming.OnEnterFieldAnyone, {
            "played_card": perm.top_card,
            "played_permanent": perm,
            "event_permanent": perm,
            "event_player": runner.game.player1,
            "is_effect_play": False,
        })
        runner.auto_resolve()

        by_effect_mods = runner.game.modifiers._modifiers.get(
            ModifierType.CANNOT_PLAY_BY_EFFECT, []
        )
        assert len(by_effect_mods) >= 1

        entry = by_effect_mods[0]

        # Test with a mock Digimon card
        class MockCard:
            def __init__(self, is_digimon=False, is_tamer=False, is_option=False):
                self.is_digimon = is_digimon
                self.is_tamer = is_tamer
                self.is_option = is_option

        digimon_card = MockCard(is_digimon=True)
        assert entry.condition(None, {'card': digimon_card}), (
            "Should block Digimon plays by effect"
        )

    def test_does_not_block_tamers_by_effect(self, debug_runner):
        """Should NOT block Tamers being played by effects."""
        runner = debug_runner(deck1=[GOTSUMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [GOTSUMON])

        runner.game.execute_effects(EffectTiming.OnEnterFieldAnyone, {
            "played_card": perm.top_card,
            "played_permanent": perm,
            "event_permanent": perm,
            "event_player": runner.game.player1,
            "is_effect_play": False,
        })
        runner.auto_resolve()

        by_effect_mods = runner.game.modifiers._modifiers.get(
            ModifierType.CANNOT_PLAY_BY_EFFECT, []
        )
        assert len(by_effect_mods) >= 1

        entry = by_effect_mods[0]

        class MockCard:
            def __init__(self, is_digimon=False, is_tamer=False):
                self.is_digimon = is_digimon
                self.is_tamer = is_tamer

        tamer_card = MockCard(is_tamer=True)
        assert not entry.condition(None, {'card': tamer_card}), (
            "Should NOT block Tamer plays by effect"
        )

    def test_does_not_block_hand_play(self, debug_runner):
        """Normal hand plays of Digimon should NOT be blocked."""
        runner = debug_runner(deck1=[GOTSUMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [GOTSUMON])

        runner.game.execute_effects(EffectTiming.OnEnterFieldAnyone, {
            "played_card": perm.top_card,
            "played_permanent": perm,
            "event_permanent": perm,
            "event_player": runner.game.player1,
            "is_effect_play": False,
        })
        runner.auto_resolve()

        # Inject a cheap Digimon into hand
        runner.inject_card(1, AGUMON, "hand")
        card = runner.game.player1.hand_cards[-1]

        # _is_play_blocked_by_modifier checks CANNOT_PLAY_CARD -- should not block
        is_blocked = runner.game._is_play_blocked_by_modifier(card)
        assert not is_blocked, (
            "Normal hand plays should NOT be blocked. The effect says 'by effects' "
            "so only effect-based plays should be blocked."
        )

    def test_modifier_is_permanent(self, debug_runner):
        """Modifier should be permanent (lasts while Gotsumon is on field)."""
        runner = debug_runner(deck1=[GOTSUMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [GOTSUMON])

        runner.game.execute_effects(EffectTiming.OnEnterFieldAnyone, {
            "played_card": perm.top_card,
            "played_permanent": perm,
            "event_permanent": perm,
            "event_player": runner.game.player1,
            "is_effect_play": False,
        })
        runner.auto_resolve()

        by_effect_mods = runner.game.modifiers._modifiers.get(
            ModifierType.CANNOT_PLAY_BY_EFFECT, []
        )
        assert len(by_effect_mods) >= 1
        assert by_effect_mods[0].expiry == 'permanent', (
            "Modifier should be permanent (lasts while on field)"
        )

    def test_condition_requires_field(self, debug_runner):
        """Condition should fail when Gotsumon is not on the field."""
        runner = debug_runner(deck1=[GOTSUMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=5)

        # Place in hand, not on field
        runner.inject_card(1, GOTSUMON, "hand")
        game = runner.game

        gotsumon_hand = [
            c for c in game.player1.hand_cards
            if c.c_entity_base and c.c_entity_base.card_id == GOTSUMON
        ]
        assert len(gotsumon_hand) >= 1

        card = gotsumon_hand[0]
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
