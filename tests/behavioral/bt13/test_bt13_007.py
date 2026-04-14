"""Behavioral tests for BT13-007 King Drasil_7D6 (Digi-Egg, White).

Card text:
[Breeding] [Your Turn] All of your Digimon can't digivolve.
[Breeding] [Your Turn] [Once Per Turn] When a [Royal Knight] trait Digimon card
    would be played, you may reduce the play cost by 4. Further reduce it by 1
    for each of this Digimon's digivolution cards.
[Breeding] [Start of Your Main Phase] Reveal the top card of your Digi-Egg deck,
    then place that card and all of your [Royal Knight] trait Digimon as this
    Digimon's bottom digivolution cards.

Inherited Effect:
[Breeding] [Your Turn] [Once Per Turn] When an Option card with the [Royal Knight]
    trait is placed in the battle area, gain 1 memory.

C# Reference: DCGO/Assets/Scripts/CardEffect/BT13/White/BT13_007.cs
- CanNotDigivolveStaticEffect: continuous while in breeding, your turn only
- BeforePayCost: OPT hash "Reduce_BT13_007". Dynamic cost = DigivolutionCards.Count + 4
- Start of Main Phase: Reveals top digi-egg, absorbs it + all RK Digimon
- Inherited: OPT memory +1 when RK Option placed. Hash "Memory+1_BT13_007"
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase
from digimon_gym.engine.interfaces.modifiers import ModifierType


# -- Helpers -------------------------------------------------------------------

# Generic decks for DebugRunner. These are never used for their actual cards;
# state injection replaces everything we care about.
_FILLER_DECK = (
    ["BT1-001"] * 4  # digi-eggs
    + ["ST1-02"] * 4  # Lv.3
    + ["ST1-03"] * 4
    + ["ST1-05"] * 4
    + ["ST1-07"] * 4
    + ["ST1-08"] * 4
    + ["ST1-09"] * 4
    + ["ST1-11"] * 4
    + ["ST1-12"] * 4
    + ["ST1-13"] * 4
    + ["ST1-14"] * 4
    + ["ST1-16"] * 4
    + ["ST1-06"] * 6
)


@pytest.mark.behavioral
class TestBT13007CannotDigivolve:
    """[Breeding][Your Turn] All of your Digimon can't digivolve."""

    def test_cannot_digivolve_blocks_own_digimon(self, debug_runner):
        """When BT13-007 is in breeding during your turn, your Digimon on the
        battle area should not be able to digivolve."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        # Place BT13-007 in breeding
        runner.place_in_breeding(1, ["BT13-007"])
        # Place a Lv.3 Digimon on P1's field
        runner.place_on_field(1, ["ST1-03"])
        # Inject a Lv.4 into P1's hand that could digivolve onto the Lv.3
        runner.inject_card(1, "ST1-07", "hand")  # Greymon Lv.4

        game = runner.game
        breeding_perm = game.player1.breeding_area
        assert breeding_perm is not None

        # Must be in Main phase for digivolve actions to appear
        runner.set_phase("Main")
        game.active_player = game.player1
        # Trigger OnStartMainPhase to register CANNOT_DIGIVOLVE modifier
        game.execute_effects(EffectTiming.OnStartMainPhase)

        # Check action mask — digivolve actions should be masked out
        digivolve_actions = runner.find_actions("digivolve")
        assert len(digivolve_actions) == 0, (
            "All digivolve actions should be blocked while BT13-007 is in breeding. "
            f"Found: {digivolve_actions}"
        )

    def test_cannot_digivolve_only_during_your_turn(self, debug_runner):
        """The can't-digivolve restriction only applies during the owner's turn.
        On the opponent's turn, it should NOT block digivolve (per C# condition)."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        # BT13-007 belongs to P1 (in breeding)
        runner.place_in_breeding(1, ["BT13-007"])

        # Get the breeding perm and check the condition
        perm = runner.game.player1.breeding_area
        top = perm.top_card
        effects = top.effect_list(None)

        # Find the CANNOT_DIGIVOLVE effect
        nodigi_effects = [
            e for e in effects
            if "can't digivolve" in (getattr(e, 'effect_name', '') or '').lower()
        ]
        assert len(nodigi_effects) >= 1, "Should have a can't digivolve effect"

        # It should check owner.is_my_turn in its condition
        nodigi = nodigi_effects[0]
        # During P1's turn (is_my_turn=True), condition should pass
        runner.game.player1.is_my_turn = True
        ctx = {"permanent": perm, "player": runner.game.player1, "game": runner.game}
        assert nodigi.can_use_condition(ctx), \
            "Can't digivolve should be active during owner's turn"

        # During opponent's turn (is_my_turn=False), condition should fail
        runner.game.player1.is_my_turn = False
        assert not nodigi.can_use_condition(ctx), \
            "Can't digivolve should NOT be active during opponent's turn"

    def test_cannot_digivolve_only_when_in_breeding(self, debug_runner):
        """The can't-digivolve restriction only applies while BT13-007 is in
        the breeding area. If moved to field it should not apply."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])
        top = perm.top_card
        effects = top.effect_list(None)

        nodigi_effects = [
            e for e in effects
            if "can't digivolve" in (getattr(e, 'effect_name', '') or '').lower()
        ]
        assert len(nodigi_effects) >= 1
        nodigi = nodigi_effects[0]

        # When in breeding: condition should pass
        runner.game.player1.is_my_turn = True
        assert nodigi.can_use_condition({
            "permanent": perm,
            "player": runner.game.player1,
            "game": runner.game,
        })

        # Simulate removal from breeding (set breeding_area to None)
        runner.game.player1.breeding_area = None
        assert not nodigi.can_use_condition({
            "permanent": perm,
            "player": runner.game.player1,
            "game": runner.game,
        }), "Can't digivolve should NOT be active when not in breeding area"

    def test_cannot_digivolve_does_not_affect_opponent(self, debug_runner):
        """Only YOUR Digimon can't digivolve. The CANNOT_DIGIVOLVE modifier
        should NOT be registered on opponent's Digimon."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        # BT13-007 in P1's breeding
        perm = runner.place_in_breeding(1, ["BT13-007"])
        # P2 has a Digimon on field
        opp_perm = runner.place_on_field(2, ["ST1-03"])

        game = runner.game

        # The CANNOT_DIGIVOLVE process registers modifiers on P1's battle area
        # permanents, not P2's. Verify P2's Digimon has no such modifier.
        assert not game.modifiers.has_modifier(opp_perm, ModifierType.CANNOT_DIGIVOLVE), (
            "Opponent's Digimon should NOT have CANNOT_DIGIVOLVE modifier from "
            "player 1's BT13-007"
        )


@pytest.mark.behavioral
class TestBT13007CostReduction:
    """[Breeding][Your Turn][Once Per Turn] Royal Knight play cost reduction."""

    def test_cost_reduction_basic(self, debug_runner):
        """Playing a Royal Knight Digimon with BT13-007 in breeding should reduce
        the play cost by 4 (base) + number of digivolution cards."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=15,
        )
        # Place BT13-007 alone in breeding (0 digi-cards under it)
        perm = runner.place_in_breeding(1, ["BT13-007"])

        # BT8-038 Magnamon: Royal Knight Digimon, play cost 7
        runner.inject_card(1, "BT8-038", "hand")

        game = runner.game
        # Find the card in hand
        magnamon = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT8-038":
                magnamon = c
                break
        assert magnamon is not None, "Magnamon should be in hand"

        # Calculate cost: base 7, reduction = 4 + 0 digi-cards = 4, final = 3
        cost = game.calculate_play_cost(game.player1, magnamon)
        assert cost == 3, (
            f"Expected play cost 3 (7 - 4), got {cost}. "
            "BT13-007 with 0 digi-cards should reduce by 4."
        )

    def test_cost_reduction_with_digi_cards(self, debug_runner):
        """Cost reduction increases by 1 for each digivolution card under BT13-007."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=15,
        )
        # Place BT13-007 with 2 digivolution cards under it
        # Stack: [BT1-001, ST1-02, BT13-007] — 2 digi-cards
        perm = runner.place_in_breeding(1, ["BT1-001", "ST1-02", "BT13-007"])

        runner.inject_card(1, "BT8-038", "hand")  # Magnamon, cost 7

        game = runner.game
        magnamon = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT8-038":
                magnamon = c
                break
        assert magnamon is not None

        # Reduction = 4 + 2 digi-cards = 6, cost = max(0, 7 - 6) = 1
        cost = game.calculate_play_cost(game.player1, magnamon)
        assert cost == 1, (
            f"Expected play cost 1 (7 - 6), got {cost}. "
            "BT13-007 with 2 digi-cards should reduce by 6."
        )

    def test_cost_reduction_floors_at_zero(self, debug_runner):
        """Cost reduction should not go below 0."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=15,
        )
        # Place BT13-007 with 5 digi-cards under it
        perm = runner.place_in_breeding(1, [
            "BT1-001", "ST1-02", "ST1-03", "ST1-05", "ST1-07", "BT13-007"
        ])

        runner.inject_card(1, "BT8-038", "hand")  # Magnamon, cost 7

        game = runner.game
        magnamon = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT8-038":
                magnamon = c
                break
        assert magnamon is not None

        # Reduction = 4 + 5 = 9, cost = max(0, 7 - 9) = 0
        cost = game.calculate_play_cost(game.player1, magnamon)
        assert cost == 0, (
            f"Expected play cost 0 (7 - 9 floored), got {cost}."
        )

    def test_cost_reduction_only_for_royal_knight_digimon(self, debug_runner):
        """Cost reduction should NOT apply to non-Royal Knight Digimon."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=15,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])

        # ST1-08 MetalGreymon: NOT a Royal Knight, play cost 8
        runner.inject_card(1, "ST1-08", "hand")

        game = runner.game
        metal = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "ST1-08":
                metal = c
                break
        assert metal is not None

        cost = game.calculate_play_cost(game.player1, metal)
        assert cost == metal.get_cost_itself, (
            f"Non-RK Digimon cost should be unchanged. "
            f"Expected {metal.get_cost_itself}, got {cost}."
        )

    def test_cost_reduction_only_for_digimon_not_options(self, debug_runner):
        """Cost reduction should NOT apply to Option cards (even with RK trait)."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=15,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])

        # BT10-110 Seiken Meppa: Royal Knight Option, cost 3
        runner.inject_card(1, "BT10-110", "hand")

        game = runner.game
        option = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT10-110":
                option = c
                break
        assert option is not None

        cost = game.calculate_play_cost(game.player1, option)
        assert cost == option.get_cost_itself, (
            f"RK Option cost should NOT be reduced. "
            f"Expected {option.get_cost_itself}, got {cost}."
        )

    def test_cost_reduction_once_per_turn(self, debug_runner):
        """Cost reduction should only apply once per turn (hash-tracked)."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=15,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])
        runner.inject_card(1, "BT8-038", "hand")
        runner.inject_card(1, "BT13-040", "hand")  # Another RK Digimon

        game = runner.game

        # Find both RK cards
        magnamon = None
        magnamon2 = None
        for c in game.player1.hand_cards:
            if c.c_entity_base:
                if c.c_entity_base.card_id == "BT8-038" and magnamon is None:
                    magnamon = c
                elif c.c_entity_base.card_id == "BT13-040" and magnamon2 is None:
                    magnamon2 = c
        assert magnamon is not None
        assert magnamon2 is not None

        # First calculation should apply reduction (commit=True records activation)
        cost1 = game.calculate_play_cost(game.player1, magnamon, commit=True)
        assert cost1 < magnamon.get_cost_itself, "First RK should get cost reduction"

        # Second calculation for another RK — once per turn exhausted
        cost2 = game.calculate_play_cost(game.player1, magnamon2)
        assert cost2 == magnamon2.get_cost_itself, (
            f"Second RK Digimon should NOT get cost reduction (once per turn). "
            f"Expected {magnamon2.get_cost_itself}, got {cost2}."
        )

    def test_cost_reduction_is_optional(self, debug_runner):
        """The cost reduction effect should be marked as optional."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=15,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])
        top = perm.top_card
        effects = top.effect_list(None)

        cost_effects = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.BeforePayCost
        ]
        assert len(cost_effects) >= 1
        assert cost_effects[0].is_optional, \
            "Cost reduction should be optional per card text 'you may'"

    def test_cost_reduction_not_on_opponent_turn(self, debug_runner):
        """Cost reduction should not apply during opponent's turn."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=15,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])
        runner.inject_card(1, "BT8-038", "hand")

        game = runner.game
        magnamon = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT8-038":
                magnamon = c
                break
        assert magnamon is not None

        # Simulate opponent's turn
        game.player1.is_my_turn = False
        cost = game.calculate_play_cost(game.player1, magnamon)
        assert cost == magnamon.get_cost_itself, (
            "Cost reduction should NOT apply during opponent's turn."
        )

    def test_cost_reduction_not_when_not_in_breeding(self, debug_runner):
        """Cost reduction should not apply if BT13-007 is not in breeding area."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=15,
        )
        # Place BT13-007 on the field instead of breeding
        perm = runner.place_on_field(1, ["BT13-007"])
        runner.inject_card(1, "BT8-038", "hand")

        game = runner.game
        magnamon = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT8-038":
                magnamon = c
                break
        assert magnamon is not None

        cost = game.calculate_play_cost(game.player1, magnamon)
        assert cost == magnamon.get_cost_itself, (
            "Cost reduction should NOT apply when BT13-007 is on field, not breeding."
        )


@pytest.mark.behavioral
class TestBT13007AbsorbEffect:
    """[Breeding][Start of Your Main Phase] Absorb digi-egg + Royal Knight Digimon."""

    def test_absorb_digi_egg(self, debug_runner):
        """Start of main phase should reveal and absorb top digi-egg card."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])

        game = runner.game
        # Ensure there's a digi-egg in the library
        egg_count_before = len(game.player1.digitama_library_cards)
        assert egg_count_before >= 1, "Need at least 1 digi-egg in library"
        top_egg = game.player1.digitama_library_cards[0]

        cards_before = len(perm.card_sources)

        # Find and execute the OnStartMainPhase effect
        top = perm.top_card
        effects = top.effect_list(None)
        absorb_effects = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnStartMainPhase
            and "absorb" in (getattr(e, 'effect_name', '') or '').lower()
        ]
        assert len(absorb_effects) >= 1, "Should have OnStartMainPhase absorb effect"

        absorb = absorb_effects[0]
        absorb.on_process_callback({
            "player": game.player1,
            "permanent": perm,
            "game": game,
        })

        # Digi-egg deck should have 1 fewer card
        assert len(game.player1.digitama_library_cards) == egg_count_before - 1, \
            "Should remove top digi-egg from library"

        # The absorbed egg should be in the breeding perm's card_sources
        assert top_egg in perm.card_sources, \
            "Absorbed digi-egg should be in the breeding perm's card stack"

        # It should be at the BOTTOM (index 0)
        assert perm.card_sources[0] is top_egg, \
            "Absorbed card should be placed as bottom digivolution card"

    def test_absorb_royal_knight_digimon(self, debug_runner):
        """Start of main phase should absorb all Royal Knight trait Digimon
        from the battle area."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])

        # Place a Royal Knight Digimon on field
        # BT2-020 Gallantmon: Royal Knight, Lv.6
        rk_perm = runner.place_on_field(1, ["BT2-020"])

        game = runner.game
        rk_top = rk_perm.top_card
        field_before = len(game.player1.battle_area)

        top = perm.top_card
        effects = top.effect_list(None)
        absorb = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnStartMainPhase
            and "absorb" in (getattr(e, 'effect_name', '') or '').lower()
        ][0]

        absorb.on_process_callback({
            "player": game.player1,
            "permanent": perm,
            "game": game,
        })

        # RK Digimon should be removed from battle area
        assert rk_perm not in game.player1.battle_area, \
            "Royal Knight Digimon should be removed from battle area"

        # Its card sources should be in the breeding perm's stack
        assert rk_top in perm.card_sources, \
            "Royal Knight's card source should be in breeding perm's stack"

    def test_absorb_does_not_take_non_rk_digimon(self, debug_runner):
        """Non-Royal Knight Digimon should NOT be absorbed."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])

        # Place a non-RK Digimon
        non_rk_perm = runner.place_on_field(1, ["ST1-08"])  # MetalGreymon

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        absorb = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnStartMainPhase
            and "absorb" in (getattr(e, 'effect_name', '') or '').lower()
        ][0]

        absorb.on_process_callback({
            "player": game.player1,
            "permanent": perm,
            "game": game,
        })

        # Non-RK Digimon should still be on field
        assert non_rk_perm in game.player1.battle_area, \
            "Non-Royal Knight Digimon should NOT be absorbed"

    def test_absorb_multiple_rk_digimon(self, debug_runner):
        """Should absorb ALL Royal Knight Digimon, not just one."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])

        # Place 2 RK Digimon
        rk1 = runner.place_on_field(1, ["BT2-020"])  # Gallantmon
        rk2 = runner.place_on_field(1, ["BT3-030"])  # Leopardmon

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        absorb = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnStartMainPhase
            and "absorb" in (getattr(e, 'effect_name', '') or '').lower()
        ][0]

        absorb.on_process_callback({
            "player": game.player1,
            "permanent": perm,
            "game": game,
        })

        # Both RK Digimon should be removed
        assert rk1 not in game.player1.battle_area, "First RK should be absorbed"
        assert rk2 not in game.player1.battle_area, "Second RK should be absorbed"

    def test_absorb_rk_with_digi_stack(self, debug_runner):
        """When absorbing a RK Digimon with its own digivolution stack, all cards
        from that stack should become digivolution cards of the breeding perm."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])

        # Place a RK Digimon with a digivolution stack
        rk_perm = runner.place_on_field(1, ["ST1-03", "ST1-07", "BT2-020"])
        rk_sources = list(rk_perm.card_sources)  # 3 cards
        assert len(rk_sources) == 3

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        absorb = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnStartMainPhase
            and "absorb" in (getattr(e, 'effect_name', '') or '').lower()
        ][0]

        absorb.on_process_callback({
            "player": game.player1,
            "permanent": perm,
            "game": game,
        })

        # All 3 cards from the RK stack should be in the breeding perm
        for cs in rk_sources:
            assert cs in perm.card_sources, \
                f"Card {cs.c_entity_base.card_id if cs.c_entity_base else '?'} from RK stack should be absorbed"

    def test_absorb_empty_digi_egg_deck(self, debug_runner):
        """If digi-egg deck is empty, should still absorb RK Digimon."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])

        # Clear the digi-egg deck
        runner.game.player1.digitama_library_cards.clear()

        rk_perm = runner.place_on_field(1, ["BT2-020"])

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        absorb = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnStartMainPhase
            and "absorb" in (getattr(e, 'effect_name', '') or '').lower()
        ][0]

        cards_before = len(perm.card_sources)
        absorb.on_process_callback({
            "player": game.player1,
            "permanent": perm,
            "game": game,
        })

        # RK should still be absorbed even with no digi-egg
        assert rk_perm not in game.player1.battle_area
        assert len(perm.card_sources) > cards_before

    def test_absorb_condition_requires_breeding(self, debug_runner):
        """The absorb effect should only fire when BT13-007 is in breeding."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])

        top = perm.top_card
        effects = top.effect_list(None)
        absorb = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnStartMainPhase
            and "absorb" in (getattr(e, 'effect_name', '') or '').lower()
        ][0]

        # When in breeding: condition should pass
        assert absorb.can_use_condition({
            "permanent": perm,
            "player": runner.game.player1,
            "game": runner.game,
        })

        # Remove from breeding
        runner.game.player1.breeding_area = None
        assert not absorb.can_use_condition({
            "permanent": perm,
            "player": runner.game.player1,
            "game": runner.game,
        }), "Absorb should NOT trigger when not in breeding"

    def test_absorb_does_not_take_opponent_rk(self, debug_runner):
        """Should only absorb YOUR Royal Knight Digimon, not opponent's."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])

        # Place RK on opponent's field
        opp_rk = runner.place_on_field(2, ["BT2-020"])

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        absorb = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnStartMainPhase
            and "absorb" in (getattr(e, 'effect_name', '') or '').lower()
        ][0]

        absorb.on_process_callback({
            "player": game.player1,
            "permanent": perm,
            "game": game,
        })

        # Opponent's RK should NOT be absorbed
        assert opp_rk in game.player2.battle_area, \
            "Opponent's Royal Knight should NOT be absorbed"

    def test_absorb_cleans_up_modifiers(self, debug_runner):
        """When absorbing a RK Digimon, its modifiers should be cleaned up."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])
        rk_perm = runner.place_on_field(1, ["BT2-020"])

        game = runner.game

        # Register a test modifier on the RK perm
        from digimon_gym.engine.interfaces.modifiers import ModifierEntry
        test_mod = ModifierEntry(
            modifier_type=ModifierType.CHANGE_DP,
            condition=None,
            value_fn=lambda p, ctx: 1000,
            source_effect=None,
            source_permanent=rk_perm,
        )
        game.modifiers.register(test_mod)

        top = perm.top_card
        effects = top.effect_list(None)
        absorb = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnStartMainPhase
            and "absorb" in (getattr(e, 'effect_name', '') or '').lower()
        ][0]

        absorb.on_process_callback({
            "player": game.player1,
            "permanent": perm,
            "game": game,
        })

        # After absorption, modifiers from the absorbed perm should be cleaned
        assert not game.modifiers.has_modifier(rk_perm, ModifierType.CHANGE_DP), \
            "Modifiers from absorbed permanent should be cleaned up"


@pytest.mark.behavioral
class TestBT13007InheritedEffect:
    """Inherited: [Breeding][Your Turn][Once Per Turn] When an Option card with
    the [Royal Knight] trait is placed in the battle area, gain 1 memory."""

    def test_inherited_memory_gain_on_rk_option(self, debug_runner):
        """Should gain 1 memory when a Royal Knight Option is played."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)

        # Find the inherited memory effect
        mem_effects = [
            e for e in effects
            if "gain 1 memory" in (getattr(e, 'effect_name', '') or '').lower()
            or "memory" in (getattr(e, 'effect_name', '') or '').lower()
        ]
        assert len(mem_effects) >= 1, "Should have inherited memory gain effect"

        mem_effect = mem_effects[0]

        # Create a mock RK Option card source for condition check
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        rk_option = db.create_card_source("BT10-110", game.player1)  # Seiken Meppa
        assert rk_option is not None
        assert rk_option.is_option

        # Condition should pass with RK Option as played_card
        result = mem_effect.can_use_condition({
            "permanent": perm,
            "player": game.player1,
            "game": game,
            "played_card": rk_option,
        })
        assert result, "Condition should pass for RK Option played_card"

        # Execute the effect
        mem_before = game.memory
        mem_effect.on_process_callback({
            "player": game.player1,
            "game": game,
            "permanent": perm,
        })
        assert game.memory == mem_before + 1, \
            f"Should gain 1 memory. Before: {mem_before}, After: {game.memory}"

    def test_inherited_no_memory_for_non_rk_option(self, debug_runner):
        """Should NOT gain memory for Options without Royal Knight trait."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)

        mem_effects = [
            e for e in effects
            if "gain 1 memory" in (getattr(e, 'effect_name', '') or '').lower()
            or "memory" in (getattr(e, 'effect_name', '') or '').lower()
        ]
        assert len(mem_effects) >= 1
        mem_effect = mem_effects[0]

        # Use a non-RK option (find one)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        non_rk_option = db.create_card_source("ST1-16", game.player1)  # Gaia Force (not RK)
        assert non_rk_option is not None

        result = mem_effect.can_use_condition({
            "permanent": perm,
            "player": game.player1,
            "game": game,
            "played_card": non_rk_option,
        })
        assert not result, "Should NOT trigger for non-RK Option"

    def test_inherited_no_memory_for_rk_digimon(self, debug_runner):
        """Should NOT gain memory for RK Digimon — only Options."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)

        mem_effects = [
            e for e in effects
            if "gain 1 memory" in (getattr(e, 'effect_name', '') or '').lower()
            or "memory" in (getattr(e, 'effect_name', '') or '').lower()
        ]
        assert len(mem_effects) >= 1
        mem_effect = mem_effects[0]

        # Use a RK Digimon (not an Option)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        rk_digimon = db.create_card_source("BT2-020", game.player1)  # Gallantmon (Digimon)
        assert rk_digimon is not None

        result = mem_effect.can_use_condition({
            "permanent": perm,
            "player": game.player1,
            "game": game,
            "played_card": rk_digimon,
        })
        assert not result, "Should NOT trigger for RK Digimon (only Options)"

    def test_inherited_once_per_turn(self, debug_runner):
        """Inherited memory gain should only activate once per turn."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)

        mem_effects = [
            e for e in effects
            if "gain 1 memory" in (getattr(e, 'effect_name', '') or '').lower()
            or "memory" in (getattr(e, 'effect_name', '') or '').lower()
        ]
        assert len(mem_effects) >= 1
        mem_effect = mem_effects[0]

        assert mem_effect.max_count_per_turn == 1, \
            "Inherited effect should be limited to once per turn"

    def test_inherited_requires_breeding(self, debug_runner):
        """Inherited effect should only trigger when BT13-007 is in breeding."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)

        mem_effects = [
            e for e in effects
            if "gain 1 memory" in (getattr(e, 'effect_name', '') or '').lower()
            or "memory" in (getattr(e, 'effect_name', '') or '').lower()
        ]
        assert len(mem_effects) >= 1
        mem_effect = mem_effects[0]

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        rk_option = db.create_card_source("BT10-110", game.player1)

        # Should pass when in breeding
        assert mem_effect.can_use_condition({
            "permanent": perm,
            "player": game.player1,
            "game": game,
            "played_card": rk_option,
        })

        # Remove from breeding
        game.player1.breeding_area = None
        assert not mem_effect.can_use_condition({
            "permanent": perm,
            "player": game.player1,
            "game": game,
            "played_card": rk_option,
        }), "Inherited effect should NOT trigger when not in breeding"


@pytest.mark.behavioral
class TestBT13007EffectStructure:
    """Verify effect structure: timing, hash strings, flags."""

    def test_has_all_four_effects(self, debug_runner):
        """BT13-007 should have 4 effects: can't digivolve, cost reduction,
        absorb, inherited memory."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])
        top = perm.top_card
        effects = top.effect_list(None)

        assert len(effects) == 4, (
            f"BT13-007 should have exactly 4 effects, got {len(effects)}. "
            f"Names: {[getattr(e, 'effect_name', '?') for e in effects]}"
        )

    def test_cost_reduction_hash_string(self, debug_runner):
        """Cost reduction effect should have a hash string for once-per-turn tracking."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])
        top = perm.top_card
        effects = top.effect_list(None)

        cost_effects = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.BeforePayCost
        ]
        assert len(cost_effects) >= 1
        hash_str = getattr(cost_effects[0], 'hash_string', None)
        assert hash_str is not None and hash_str != '', \
            "Cost reduction should have a hash string for OPT tracking"

    def test_inherited_memory_hash_string(self, debug_runner):
        """Inherited memory effect should have a hash string."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])
        top = perm.top_card
        effects = top.effect_list(None)

        # The inherited effect uses OnEnterFieldAnyone timing
        mem_effects = [
            e for e in effects
            if "memory" in (getattr(e, 'effect_name', '') or '').lower()
        ]
        assert len(mem_effects) >= 1
        hash_str = getattr(mem_effects[0], 'hash_string', None)
        assert hash_str is not None and hash_str != '', \
            "Inherited memory effect should have a hash string for OPT tracking"

    def test_cost_reduction_allow_breeding_source(self, debug_runner):
        """Cost reduction effect should have _allow_breeding_source=True."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])
        top = perm.top_card
        effects = top.effect_list(None)

        cost_effects = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.BeforePayCost
        ]
        assert len(cost_effects) >= 1
        assert getattr(cost_effects[0], '_allow_breeding_source', False), \
            "Cost reduction must have _allow_breeding_source=True for breeding area scanning"

    def test_cost_reduction_self_play_guard(self, debug_runner):
        """Cost reduction should not apply when playing BT13-007 itself
        (card_source is card guard)."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=15,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])
        top = perm.top_card
        effects = top.effect_list(None)

        cost_effects = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.BeforePayCost
        ]
        assert len(cost_effects) >= 1
        effect = cost_effects[0]

        # The condition should reject when card_source is the card itself
        result = effect.can_use_condition({
            "permanent": perm,
            "player": runner.game.player1,
            "game": runner.game,
            "card_source": top,  # self
        })
        assert not result, \
            "Cost reduction should NOT apply to playing BT13-007 itself"


@pytest.mark.behavioral
class TestBT13007CSharpFaithfulness:
    """Tests for C# faithfulness gaps identified by comparing BT13_007.cs."""

    def test_absorb_skips_token_rk_digimon(self, debug_runner):
        """C# checks !permanent.IsToken before absorbing. Token RK Digimon
        should NOT be absorbed. (BT13_007.cs line 320: !permanent.IsToken)"""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_in_breeding(1, ["BT13-007"])

        # Place a token RK Digimon on field
        # We need to create a token and mark it as such
        rk_perm = runner.place_on_field(1, ["BT2-020"])  # Gallantmon (RK)
        rk_perm.top_card.is_token = True  # Mark as token

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        absorb = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnStartMainPhase
            and "absorb" in (getattr(e, 'effect_name', '') or '').lower()
        ][0]

        absorb.on_process_callback({
            "player": game.player1,
            "permanent": perm,
            "game": game,
        })

        # Token RK should NOT be absorbed (per C#)
        assert rk_perm in game.player1.battle_area, (
            "Token Royal Knight Digimon should NOT be absorbed. "
            "C# checks !permanent.IsToken at line 320."
        )

    def test_cannot_digivolve_preexisting_digimon(self, debug_runner):
        """The can't-digivolve restriction must apply to Digimon that were
        already on the field before anything enters. C# uses a continuous
        static effect (CanNotDigivolveStaticEffect) that checks on every
        digivolve attempt. The Python implementation must block digivolution
        regardless of when the field Digimon arrived."""
        runner = debug_runner(
            deck1=_FILLER_DECK, deck2=_FILLER_DECK,
            initial_memory=5,
        )
        # Place a Lv.3 on field BEFORE BT13-007 goes in breeding
        runner.place_on_field(1, ["ST1-03"])  # Agumon Lv.3
        runner.inject_card(1, "ST1-07", "hand")  # Greymon Lv.4 in hand

        # Now place BT13-007 in breeding
        runner.place_in_breeding(1, ["BT13-007"])

        # Must be in Main phase for digivolve actions to appear
        runner.set_phase("Main")
        runner.game.active_player = runner.game.player1
        # Trigger OnStartMainPhase to register CANNOT_DIGIVOLVE modifier
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        # The pre-existing Digimon should also be blocked from digivolving
        digivolve_actions = runner.find_actions("digivolve")
        assert len(digivolve_actions) == 0, (
            "Pre-existing Digimon should also be blocked from digivolving. "
            "C# uses CanNotDigivolveStaticEffect (continuous). "
            f"Found digivolve actions: {digivolve_actions}"
        )
