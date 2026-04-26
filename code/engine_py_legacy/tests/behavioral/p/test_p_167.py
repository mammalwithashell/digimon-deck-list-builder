"""Behavioral tests for P-167 Landramon (Lv.4, Black, Mineral/LIBERATOR).

Card text:
[Start of Your Main Phase] [When Digivolving] By trashing 1 [Mineral] or [Rock]
trait card from any of your Digimon's digivolution cards, reveal the top 3 cards
of your deck. Among them, add 1 [Mineral] or [Rock] trait card to the hand, or
place 1 such card as this Digimon's bottom digivolution card. Return the rest to
the top or bottom of the deck.

Inherited: When effects trash this card from digivolution cards of a [Mineral] or
[Rock] trait Digimon, <De-Digivolve 1> 1 of your opponent's Digimon.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


# --- Card IDs ---
P_167 = "P-167"             # Landramon Lv.4, Mineral/LIBERATOR (the card under test)
BT4_066 = "BT4-066"         # Golemon Lv.4, Mineral
BT2_054 = "BT2-054"         # Gotsumon Lv.3, Rock
EX8_051 = "EX8-051"         # Proganomon Lv.5, Mineral/LIBERATOR
EX8_055 = "EX8-055"         # Pyramidimon Lv.6, Mineral/LIBERATOR
ST1_03 = "ST1-03"           # Agumon Lv.3, non-Mineral/Rock filler
ST1_11 = "ST1-11"           # WarGreymon Lv.6, Red, generic opponent

FILLER = ST1_03
FILLER_DECK = [FILLER] * 54


@pytest.mark.behavioral
class TestP167WhenDigivolving:
    """[When Digivolving] Trash 1 Mineral/Rock source, reveal 3, add/place."""

    def test_when_digivolving_effect_exists(self, debug_runner):
        """Should have a When Digivolving effect."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [P_167])

        card = perm.top_card
        effects = card.effect_list(None)
        wd_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        assert len(wd_effects) >= 1, "Should have a When Digivolving effect"

    def test_when_digivolving_effect_is_optional(self, debug_runner):
        """When Digivolving effect uses 'By trashing' which is optional."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [P_167])

        card = perm.top_card
        effects = card.effect_list(None)
        wd_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        assert wd_effects[0].is_optional, "When Digivolving effect should be optional"

    def test_condition_fails_without_mineral_rock_sources(self, debug_runner):
        """Condition should fail when no Digimon has Mineral/Rock digivolution cards."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        # Only P-167 on field with no sources underneath
        perm = runner.place_on_field(1, [P_167])

        card = perm.top_card
        effects = card.effect_list(None)
        wd_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        ctx = {"permanent": perm, "player": runner.game.player1, "game": runner.game}
        assert not wd_effects[0].can_use_condition(ctx), \
            "Condition should fail without Mineral/Rock sources"

    def test_condition_passes_with_mineral_source(self, debug_runner):
        """Condition passes when a Digimon has a Mineral digivolution card underneath."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        # BT4-066 Golemon (Mineral) under EX8-051 Proganomon
        perm = runner.place_on_field(1, [BT4_066, EX8_051])
        # P-167 needs to be on field for its condition to pass
        p167_perm = runner.place_on_field(1, [P_167])

        card = p167_perm.top_card
        effects = card.effect_list(None)
        wd_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        ctx = {"permanent": p167_perm, "player": runner.game.player1, "game": runner.game}
        assert wd_effects[0].can_use_condition(ctx), \
            "Condition should pass with Mineral source available"

    def test_condition_passes_with_rock_source(self, debug_runner):
        """Condition passes when a Digimon has a Rock digivolution card underneath."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        # BT2-054 Gotsumon (Rock) under P-167
        perm = runner.place_on_field(1, [BT2_054, P_167])

        card = perm.top_card
        effects = card.effect_list(None)
        wd_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        ctx = {"permanent": perm, "player": runner.game.player1, "game": runner.game}
        assert wd_effects[0].can_use_condition(ctx), \
            "Condition should pass with Rock source available"

    def test_trash_uses_trash_specific_api(self, debug_runner):
        """Effect should use trash_specific_digivolution_cards API, not manual remove.

        Verify by checking that OnDigivolutionCardDiscarded fires BEFORE the card
        is removed from card_sources (the proper API fires before removal)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        # Rock source under P-167
        perm = runner.place_on_field(1, [BT2_054, P_167])

        game = runner.game
        player = game.player1

        # Track whether the timing fires while cards are still in card_sources
        timing_context_captured = []

        original_fire = perm._fire_timing

        def tracking_fire(timing, context=None):
            if timing == EffectTiming.OnDigivolutionCardDiscarded:
                trashed = context.get('trashed_cards', []) if context else []
                # Check that trashed cards are still in card_sources (fired before removal)
                still_in = [cs for cs in trashed if cs in perm.card_sources]
                timing_context_captured.append({
                    'trashed': trashed,
                    'still_in_sources': still_in,
                })
            return original_fire(timing, context)

        perm._fire_timing = tracking_fire

        card = perm.top_card
        effects = card.effect_list(None)
        wd_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        effect = wd_effects[0]
        effect.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm,
        })

        # effect_select_own_permanent enters SelectTarget phase.
        # Explicitly select the permanent (action 100 = SEL_MY_FIELD_START + 0)
        # rather than auto_resolve which would decline the optional selection.
        from engine_py_legacy.engine.game.constants import SEL_MY_FIELD_START
        from engine_py_legacy.engine.data.enums import GamePhase
        assert game.current_phase == GamePhase.SelectTarget, \
            f"Should be in SelectTarget phase, got {game.current_phase}"
        game.decode_action(SEL_MY_FIELD_START + 0, game.current_player_id)

        # Auto-resolve remaining selections (branch choices, top/bottom)
        runner.auto_resolve()

        assert len(timing_context_captured) >= 1, \
            "OnDigivolutionCardDiscarded should fire"
        # The API fires BEFORE removal, so trashed cards should still be in sources
        # when the timing fires
        captured = timing_context_captured[0]
        assert len(captured['still_in_sources']) > 0, \
            "Trashed cards should still be in card_sources when timing fires (API fires before removal)"


@pytest.mark.behavioral
class TestP167StartOfMainPhase:
    """[Start of Your Main Phase] Same effect as When Digivolving."""

    def test_start_main_phase_effect_exists(self, debug_runner):
        """Should have a Start of Main Phase effect."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [P_167])

        card = perm.top_card
        effects = card.effect_list(None)
        somp_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnStartMainPhase
        ]
        assert len(somp_effects) >= 1, "Should have Start of Main Phase effect"

    def test_start_main_phase_is_optional(self, debug_runner):
        """Start of Main Phase effect should be optional."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [P_167])

        card = perm.top_card
        effects = card.effect_list(None)
        somp_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnStartMainPhase
        ]
        assert somp_effects[0].is_optional, "Start of Main Phase effect should be optional"

    def test_start_main_condition_requires_own_turn(self, debug_runner):
        """Condition should fail on opponent's turn."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [BT2_054, P_167])

        game = runner.game
        player = game.player1
        # Simulate not being player's turn
        player.is_my_turn = False

        card = perm.top_card
        effects = card.effect_list(None)
        somp_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnStartMainPhase
        ]
        ctx = {"permanent": perm, "player": player, "game": game}
        assert not somp_effects[0].can_use_condition(ctx), \
            "Start of Main Phase should only trigger on own turn"


@pytest.mark.behavioral
class TestP167InheritedDedigivolve:
    """Inherited: When effects trash this card from digivolution cards of a
    [Mineral] or [Rock] trait Digimon, <De-Digivolve 1> 1 of opponent's Digimon."""

    def test_inherited_effect_exists(self, debug_runner):
        """Should have an OnDigivolutionCardDiscarded inherited effect."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [P_167])

        card = perm.top_card
        effects = card.effect_list(None)
        inh_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnDigivolutionCardDiscarded
            and e.is_inherited_effect
        ]
        assert len(inh_effects) >= 1, \
            "Should have OnDigivolutionCardDiscarded inherited effect"

    def test_inherited_triggers_when_this_card_trashed_from_mineral_digimon(self, debug_runner):
        """Inherited should trigger when P-167 itself is trashed from a Mineral Digimon."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        # Stack: P-167 under EX8-055 Pyramidimon (Mineral trait on top)
        perm = runner.place_on_field(1, [P_167, EX8_055])
        # Opponent Digimon to de-digivolve
        opp_perm = runner.place_on_field(2, [ST1_03, ST1_11])
        opp_initial_sources = len(opp_perm.card_sources)

        game = runner.game
        player = game.player1

        # Get the P-167 CardSource (it's under the top card)
        p167_cs = perm.card_sources[0]
        assert p167_cs.c_entity_base.card_id == P_167

        # Use trash_specific_digivolution_cards to trash P-167
        trashed = perm.trash_specific_digivolution_cards([p167_cs])
        player.trash_cards.extend(trashed)

        # Auto-resolve the de-digivolve target selection
        runner.auto_resolve()

        # Opponent should have lost 1 source (de-digivolve 1)
        assert len(opp_perm.card_sources) == opp_initial_sources - 1, \
            f"Opponent should be de-digivolved by 1, was {opp_initial_sources}, now {len(opp_perm.card_sources)}"

    def test_inherited_triggers_when_this_card_trashed_from_rock_digimon(self, debug_runner):
        """Inherited should trigger when P-167 is trashed from a Rock trait Digimon."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        # BT4-070 Meteormon is Lv5/Rock but let's use a simpler setup
        # Stack: P-167 (Mineral) under BT2-054 (Gotsumon Lv3 Rock) -- bad level order
        # Actually we need Rock on top for trait check. Use a different combo.
        # Let's put P-167 under Gotsumon ST1-03 won't work (not Rock)...
        # Actually the trait check is on the PERMANENT (top card). So we need a
        # Rock-trait Digimon with P-167 underneath.
        # BT4-070 Meteormon Lv5 Rock
        perm = runner.place_on_field(1, [P_167, "BT4-070"])
        opp_perm = runner.place_on_field(2, [ST1_03, ST1_11])
        opp_initial_sources = len(opp_perm.card_sources)

        game = runner.game
        player = game.player1

        p167_cs = perm.card_sources[0]
        assert p167_cs.c_entity_base.card_id == P_167

        trashed = perm.trash_specific_digivolution_cards([p167_cs])
        player.trash_cards.extend(trashed)
        runner.auto_resolve()

        assert len(opp_perm.card_sources) == opp_initial_sources - 1, \
            "Opponent should be de-digivolved when P-167 trashed from Rock Digimon"

    def test_inherited_does_NOT_trigger_when_other_card_trashed(self, debug_runner):
        """Inherited should NOT fire when a different card is trashed, not P-167 itself.

        This is the key faithfulness check: 'When effects trash THIS CARD' means
        only P-167 itself, not any other card being trashed from the same Digimon.
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        # Stack: BT2-054 (Gotsumon/Rock) then P-167 then EX8-055 (Pyramidimon/Mineral)
        # P-167 is a digivolution card (not top), so its inherited is active.
        # We trash BT2-054 (Gotsumon), NOT P-167. The inherited should NOT fire.
        perm = runner.place_on_field(1, [BT2_054, P_167, EX8_055])
        opp_perm = runner.place_on_field(2, [ST1_03, ST1_11])
        opp_initial_sources = len(opp_perm.card_sources)

        game = runner.game
        player = game.player1

        # Trash the Gotsumon (not P-167)
        gotsumon_cs = perm.card_sources[0]
        assert gotsumon_cs.c_entity_base.card_id == BT2_054

        trashed = perm.trash_specific_digivolution_cards([gotsumon_cs])
        player.trash_cards.extend(trashed)
        runner.auto_resolve()

        # Opponent should NOT have been de-digivolved since P-167 wasn't trashed
        assert len(opp_perm.card_sources) == opp_initial_sources, \
            "Opponent should NOT be de-digivolved when a different card is trashed"

    def test_inherited_does_NOT_trigger_from_non_mineral_rock_digimon(self, debug_runner):
        """Inherited should NOT fire when P-167 is trashed from a Digimon without
        Mineral or Rock trait."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        # P-167 under ST1-11 WarGreymon (no Mineral/Rock trait)
        perm = runner.place_on_field(1, [P_167, ST1_11])
        opp_perm = runner.place_on_field(2, [ST1_03, ST1_11])
        opp_initial_sources = len(opp_perm.card_sources)

        game = runner.game
        player = game.player1

        p167_cs = perm.card_sources[0]
        assert p167_cs.c_entity_base.card_id == P_167

        trashed = perm.trash_specific_digivolution_cards([p167_cs])
        player.trash_cards.extend(trashed)
        runner.auto_resolve()

        assert len(opp_perm.card_sources) == opp_initial_sources, \
            "Opponent should NOT be de-digivolved from non-Mineral/Rock Digimon"

    def test_inherited_condition_checks_trashed_cards_list(self, debug_runner):
        """Directly verify condition checks that card is in trashed_cards."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, [P_167, EX8_055])

        p167_cs = perm.card_sources[0]
        card_obj = p167_cs  # The CardSource for P-167

        effects = card_obj.effect_list(None)
        inh_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnDigivolutionCardDiscarded
            and e.is_inherited_effect
        ]
        assert len(inh_effects) >= 1
        inh = inh_effects[0]

        # Should pass when card IS in trashed_cards and perm has Mineral trait
        ctx_pass = {
            "event_permanent": perm,
            "trashed_cards": [p167_cs],
            "permanent": perm,
            "player": runner.game.player1,
            "game": runner.game,
        }
        assert inh.can_use_condition(ctx_pass), \
            "Condition should pass when card is in trashed_cards"

        # Should FAIL when card is NOT in trashed_cards
        other_cs = perm.card_sources[0]  # could be same or different
        # Create a dummy list that doesn't include p167_cs
        ctx_fail = {
            "event_permanent": perm,
            "trashed_cards": [],  # P-167 not in list
            "permanent": perm,
            "player": runner.game.player1,
            "game": runner.game,
        }
        assert not inh.can_use_condition(ctx_fail), \
            "Condition should fail when card is NOT in trashed_cards"
