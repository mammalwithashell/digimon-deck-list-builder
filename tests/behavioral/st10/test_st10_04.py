"""Behavioral tests for ST10-04 Gatomon (Lv.4 Yellow/Purple Digimon, cost 5).

Card text:
  [On Play] Reveal the top 3 cards of your deck. Add 1 yellow Digimon card and
    1 purple Digimon card among them to your hand. Place the remaining cards
    at the bottom of your deck in any order.
  [Your Turn] When this Digimon would digivolve into a card with the
    [Archangel] or [Fallen Angel] trait, reduce the memory cost of the
    digivolution by 2.

  Inherited: [End of Your Turn] This Digimon and any of your other Digimon
    may DNA digivolve into a Digimon card in the hand.

Clause decomposition:
  1. [On Play] Reveal top 3 → pick 1 yellow Digimon → pick 1 purple Digimon
     → remaining to deck bottom.
  2. [Your Turn] CONTINUOUS digivolve cost reduction of -2 when this Gatomon
     would digivolve into a card with the Archangel or Fallen Angel trait.
  3. [Inherited][EoT] DNA digivolve trigger from hand (player choice).

Test cards:
  - ST10-04 Gatomon         (Yellow+Purple Lv.4, card under test)
  - ST10-02 Salamon         (Yellow Lv.3 Digimon)
  - ST10-05 Angewomon       (Yellow Lv.5 Archangel)
  - ST10-07 Ghostmon        (Purple Lv.3 Ghost Digimon)
  - ST10-06 Mastemon        (Yellow+Purple Lv.6 DNA target)
  - BT3-093 Davis Motomiya  (Yellow Tamer - negative for digimon filter)
  - ST1-03 Agumon           (Red Lv.3, negative / non-yellow/purple)
  - BT11-083 LadyDevimon    (Purple Lv.5 Fallen Angel)
"""

import pytest

from digimon_gym.engine.data.enums import EffectTiming


# === Card IDs ===
GATOMON = "ST10-04"                 # Card under test
SALAMON = "ST10-02"                 # Yellow Lv.3 Digimon
ANGEWOMON = "ST10-05"               # Yellow Lv.5 Archangel Digimon (digivolves from Gatomon)
GHOSTMON = "ST10-07"                # Purple Lv.3 Ghost Digimon
MASTEMON = "ST10-06"                # Yellow+Purple Lv.6 DNA target
AGUMON = "ST1-03"                   # Red Lv.3 Digimon (negative)
DAVIS_TAMER = "BT3-093"             # Yellow Tamer (negative for digimon filter)
LADYDEVIMON = "BT11-083"            # Purple Lv.5 Fallen Angel Digimon


def _build_deck(extra=None):
    """Build a 50-card minimal deck padded with Ghostmon filler."""
    deck = []
    if extra:
        deck.extend(extra)
    while len(deck) < 50:
        deck.append(GHOSTMON)
    return deck


@pytest.mark.behavioral
class TestST10_04_OnPlayReveal:
    """Clause 1: [On Play] Reveal top 3, add 1 yellow Digimon + 1 purple Digimon."""

    def test_on_play_effect_exists(self, debug_runner):
        """Should have exactly one On Play effect with the proper timing flag."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [GATOMON])
        effects = perm.top_card.effect_list(None)

        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        assert len(on_play) == 1, (
            f"Should have exactly 1 On Play effect, got {len(on_play)}"
        )

    def test_on_play_adds_yellow_and_purple_digimon_to_hand(self, debug_runner):
        """Top 3 of deck contains a Yellow and a Purple Digimon → both land in hand."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        runner.clear_zone(1, "hand")
        # Stack injection is top-biased: later inject = deeper pop from top.
        # Final library top order (top → bottom): SALAMON, GHOSTMON, AGUMON
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, GHOSTMON, "library_top")
        runner.inject_card(1, SALAMON, "library_top")

        # Place Gatomon on the field
        perm = runner.place_on_field(1, [GATOMON])

        effects = perm.top_card.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        on_play.on_process_callback({'player': player, 'game': game})
        runner.auto_resolve()

        hand_ids = [c.card_id for c in player.hand_cards]
        assert SALAMON in hand_ids, (
            f"Yellow Digimon (Salamon) should be added to hand. Hand: {hand_ids}"
        )
        assert GHOSTMON in hand_ids, (
            f"Purple Digimon (Ghostmon) should be added to hand. Hand: {hand_ids}"
        )
        # Red Agumon must go to deck bottom (not hand)
        assert AGUMON not in hand_ids, (
            f"Red Agumon should NOT be added to hand. Hand: {hand_ids}"
        )

    def test_on_play_places_remaining_at_deck_bottom(self, debug_runner):
        """Non-selected revealed cards should go to deck bottom."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        runner.clear_zone(1, "hand")

        # Clear library then inject controlled contents
        player.library_cards.clear()
        # Library base (will stay untouched - used to verify bottom placement)
        for _ in range(5):
            runner.inject_card(1, GHOSTMON, "library_bottom")

        # Top of library: Salamon (yellow), Ghostmon (purple), Agumon (red)
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, GHOSTMON, "library_top")
        runner.inject_card(1, SALAMON, "library_top")

        lib_size_before = len(player.library_cards)

        perm = runner.place_on_field(1, [GATOMON])
        effects = perm.top_card.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        on_play.on_process_callback({'player': player, 'game': game})
        runner.auto_resolve()

        # Two selected (Salamon + Ghostmon) went to hand, 1 (Agumon) to deck bottom
        assert len(player.library_cards) == lib_size_before - 2, (
            f"Library should lose 2 cards (2 selected to hand). "
            f"Before: {lib_size_before}, After: {len(player.library_cards)}"
        )
        # Bottom card of deck should be Agumon (the red one not picked)
        assert player.library_cards[-1].card_id == AGUMON, (
            f"Bottom of deck should be Agumon (unpicked red card). "
            f"Got: {player.library_cards[-1].card_id}"
        )

    def test_on_play_does_not_add_non_digimon(self, debug_runner):
        """Tamer/Option cards in revealed pool should NOT be added to hand."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        runner.clear_zone(1, "hand")
        player.library_cards.clear()
        for _ in range(5):
            runner.inject_card(1, AGUMON, "library_bottom")

        # Top 3: Yellow Tamer, Purple Digimon, Yellow Digimon
        runner.inject_card(1, SALAMON, "library_top")         # yellow digimon
        runner.inject_card(1, GHOSTMON, "library_top")        # purple digimon
        runner.inject_card(1, DAVIS_TAMER, "library_top")     # yellow TAMER (skip)

        perm = runner.place_on_field(1, [GATOMON])
        effects = perm.top_card.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        on_play.on_process_callback({'player': player, 'game': game})
        runner.auto_resolve()

        hand_ids = [c.card_id for c in player.hand_cards]
        # Davis Tamer is yellow but NOT a Digimon - must not be picked
        assert DAVIS_TAMER not in hand_ids, (
            f"Yellow Tamer should NOT be picked (requires Digimon). Hand: {hand_ids}"
        )
        # Salamon (yellow digimon) and Ghostmon (purple digimon) should be added
        assert SALAMON in hand_ids, (
            f"Salamon (yellow Digimon) should be added. Hand: {hand_ids}"
        )
        assert GHOSTMON in hand_ids, (
            f"Ghostmon (purple Digimon) should be added. Hand: {hand_ids}"
        )

    def test_on_play_only_purple_available(self, debug_runner):
        """If no yellow Digimon in revealed cards, still add the purple one."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        runner.clear_zone(1, "hand")
        player.library_cards.clear()
        for _ in range(5):
            runner.inject_card(1, AGUMON, "library_bottom")

        # Top 3: Agumon, Agumon, Ghostmon (only purple is reachable, no yellow)
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, GHOSTMON, "library_top")

        perm = runner.place_on_field(1, [GATOMON])
        effects = perm.top_card.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        on_play.on_process_callback({'player': player, 'game': game})
        runner.auto_resolve()

        hand_ids = [c.card_id for c in player.hand_cards]
        assert GHOSTMON in hand_ids, (
            f"Purple Digimon should be added even if no yellow available. "
            f"Hand: {hand_ids}"
        )

    def test_on_play_uses_engine_reveal_helper(self, debug_runner):
        """Process should use effect_reveal_and_select_multi (not raw library ops)."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        runner.clear_zone(1, "hand")
        # Stack a reveal pool
        runner.inject_card(1, SALAMON, "library_top")
        runner.inject_card(1, GHOSTMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")

        # Monkey-patch effect_reveal_and_select_multi to verify it is called
        call_record = {'called': False, 'count': None, 'pass_count': None}

        original_fn = game.effect_reveal_and_select_multi

        def tracked(*args, **kwargs):
            call_record['called'] = True
            # count positional arg: (player, count, passes, ...)
            if len(args) >= 2:
                call_record['count'] = args[1]
            else:
                call_record['count'] = kwargs.get('count')
            passes = None
            if len(args) >= 3:
                passes = args[2]
            else:
                passes = kwargs.get('passes')
            if passes is not None:
                call_record['pass_count'] = len(passes)
            return original_fn(*args, **kwargs)

        game.effect_reveal_and_select_multi = tracked

        perm = runner.place_on_field(1, [GATOMON])
        effects = perm.top_card.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]
        on_play.on_process_callback({'player': player, 'game': game})
        runner.auto_resolve()

        assert call_record['called'], (
            "On Play should use game.effect_reveal_and_select_multi helper"
        )
        assert call_record['count'] == 3, (
            f"Reveal count should be 3, got {call_record['count']}"
        )
        assert call_record['pass_count'] == 2, (
            f"Should have 2 selection passes (yellow + purple), "
            f"got {call_record['pass_count']}"
        )


@pytest.mark.behavioral
class TestST10_04_DigivolveCostReduction:
    """Clause 2: [Your Turn] Reduce digivolve cost by 2 for Archangel/Fallen Angel."""

    def _get_cost_effect(self, perm):
        """Return the cost reduction effect from Gatomon's effect list."""
        effects = perm.top_card.effect_list(None)
        matching = [
            e for e in effects
            if getattr(e, 'cost_reduction', 0) > 0
            and not getattr(e, 'is_inherited_effect', False)
        ]
        assert len(matching) == 1, (
            f"Should have exactly 1 non-inherited cost_reduction effect. "
            f"Got {len(matching)}"
        )
        return matching[0]

    def test_cost_reduction_effect_exists(self, debug_runner):
        """ST10-04 should expose a cost_reduction=2 effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [GATOMON])
        eff = self._get_cost_effect(perm)
        assert eff.cost_reduction == 2, (
            f"cost_reduction should be 2, got {eff.cost_reduction}"
        )

    def test_cost_reduction_timing_is_when_would_digivolve(self, debug_runner):
        """Digivolve cost reduction must use WhenWouldDigivolve timing."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [GATOMON])
        eff = self._get_cost_effect(perm)
        assert eff.timing == EffectTiming.WhenWouldDigivolve, (
            f"Cost reduction timing should be WhenWouldDigivolve, got {eff.timing}"
        )

    def test_cost_reduction_applies_to_archangel(self, debug_runner):
        """Condition should PASS when digivolving into an Archangel card."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        perm = runner.place_on_field(1, [GATOMON])
        eff = self._get_cost_effect(perm)

        # Create an Angewomon (Archangel) card as the digivolve target
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        angewomon_cs = db.create_card_source(ANGEWOMON, player)

        ctx = {"player": player, "permanent": perm, "card_source": angewomon_cs}
        assert eff.can_use_condition(ctx), (
            "Cost reduction should apply when digivolving into an Archangel "
            f"(Angewomon traits: {angewomon_cs.card_traits})"
        )

    def test_cost_reduction_applies_to_fallen_angel(self, debug_runner):
        """Condition should PASS when digivolving into a Fallen Angel card."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        perm = runner.place_on_field(1, [GATOMON])
        eff = self._get_cost_effect(perm)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        lady_cs = db.create_card_source(LADYDEVIMON, player)

        ctx = {"player": player, "permanent": perm, "card_source": lady_cs}
        assert eff.can_use_condition(ctx), (
            "Cost reduction should apply when digivolving into a Fallen Angel "
            f"(LadyDevimon traits: {lady_cs.card_traits})"
        )

    def test_cost_reduction_does_not_apply_to_other_traits(self, debug_runner):
        """Condition should FAIL when target has neither Archangel nor Fallen Angel."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        perm = runner.place_on_field(1, [GATOMON])
        eff = self._get_cost_effect(perm)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        # Mastemon has trait "Angel" - NOT "Archangel" or "Fallen Angel"
        mastemon_cs = db.create_card_source(MASTEMON, player)

        ctx = {"player": player, "permanent": perm, "card_source": mastemon_cs}
        assert not eff.can_use_condition(ctx), (
            f"Cost reduction should NOT apply for non-Archangel/Fallen Angel "
            f"(Mastemon traits: {mastemon_cs.card_traits})"
        )

    def test_cost_reduction_fails_on_opponent_turn(self, debug_runner):
        """[Your Turn] gating - condition must fail on opponent's turn."""
        runner = debug_runner(initial_memory=5, first_player=2)
        game = runner.game
        player = game.player1

        perm = runner.place_on_field(1, [GATOMON])
        eff = self._get_cost_effect(perm)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        angewomon_cs = db.create_card_source(ANGEWOMON, player)

        ctx = {"player": player, "permanent": perm, "card_source": angewomon_cs}
        assert not player.is_my_turn, "Sanity: P1 should not be turn player"
        assert not eff.can_use_condition(ctx), (
            "Cost reduction should NOT fire on opponent's turn ([Your Turn])"
        )

    def test_cost_reduction_fails_when_not_on_field(self, debug_runner):
        """Condition must fail if Gatomon is not on the battle area."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        runner.inject_card(1, GATOMON, "hand")
        gatomon_cs = [
            c for c in player.hand_cards
            if c.card_id == GATOMON
        ][0]

        effects = gatomon_cs.effect_list(None)
        cost_effects = [
            e for e in effects
            if getattr(e, 'cost_reduction', 0) > 0
            and not getattr(e, 'is_inherited_effect', False)
        ]
        assert len(cost_effects) == 1
        eff = cost_effects[0]

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        angewomon_cs = db.create_card_source(ANGEWOMON, player)

        ctx = {"player": player, "permanent": None, "card_source": angewomon_cs}
        assert not eff.can_use_condition(ctx), (
            "Cost reduction must fail when Gatomon is in hand (not on field)"
        )

    def test_cost_reduction_integration_via_digivolve(self, debug_runner):
        """End-to-end: digivolving Gatomon into Angewomon costs (3 - 2) = 1 memory."""
        runner = debug_runner(initial_memory=10)
        game = runner.game
        player = game.player1

        # Place Gatomon on field. Angewomon (Yellow Lv.5) evo cost from Yellow Lv.4 = 3 memory.
        perm = runner.place_on_field(1, [GATOMON])

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        angewomon_cs = db.create_card_source(ANGEWOMON, player)

        # Call player.digivolve() which applies cost reduction
        final_cost = player.digivolve(perm, angewomon_cs)

        # Expected: base 3 - 2 (Gatomon's cost reduction) = 1
        assert final_cost == 1, (
            f"Digivolve cost should be 3 - 2 = 1. Got {final_cost}"
        )


@pytest.mark.behavioral
class TestST10_04_InheritedDNADigivolve:
    """Inherited: [End of Your Turn] DNA digivolve from hand."""

    def test_inherited_dna_effect_exists(self, debug_runner):
        """Inherited OnEndTurn DNA digivolve effect should exist."""
        runner = debug_runner(initial_memory=5)
        # Stack Gatomon under a higher-form Digimon (Angewomon digivolves on Gatomon)
        perm = runner.place_on_field(1, [GATOMON, ANGEWOMON])

        gatomon_source = perm.card_sources[0]
        assert gatomon_source.card_id == GATOMON, (
            f"Bottom of stack should be Gatomon. Got: {gatomon_source.card_id}"
        )

        effects = gatomon_source.effect_list(None)
        eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn
            and e.is_inherited_effect
        ]
        assert len(eot) == 1, (
            f"Should have exactly 1 inherited OnEndTurn effect. Got {len(eot)}"
        )
        assert eot[0].is_optional, (
            "DNA digivolve at end of turn must be optional"
        )

    def test_inherited_dna_condition_requires_owner_turn(self, debug_runner):
        """Condition must fail on opponent's turn ([Your Turn])."""
        runner = debug_runner(initial_memory=5, first_player=2)
        perm = runner.place_on_field(1, [GATOMON, ANGEWOMON])

        gatomon_source = perm.card_sources[0]
        effects = gatomon_source.effect_list(None)
        eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn
            and e.is_inherited_effect
        ][0]

        assert not runner.game.player1.is_my_turn, "Sanity: not P1 turn"
        assert not eot.can_use_condition({}), (
            "Inherited DNA digivolve should NOT activate on opponent's turn"
        )

    def test_inherited_dna_condition_fails_when_not_on_field(self, debug_runner):
        """Condition must fail if source card is not on field."""
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, GATOMON, "hand")

        gatomon = [
            c for c in runner.game.player1.hand_cards
            if c.card_id == GATOMON
        ][0]

        effects = gatomon.effect_list(None)
        eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn
            and e.is_inherited_effect
        ]
        if eot:
            assert not eot[0].can_use_condition({}), (
                "Condition should fail when card is in hand"
            )

    def test_inherited_dna_process_calls_helper(self, debug_runner):
        """Process callback should invoke game.effect_dna_digivolve_from_hand."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        # Stack Gatomon under Angewomon, plus another Yellow Lv.5 for DNA material
        perm = runner.place_on_field(1, [GATOMON, ANGEWOMON])
        runner.place_on_field(1, [LADYDEVIMON])  # Purple Lv.5

        # Put Mastemon in hand (Yellow Lv.5 + Purple Lv.5 DNA target)
        runner.clear_zone(1, "hand")
        runner.inject_card(1, MASTEMON, "hand")

        gatomon_source = perm.card_sources[0]
        effects = gatomon_source.effect_list(None)
        eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn
            and e.is_inherited_effect
        ][0]

        call_record = {'called': False}
        original_fn = game.effect_dna_digivolve_from_hand

        def tracked(*args, **kwargs):
            call_record['called'] = True
            return original_fn(*args, **kwargs)

        game.effect_dna_digivolve_from_hand = tracked

        eot.on_process_callback({'player': player, 'game': game})

        assert call_record['called'], (
            "Inherited EoT process should invoke effect_dna_digivolve_from_hand"
        )


@pytest.mark.behavioral
class TestST10_04_ScriptImportability:
    """Sanity: script imports cleanly and has all 3 declared effects."""

    def test_script_loads(self):
        """Script module imports without errors and exposes class ST10_04."""
        from digimon_gym.engine.data.scripts.st10.st10_04 import ST10_04
        assert ST10_04 is not None

    def test_script_has_three_top_level_effects(self, debug_runner):
        """Gatomon should expose 3 distinct effects on its top-card view."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [GATOMON])
        effects = perm.top_card.effect_list(None)

        # Non-inherited effects visible on the top card
        non_inherited = [e for e in effects if not getattr(e, 'is_inherited_effect', False)]
        assert len(non_inherited) >= 2, (
            f"Should expose On Play + cost reduction at minimum. Got {len(non_inherited)}"
        )

        on_play = [e for e in non_inherited if getattr(e, 'is_on_play', False)]
        cost_red = [e for e in non_inherited if getattr(e, 'cost_reduction', 0) > 0]
        assert len(on_play) == 1, "Expected 1 on-play effect"
        assert len(cost_red) == 1, "Expected 1 cost_reduction effect"
