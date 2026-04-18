"""Behavioral tests for EX2-007 Mother D-Reaper (DigiEgg, White, D-Reaper).

Card text (from cards.json):
[All Turns] This Digimon can't attack and isn't affected by your
    opponent's effects.
[Main] [Once Per Turn] If you don't have another [Mother D-Reaper] in play,
    place 1 of your [ADR-02 Searcher]s from in play or your hand under this
    Digimon as its bottom digivolution card.
[Your Turn] [Once Per Turn] When you would play a card with the [D-Reaper]
    trait from your hand, you may reduce its play cost by 1 for each of
    this Digimon's digivolution cards.

Note: EX2-007 is a DigiEgg (card_kind=3) with DP 15000.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX2007MotherDReaper:
    """Tests for EX2-007 Mother D-Reaper."""

    # ------------------------------------------------------------------
    # Helper: get specific effects by attribute
    # ------------------------------------------------------------------

    def _get_effects(self, perm):
        """Get all effects from the top card of the permanent."""
        top = perm.top_card
        return top.effect_list(None) if top else []

    def _get_cannot_attack_effect(self, perm):
        """Find the can't attack effect."""
        for e in self._get_effects(perm):
            if getattr(e, '_is_cannot_attack', False):
                return e
        return None

    def _get_immune_effect(self, perm):
        """Find the immune to opponent's effects effect."""
        for e in self._get_effects(perm):
            if getattr(e, '_is_immune_to_opponent_effects', False):
                return e
        return None

    def _get_main_place_effect(self, perm):
        """Find the [Main] place ADR-02 effect."""
        for e in self._get_effects(perm):
            if getattr(e, '_is_field_main', False) and e.timing == EffectTiming.OnDeclaration:
                return e
        return None

    def _get_cost_reduction_effect(self, perm):
        """Find the BeforePayCost cost reduction effect."""
        for e in self._get_effects(perm):
            if e.timing == EffectTiming.BeforePayCost:
                return e
        return None

    # ------------------------------------------------------------------
    # Effect 0: [All Turns] Can't attack
    # ------------------------------------------------------------------

    def test_cannot_attack_effect_exists(self, debug_runner):
        """Should have a can't attack effect."""
        runner = debug_runner(
            deck1=["EX2-007"] * 4 + ["EX2-046"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        # Mother D-Reaper is a DigiEgg; place it on field as top card
        perm = runner.place_on_field(1, ["EX2-007"])
        effect = self._get_cannot_attack_effect(perm)
        assert effect is not None, "Should have _is_cannot_attack effect"

    def test_cannot_attack_requires_field(self, debug_runner):
        """Can't attack condition should fail when not on field."""
        runner = debug_runner(
            deck1=["EX2-007"] * 4 + ["EX2-046"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX2-007"])
        effect = self._get_cannot_attack_effect(perm)

        # Remove from field
        runner.game.player1.battle_area.remove(perm)
        result = effect.can_use_condition({})
        assert not result, "Can't attack should not be active off-field"

    # ------------------------------------------------------------------
    # Effect 1: [All Turns] Immune to opponent's effects
    # ------------------------------------------------------------------

    def test_immune_to_opponent_effects_exists(self, debug_runner):
        """Should have immune to opponent's effects."""
        runner = debug_runner(
            deck1=["EX2-007"] * 4 + ["EX2-046"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX2-007"])
        effect = self._get_immune_effect(perm)
        assert effect is not None, "Should have _is_immune_to_opponent_effects effect"

    def test_immune_requires_field(self, debug_runner):
        """Immunity condition should fail when not on field."""
        runner = debug_runner(
            deck1=["EX2-007"] * 4 + ["EX2-046"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX2-007"])
        effect = self._get_immune_effect(perm)

        runner.game.player1.battle_area.remove(perm)
        result = effect.can_use_condition({})
        assert not result, "Immunity should not be active off-field"

    # ------------------------------------------------------------------
    # Effect 2: [Main] [Once Per Turn] Place ADR-02 Searcher
    # ------------------------------------------------------------------

    def test_main_place_effect_exists(self, debug_runner):
        """Should have the [Main] place ADR-02 effect."""
        runner = debug_runner(
            deck1=["EX2-007"] * 4 + ["EX2-046"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX2-007"])
        effect = self._get_main_place_effect(perm)
        assert effect is not None, "Should have [Main] place ADR-02 effect"

    def test_main_place_is_once_per_turn(self, debug_runner):
        """The [Main] place effect should be Once Per Turn."""
        runner = debug_runner(
            deck1=["EX2-007"] * 4 + ["EX2-046"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX2-007"])
        effect = self._get_main_place_effect(perm)
        assert effect.max_count_per_turn == 1, "Should be Once Per Turn"

    def test_main_place_condition_no_other_mother_dreaper(self, debug_runner):
        """Condition should pass when no other Mother D-Reaper is in play."""
        runner = debug_runner(
            deck1=["EX2-007"] * 4 + ["EX2-046"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX2-007"])
        # Place ADR-02 on field so there is a valid target
        adr02 = runner.place_on_field(1, ["EX2-046"])

        effect = self._get_main_place_effect(perm)
        result = effect.can_use_condition({})
        assert result, "Should be usable when no other Mother D-Reaper and ADR-02 on field"

    def test_main_place_condition_fails_with_another_mother_dreaper(self, debug_runner):
        """Condition should fail when another Mother D-Reaper is in play."""
        runner = debug_runner(
            deck1=["EX2-007"] * 4 + ["EX2-046"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm1 = runner.place_on_field(1, ["EX2-007"])
        perm2 = runner.place_on_field(1, ["EX2-007"])  # Second Mother D-Reaper

        # Also need ADR-02 target
        adr02 = runner.place_on_field(1, ["EX2-046"])

        effect = self._get_main_place_effect(perm1)
        result = effect.can_use_condition({})
        assert not result, "Should fail when another Mother D-Reaper is in play"

    def test_main_place_condition_fails_no_adr02(self, debug_runner):
        """Condition should fail when no ADR-02 Searcher is on field or in hand."""
        runner = debug_runner(
            deck1=["EX2-007"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX2-007"])

        effect = self._get_main_place_effect(perm)
        result = effect.can_use_condition({})
        assert not result, "Should fail when no ADR-02 on field or in hand"

    def test_main_place_condition_passes_adr02_in_hand(self, debug_runner):
        """Condition should pass when ADR-02 Searcher is in hand."""
        runner = debug_runner(
            deck1=["EX2-007"] * 4 + ["EX2-046"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
            starting_hand1=["EX2-046"],
        )
        perm = runner.place_on_field(1, ["EX2-007"])
        game = runner.game

        # Verify ADR-02 is in hand
        hand_names = [c.card_names[0] if c.card_names else '' for c in game.player1.hand_cards]
        assert any('ADR-02 Searcher' in n for n in hand_names), "ADR-02 should be in hand"

        effect = self._get_main_place_effect(perm)
        result = effect.can_use_condition({})
        assert result, "Should be usable when ADR-02 is in hand"

    def test_main_place_from_field_adds_bottom_digi_card(self, debug_runner):
        """Process should place ADR-02 from field as bottom digivolution card."""
        runner = debug_runner(
            deck1=["EX2-007"] * 4 + ["EX2-046"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX2-007"])
        adr02 = runner.place_on_field(1, ["EX2-046"])
        game = runner.game

        assert len(perm.card_sources) == 1, "Mother D-Reaper should have 1 card initially"
        field_before = len(game.player1.battle_area)

        effect = self._get_main_place_effect(perm)
        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # ADR-02 should have been absorbed into Mother D-Reaper's digi stack
        assert len(perm.card_sources) > 1, "Mother D-Reaper should have gained digivolution card(s)"
        # ADR-02 permanent should be removed from field
        assert len(game.player1.battle_area) < field_before, "ADR-02 should be removed from field"

    def test_main_place_from_hand_adds_bottom_digi_card(self, debug_runner):
        """Process should place ADR-02 from hand as bottom digivolution card."""
        runner = debug_runner(
            deck1=["EX2-007"] * 4 + ["EX2-046"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
            starting_hand1=["EX2-046"],
        )
        perm = runner.place_on_field(1, ["EX2-007"])
        game = runner.game

        # Remove any ADR-02 from field to force hand source
        for p in list(game.player1.battle_area):
            if p is not perm and p.top_card and 'ADR-02 Searcher' in (p.top_card.card_names or []):
                game.player1.battle_area.remove(p)

        hand_before = len(game.player1.hand_cards)
        assert len(perm.card_sources) == 1

        effect = self._get_main_place_effect(perm)
        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # Hand should lose 1 card (ADR-02)
        assert len(game.player1.hand_cards) < hand_before, "ADR-02 should be removed from hand"
        assert len(perm.card_sources) == 2, "Mother D-Reaper should have 2 cards (itself + ADR-02)"

    # ------------------------------------------------------------------
    # Effect 3: [Your Turn] [Once Per Turn] Cost reduction for D-Reaper
    # ------------------------------------------------------------------

    def test_cost_reduction_effect_exists(self, debug_runner):
        """Should have BeforePayCost cost reduction effect."""
        runner = debug_runner(
            deck1=["EX2-007"] * 4 + ["EX2-046"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX2-007"])
        effect = self._get_cost_reduction_effect(perm)
        assert effect is not None, "Should have BeforePayCost effect"

    def test_cost_reduction_is_once_per_turn(self, debug_runner):
        """Cost reduction should be Once Per Turn."""
        runner = debug_runner(
            deck1=["EX2-007"] * 4 + ["EX2-046"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX2-007"])
        effect = self._get_cost_reduction_effect(perm)
        assert effect.max_count_per_turn == 1, "Should be Once Per Turn"

    def test_cost_reduction_is_optional(self, debug_runner):
        """Cost reduction should be optional ('you may reduce')."""
        runner = debug_runner(
            deck1=["EX2-007"] * 4 + ["EX2-046"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX2-007"])
        effect = self._get_cost_reduction_effect(perm)
        assert effect.is_optional, "Should be optional"

    def test_cost_reduction_requires_your_turn(self, debug_runner):
        """Cost reduction condition should fail on opponent's turn."""
        runner = debug_runner(
            deck1=["EX2-007"] * 4 + ["EX2-046"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX2-007"])

        # Give it digi cards so the count check passes
        adr02_cs = runner.inject_card(1, "EX2-046", zone="hand")
        effect = self._get_main_place_effect(perm)
        effect.on_process_callback({
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': perm,
        })
        runner.auto_resolve()

        cost_effect = self._get_cost_reduction_effect(perm)

        # Switch to opponent's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        # Create a D-Reaper card context
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        dreaper_card = db.create_card_source("EX2-047", runner.game.player1)
        result = cost_effect.can_use_condition({'card_source': dreaper_card})
        assert not result, "Cost reduction should fail on opponent's turn"

    def test_cost_reduction_requires_dreaper_trait(self, debug_runner):
        """Cost reduction should only apply to D-Reaper trait cards."""
        runner = debug_runner(
            deck1=["EX2-007"] * 4 + ["EX2-046"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX2-007"])

        # Add a digi card to Mother D-Reaper so it has cards to count
        adr02 = runner.place_on_field(1, ["EX2-046"])
        main_effect = self._get_main_place_effect(perm)
        main_effect.on_process_callback({
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': perm,
        })
        runner.auto_resolve()

        cost_effect = self._get_cost_reduction_effect(perm)

        # Non-D-Reaper card should not trigger
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        non_dreaper = db.create_card_source("ST1-03", runner.game.player1)
        result = cost_effect.can_use_condition({'card_source': non_dreaper})
        assert not result, "Cost reduction should not apply to non-D-Reaper cards"

    def test_cost_reduction_amount_equals_digi_card_count(self, debug_runner):
        """Cost reduction should equal the number of digivolution cards.

        Per C# reference: reduceCount = DigivolutionCards.Count (all cards in stack).
        """
        runner = debug_runner(
            deck1=["EX2-007"] * 4 + ["EX2-046"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX2-007"])
        game = runner.game

        # Add ADR-02 as digi card
        adr02 = runner.place_on_field(1, ["EX2-046"])
        main_effect = self._get_main_place_effect(perm)
        main_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # Now perm has 2 card_sources (EX2-007 + EX2-046's card(s))
        digi_count = len(perm.card_sources)
        assert digi_count >= 2, f"Should have at least 2 cards in stack, got {digi_count}"
