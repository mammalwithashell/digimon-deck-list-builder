"""Behavioral tests for BT15-003 Nyaromon (Digi-Egg, Lv.2, Yellow, Lesser).

Actual card text (from cards.json):
Inherited Effect [When Attacking] [Once Per Turn] By trashing the top or bottom card
    of your security stack, gain 1 memory.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT15003Nyaromon:
    """Tests for BT15-003 Nyaromon."""

    # ------------------------------------------------------------------
    # Inherited effect structure
    # ------------------------------------------------------------------

    def test_inherited_effect_exists(self, debug_runner):
        """Should have an inherited When Attacking effect."""
        runner = debug_runner(initial_memory=5)
        # Place Nyaromon under a Lv.3 Digimon so inherited effect is active
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])

        source_card = perm.card_sources[0]  # BT15-003 is the bottom card
        all_effects = source_card.effect_list(None)
        inherited_wa = [
            e for e in all_effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_inherited_effect', False)
        ]
        assert len(inherited_wa) >= 1, "Should have inherited When Attacking effect"

    def test_inherited_effect_is_optional(self, debug_runner):
        """The inherited effect should be optional ('By trashing' = optional cost)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])

        source_card = perm.card_sources[0]
        all_effects = source_card.effect_list(None)
        inherited_wa = [
            e for e in all_effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_inherited_effect', False)
        ][0]
        assert inherited_wa.is_optional, "Effect should be optional"

    def test_inherited_effect_once_per_turn(self, debug_runner):
        """The inherited effect should be Once Per Turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])

        source_card = perm.card_sources[0]
        all_effects = source_card.effect_list(None)
        inherited_wa = [
            e for e in all_effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_inherited_effect', False)
        ][0]
        assert getattr(inherited_wa, 'max_count_per_turn', None) == 1, \
            "Effect should be Once Per Turn (max_count_per_turn=1)"

    # ------------------------------------------------------------------
    # Effect requires security cards
    # ------------------------------------------------------------------

    def test_condition_requires_security_cards(self, debug_runner):
        """Effect condition should fail when player has no security cards."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])

        game = runner.game
        # Clear all security cards
        game.player1.security_cards.clear()

        source_card = perm.card_sources[0]
        all_effects = source_card.effect_list(None)
        inherited_wa = [
            e for e in all_effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_inherited_effect', False)
        ][0]

        result = inherited_wa.can_use_condition({
            'player': game.player1,
            'permanent': perm,
        })
        assert not result, "Should not be usable with no security cards"

    def test_condition_passes_with_security(self, debug_runner):
        """Effect condition should pass when player has security cards."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])

        game = runner.game
        assert len(game.player1.security_cards) > 0, "Should have security cards after setup"

        source_card = perm.card_sources[0]
        all_effects = source_card.effect_list(None)
        inherited_wa = [
            e for e in all_effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_inherited_effect', False)
        ][0]

        result = inherited_wa.can_use_condition({
            'player': game.player1,
            'permanent': perm,
        })
        assert result, "Should be usable when player has security cards"

    # ------------------------------------------------------------------
    # Process: trash security (top or bottom choice) + gain 1 memory
    # ------------------------------------------------------------------

    def test_process_offers_top_or_bottom_choice(self, debug_runner):
        """Effect should offer a choice between trashing top or bottom security."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])

        game = runner.game
        source_card = perm.card_sources[0]
        all_effects = source_card.effect_list(None)
        inherited_wa = [
            e for e in all_effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_inherited_effect', False)
        ][0]

        security_before = len(game.player1.security_cards)
        memory_before = game.memory

        # Execute the effect
        inherited_wa.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Auto-resolve the branch choice (will pick first = top)
        runner.auto_resolve()

        # Should have trashed 1 security card
        assert len(game.player1.security_cards) == security_before - 1, \
            "Should trash 1 security card"
        # Should have gained 1 memory
        assert game.memory == memory_before + 1, \
            "Should gain 1 memory after trashing security"

    def test_trashing_top_security(self, debug_runner):
        """Choosing 'top' should trash the first security card."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])

        game = runner.game
        # Record the top security card
        if len(game.player1.security_cards) > 1:
            top_security_card = game.player1.security_cards[0]

            source_card = perm.card_sources[0]
            all_effects = source_card.effect_list(None)
            inherited_wa = [
                e for e in all_effects
                if e.timing == EffectTiming.OnUseAttack
                and getattr(e, 'is_inherited_effect', False)
            ][0]

            inherited_wa.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })

            # Auto-resolve picks first option (choice 0 = top)
            runner.auto_resolve()

            # The trashed card should be in trash
            assert top_security_card in game.player1.trash_cards, \
                "Top security card should be in trash"

    def test_process_gains_memory(self, debug_runner):
        """Effect should gain exactly 1 memory."""
        runner = debug_runner(initial_memory=3)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])

        game = runner.game
        memory_before = game.memory

        source_card = perm.card_sources[0]
        all_effects = source_card.effect_list(None)
        inherited_wa = [
            e for e in all_effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_inherited_effect', False)
        ][0]

        inherited_wa.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        assert game.memory == memory_before + 1, \
            f"Expected memory {memory_before + 1}, got {game.memory}"
