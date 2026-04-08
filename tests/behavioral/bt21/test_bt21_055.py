"""Behavioral tests for BT21-055 Sunarizamon (Lv.3, Black, Reptile/LIBERATOR).

Card text:
  Effect: [Your Turn] When this Digimon would digivolve into a card with the
  [Mineral] or [Rock] trait, reduce the digivolution cost by 1.

  Inherited: When effects trash this card from a [Mineral] or [Rock] trait
  Digimon's digivolution cards, delete 1 of your opponent's Digimon with a
  play cost of 4 or less.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming

# Clean filler decks
FILLER_DECK = ["ST1-03"] * 50


@pytest.mark.behavioral
class TestBT21055DigivolveCostReduction:
    """Tests for the main effect: digivolution cost reduction for Mineral/Rock."""

    def test_timing_is_when_would_digivolve(self, debug_runner):
        """The cost reduction effect must use WhenWouldDigivolve timing,
        not BeforePayCost (which only applies to play costs)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        perm = runner.place_on_field(1, ["BT21-055"])

        card = perm.top_card
        effects = card.effect_list(None)
        wwd_effects = [e for e in effects if e.timing == EffectTiming.WhenWouldDigivolve]
        assert len(wwd_effects) == 1, (
            f"Expected 1 WhenWouldDigivolve effect, got {len(wwd_effects)}. "
            f"All timings: {[e.timing for e in effects]}"
        )
        assert wwd_effects[0].cost_reduction == 1

    def test_cost_reduced_when_digivolving_into_mineral(self, debug_runner):
        """Digivolving into a Mineral trait Lv.4 should cost 1 less."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )

        # Place BT21-055 Sunarizamon (Lv.3, Black) on field
        perm = runner.place_on_field(1, ["BT21-055"])

        # BT4-066 Golemon: Lv.4 Black Mineral, evo cost 2 from Black Lv.3
        runner.inject_card(1, "BT4-066", "hand")

        runner.set_phase("Main")
        snap_before = runner.snapshot()

        # Digivolve Golemon onto Sunarizamon
        action = runner.find_action("Digivolve Golemon onto Sunarizamon")
        assert action is not None, (
            f"Should be able to digivolve Golemon onto Sunarizamon. "
            f"Actions: {runner.actions()}"
        )
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Base evo cost = 2, reduction = 1, final = 1
        memory_spent = snap_before.memory - snap_after.memory
        assert memory_spent == 1, (
            f"Should spend 1 memory (2 base - 1 Mineral reduction), spent {memory_spent}"
        )

    def test_no_reduction_for_non_mineral_rock(self, debug_runner):
        """Digivolving into a non-Mineral/Rock card should NOT get reduction."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )

        # Place BT21-055 Sunarizamon on field
        perm = runner.place_on_field(1, ["BT21-055"])

        # ST13-10 Gladimon: Lv.4 Black Warrior, evo cost 1 from Black Lv.3
        runner.inject_card(1, "ST13-10", "hand")

        runner.set_phase("Main")
        snap_before = runner.snapshot()

        action = runner.find_action("Digivolve Gladimon onto Sunarizamon")
        assert action is not None, (
            f"Should be able to digivolve Gladimon onto Sunarizamon. "
            f"Actions: {runner.actions()}"
        )
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Base evo cost = 1, no reduction (Warrior trait, not Mineral/Rock)
        memory_spent = snap_before.memory - snap_after.memory
        assert memory_spent == 1, (
            f"Should spend full 1 memory (no Mineral/Rock reduction), spent {memory_spent}"
        )

    def test_condition_fails_on_opponent_turn(self, debug_runner):
        """[Your Turn] restriction: cost reduction must not apply on opponent's turn."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )

        perm = runner.place_on_field(1, ["BT21-055"])
        game = runner.game

        # Switch to opponent's turn
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        card = perm.top_card
        effects = card.effect_list(None)
        wwd = [e for e in effects if e.timing == EffectTiming.WhenWouldDigivolve][0]

        # Build context similar to what Player.digivolve uses
        from digimon_gym.engine.core.card_source import CardSource
        from digimon_gym.engine.core.entity_base import CEntity_Base
        e = CEntity_Base()
        e.type_eng = ["Mineral"]
        mineral_card = CardSource()
        mineral_card.c_entity_base = e

        context = {
            "player": game.player1,
            "permanent": perm,
            "card_source": mineral_card,
        }
        assert not wwd.can_use_condition(context), (
            "Cost reduction must NOT apply on opponent's turn"
        )

    def test_effect_is_not_inherited(self, debug_runner):
        """The main digivolution cost reduction effect must not be inherited."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        perm = runner.place_on_field(1, ["BT21-055"])
        card = perm.top_card
        effects = card.effect_list(None)
        wwd = [e for e in effects if e.timing == EffectTiming.WhenWouldDigivolve][0]
        assert not wwd.is_inherited_effect, (
            "Main effect should NOT be inherited"
        )


@pytest.mark.behavioral
class TestBT21055InheritedDelete:
    """Tests for inherited effect: delete opponent's Digimon with play cost 4 or less
    when this card is trashed from a Mineral/Rock Digimon's digivolution cards."""

    def test_inherited_timing(self, debug_runner):
        """Inherited effect must use OnDigivolutionCardDiscarded timing."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        perm = runner.place_on_field(1, ["BT21-055"])
        card = perm.top_card
        effects = card.effect_list(None)
        odcd = [e for e in effects if e.timing == EffectTiming.OnDigivolutionCardDiscarded]
        assert len(odcd) == 1, (
            f"Expected 1 OnDigivolutionCardDiscarded effect, got {len(odcd)}"
        )
        assert odcd[0].is_inherited_effect, "Effect must be inherited"

    def test_inherited_triggers_from_mineral_permanent(self, debug_runner):
        """When BT21-055 is trashed from under a Mineral trait Digimon,
        the inherited should trigger and delete an opponent's low-cost Digimon."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )

        # Stack: BT21-055 (inherited source) under BT4-066 Golemon (Mineral, Lv.4)
        perm = runner.place_on_field(1, ["BT21-055", "BT4-066"])

        # Place opponent's Digimon with play cost <= 4
        # ST13-10 Gladimon: play cost 4
        runner.place_on_field(2, ["ST13-10"])

        game = runner.game
        runner.set_phase("Main")

        # Trash 1 digivolution card from the Golemon permanent
        trashed = perm.trash_digivolution_cards(1)
        assert len(trashed) == 1, "Should have trashed 1 card"
        assert trashed[0].c_entity_base.card_id == "BT21-055", (
            f"Should have trashed BT21-055, got {trashed[0].c_entity_base.card_id}"
        )

        # Move trashed cards to the owner's trash
        owner = game.player1
        owner.trash_cards.extend(trashed)

        runner.auto_resolve()

        # Opponent's Gladimon should be deleted
        snap = runner.snapshot()
        opp_ids = [s.card_id for s in snap.p2_field]
        assert "ST13-10" not in opp_ids, (
            f"Opponent's ST13-10 (cost 4) should be deleted. Opp field: {opp_ids}"
        )

    def test_inherited_does_not_trigger_from_non_mineral_rock_permanent(self, debug_runner):
        """When trashed from a non-Mineral/Rock Digimon, inherited must NOT trigger."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )

        # Stack: BT21-055 under ST13-10 Gladimon (Warrior trait, NOT Mineral/Rock)
        perm = runner.place_on_field(1, ["BT21-055", "ST13-10"])

        # Place opponent Digimon with play cost <= 4
        runner.place_on_field(2, ["ST13-10"])

        game = runner.game
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        opp_before = len(snap_before.p2_field)

        # Trash digivolution card
        trashed = perm.trash_digivolution_cards(1)
        owner = game.player1
        owner.trash_cards.extend(trashed)
        runner.auto_resolve()

        # Opponent's Digimon should still be there (no trigger)
        snap_after = runner.snapshot()
        opp_after = len(snap_after.p2_field)
        assert opp_after == opp_before, (
            f"Opponent's field should be unchanged (non-Mineral/Rock permanent). "
            f"Before: {opp_before}, After: {opp_after}"
        )

    def test_inherited_card_in_trashed_check(self, debug_runner):
        """The inherited condition must check that BT21-055 itself was among
        the trashed cards (not some other card)."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )

        # Stack: BT21-055 (bottom) + ST1-03 (middle) + BT4-066 Golemon (top, Mineral)
        perm = runner.place_on_field(1, ["BT21-055", "ST1-03", "BT4-066"])

        runner.place_on_field(2, ["ST13-10"])  # Opponent's cost-4 target

        game = runner.game
        runner.set_phase("Main")

        # Trash 1 card from top -- this should trash ST1-03, not BT21-055
        trashed = perm.trash_digivolution_cards(1)
        assert len(trashed) == 1
        assert trashed[0].c_entity_base.card_id == "ST1-03", (
            f"Should trash ST1-03 (just below top), got {trashed[0].c_entity_base.card_id}"
        )
        owner = game.player1
        owner.trash_cards.extend(trashed)
        runner.auto_resolve()

        # Opponent's Digimon should still be there (BT21-055 was not the card trashed)
        snap = runner.snapshot()
        opp_ids = [s.card_id for s in snap.p2_field]
        assert "ST13-10" in opp_ids, (
            f"Opponent's ST13-10 should survive (BT21-055 was not trashed). "
            f"Opp field: {opp_ids}"
        )

    def test_inherited_does_not_delete_high_cost_digimon(self, debug_runner):
        """The delete filter should only target Digimon with play cost 4 or less.
        Digimon with cost 5+ should not be selectable."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )

        perm = runner.place_on_field(1, ["BT21-055", "BT4-066"])

        # Place opponent Digimon with play cost > 4 only
        # ST1-07 Greymon: play cost 5 (Red Lv.4)
        runner.place_on_field(2, ["ST1-07"])

        game = runner.game
        runner.set_phase("Main")

        snap_before = runner.snapshot()

        trashed = perm.trash_digivolution_cards(1)
        owner = game.player1
        owner.trash_cards.extend(trashed)
        runner.auto_resolve()

        # Opponent's Greymon (cost 5) should NOT be deleted
        snap_after = runner.snapshot()
        opp_ids = [s.card_id for s in snap_after.p2_field]
        assert "ST1-07" in opp_ids, (
            f"ST1-07 (cost 5) should survive (only cost <= 4 can be deleted). "
            f"Opp field: {opp_ids}"
        )

    def test_inherited_triggers_from_rock_permanent(self, debug_runner):
        """The inherited should also trigger when trashed from a Rock trait Digimon."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, skip_shuffle=True,
        )

        # BT4-070 Meteormon: Lv.5 Black Rock
        # Stack: BT21-055 under Meteormon
        perm = runner.place_on_field(1, ["BT21-055", "BT4-070"])

        # Opponent's cost-4 target
        runner.place_on_field(2, ["ST13-10"])

        game = runner.game
        runner.set_phase("Main")

        trashed = perm.trash_digivolution_cards(1)
        assert trashed[0].c_entity_base.card_id == "BT21-055"
        owner = game.player1
        owner.trash_cards.extend(trashed)
        runner.auto_resolve()

        snap = runner.snapshot()
        opp_ids = [s.card_id for s in snap.p2_field]
        assert "ST13-10" not in opp_ids, (
            f"Opponent's ST13-10 should be deleted (Rock trait permanent). "
            f"Opp field: {opp_ids}"
        )
