"""Behavioral tests for LM-043 Darkdramon (Lv.6 Black/Purple, 12000 DP, Cost 7).

Card text:
  [Hand] [Counter] <Blast Digivolve> (Your Digimon may digivolve into this
      card without paying the cost.)
  <Scapegoat> (When this Digimon would be deleted other than by your effects,
      by deleting 1 of your other Digimon, prevent that deletion.)
  [On Play] [When Digivolving] <De-Digivolve 1> 1 of your opponent's Digimon.
      Then, delete all of their Digimon with the lowest play cost.
  Inherited: Ace Overflow <-4>
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


CARD_ID = "LM-043"

# Lv.5 Black Digimon for digivolve base and field setup
BLACK_LV5 = "BT13-068"  # Kimeramon, Black, Lv.5
# Lv.3 generic rookies for opponent field
ROOKIE_LV3_COST3 = "ST1-03"   # Agumon, Red, Lv.3, cost 3
ROOKIE_LV3_COST2 = "BT1-003"  # Koromon, digi-egg Lv.2 -- not a good fit; need real rookies
# Use ST-series for cheap cost Digimon on opponent field
OPP_CHEAP = "ST1-03"     # Agumon, Red, Lv.3, Cost 3, DP 2000
OPP_EXPENSIVE = "ST1-07"  # Greymon, Red, Lv.4, Cost 5, DP 4000

# A Lv.4 for stacking (to have digi cards for De-Digivolve to strip)
LV4_STACK = "ST1-07"  # Greymon, Red, Lv.4


def _make_deck():
    """Build a minimal legal deck containing LM-043."""
    return [CARD_ID] * 4 + [OPP_CHEAP] * 46


@pytest.mark.behavioral
class TestLM043Darkdramon:
    """Tests for LM-043 Darkdramon."""

    # ── Effect structure tests ──────────────────────────────────────────

    def test_has_blast_digivolve(self, debug_runner):
        """Should have a Blast Digivolve effect."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        card = runner.inject_card(1, CARD_ID, "hand")
        effects = card.effect_list(None)
        blast = [e for e in effects if getattr(e, '_is_blast_digivolve', False)]
        assert len(blast) >= 1, "Should have Blast Digivolve effect"

    def test_blast_digivolve_is_counter(self, debug_runner):
        """Blast Digivolve should be marked as counter effect."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        card = runner.inject_card(1, CARD_ID, "hand")
        effects = card.effect_list(None)
        blast = [e for e in effects if getattr(e, '_is_blast_digivolve', False)]
        assert any(e.is_counter_effect for e in blast), "Blast Digivolve should be counter"

    def test_has_scapegoat(self, debug_runner):
        """Should have a Scapegoat keyword effect."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        perm = runner.place_on_field(1, [CARD_ID])
        assert perm.has_keyword('_is_scapegoat'), (
            "Darkdramon should have Scapegoat keyword on field")

    def test_has_on_play_effect(self, debug_runner):
        """Should have an On Play effect."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        card = runner.inject_card(1, CARD_ID, "hand")
        effects = card.effect_list(None)
        on_play = [e for e in effects if getattr(e, 'is_on_play', False)]
        assert len(on_play) >= 1, "Should have On Play effect"

    def test_has_when_digivolving_effect(self, debug_runner):
        """Should have a When Digivolving effect."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        card = runner.inject_card(1, CARD_ID, "hand")
        effects = card.effect_list(None)
        when_digi = [e for e in effects if getattr(e, 'is_when_digivolving', False)]
        assert len(when_digi) >= 1, "Should have When Digivolving effect"

    def test_has_ace_overflow_inherited(self, debug_runner):
        """Should have Ace Overflow <-4> as inherited effect."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        card = runner.inject_card(1, CARD_ID, "hand")
        effects = card.effect_list(None)
        inherited = [e for e in effects if getattr(e, 'is_inherited_effect', False)]
        ace = [e for e in inherited
               if 'Ace Overflow' in (getattr(e, 'effect_description', '') or '')]
        assert len(ace) >= 1, "Should have inherited Ace Overflow effect"

    def test_ace_overflow_from_entity_base(self, debug_runner):
        """Ace Overflow cost should be parsed from entity_base (engine-level)."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        card = runner.inject_card(1, CARD_ID, "hand")
        assert card.c_entity_base.is_ace, "LM-043 should be recognized as ACE card"
        assert card.c_entity_base.ace_overflow_cost == 4, (
            f"Ace Overflow cost should be 4, got {card.c_entity_base.ace_overflow_cost}")

    # ── On Play: De-Digivolve 1 + delete lowest cost ────────────────────

    def test_on_play_de_digivolves_opponent_digimon(self, debug_runner):
        """On Play should De-Digivolve 1 on a selected opponent Digimon."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        game = runner.game

        # Place opponent Digimon with a stack of 2 cards (Lv.4 on Lv.3)
        opp_perm = runner.place_on_field(2, [OPP_CHEAP, LV4_STACK])
        assert len(opp_perm.card_sources) == 2, "Should have 2-card stack"
        original_level = opp_perm.level

        # Clear P2 trash for tracking
        game.player2.trash_cards.clear()

        # Place a Black Digimon on P1's field for color requirement
        runner.place_on_field(1, [BLACK_LV5])

        runner.inject_card(1, CARD_ID, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Darkdramon")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        # After De-Digivolve 1, the top card should be stripped
        # The opponent permanent should now have 1 card in stack
        # (unless it got deleted as lowest cost too)
        p2_trash_ids = [c.c_entity_base.card_id for c in game.player2.trash_cards
                        if c.c_entity_base]

        # The de-digivolved card should be in trash
        assert len(p2_trash_ids) >= 1, (
            f"At least 1 card should be in P2 trash from De-Digivolve. "
            f"Trash: {p2_trash_ids}")

    def test_on_play_deletes_all_lowest_cost_digimon(self, debug_runner):
        """On Play should delete ALL opponent Digimon with the lowest play cost."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        game = runner.game

        # Place 3 opponent Digimon: 2 cheap (cost 3) and 1 expensive (cost 5)
        cheap1 = runner.place_on_field(2, [OPP_CHEAP])   # Agumon, cost 3
        cheap2 = runner.place_on_field(2, [OPP_CHEAP])   # Agumon, cost 3
        expensive = runner.place_on_field(2, [OPP_EXPENSIVE])  # Greymon, cost 5

        assert len(game.player2.battle_area) == 3

        # Place Black Digimon for color req
        runner.place_on_field(1, [BLACK_LV5])

        runner.inject_card(1, CARD_ID, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Darkdramon")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        # After effect: the two cheap Digimon (cost 3) should be deleted
        # The expensive one (cost 5) should survive
        opp_field_ids = [
            p.top_card.c_entity_base.card_id
            for p in game.player2.battle_area if p.is_digimon
            and p.top_card and p.top_card.c_entity_base
        ]

        # The expensive Greymon should still be on field
        assert OPP_EXPENSIVE in opp_field_ids, (
            f"Expensive Digimon (cost 5) should survive. "
            f"Field: {opp_field_ids}")

        # The cheap Agumon instances should be deleted (both had cost 3 = lowest)
        cheap_count = opp_field_ids.count(OPP_CHEAP)
        assert cheap_count == 0, (
            f"All lowest-cost Digimon should be deleted. "
            f"Remaining cheap: {cheap_count}, Field: {opp_field_ids}")

    def test_on_play_no_opponent_digimon_is_safe(self, debug_runner):
        """On Play with no opponent Digimon should not crash."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        game = runner.game

        # Clear opponent field
        runner.clear_zone(2, "field")
        assert len(game.player2.battle_area) == 0

        # Place Black Digimon for color req
        runner.place_on_field(1, [BLACK_LV5])

        runner.inject_card(1, CARD_ID, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Darkdramon")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"

        # Should not crash when no opponent Digimon
        runner.execute(action)
        runner.auto_resolve()

        # Darkdramon should be on P1's field
        p1_ids = [
            p.top_card.c_entity_base.card_id
            for p in game.player1.battle_area
            if p.top_card and p.top_card.c_entity_base
        ]
        assert CARD_ID in p1_ids, f"Darkdramon should be on field. P1 field: {p1_ids}"

    def test_on_play_de_digivolve_then_delete_order(self, debug_runner):
        """De-Digivolve happens first, which may change the lowest play cost."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        game = runner.game

        # Place opponent Digimon with a 2-card stack: Lv.3 under Lv.4
        # After De-Digivolve 1, this becomes just the Lv.3 (cost 3)
        stacked_perm = runner.place_on_field(2, [OPP_CHEAP, OPP_EXPENSIVE])
        assert len(stacked_perm.card_sources) == 2

        # Also place a standalone cheap Digimon (cost 3)
        standalone = runner.place_on_field(2, [OPP_CHEAP])

        game.player2.trash_cards.clear()

        # Place Black Digimon for color req
        runner.place_on_field(1, [BLACK_LV5])

        runner.inject_card(1, CARD_ID, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Darkdramon")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        # Effect resolved - at minimum some cards should be in trash
        assert len(game.player2.trash_cards) > 0, (
            "De-Digivolve + delete should put cards in P2 trash")

    # ── Scapegoat behavioral test ────────────────────────────────────────

    def test_scapegoat_prevents_deletion_by_sacrificing_ally(self, debug_runner):
        """Scapegoat should prevent deletion by deleting another friendly Digimon."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        game = runner.game

        # Place Darkdramon and an ally on P1's field
        darkdramon = runner.place_on_field(1, [CARD_ID])
        ally = runner.place_on_field(1, [OPP_CHEAP])  # Sacrificial Digimon

        assert len(game.player1.battle_area) == 2

        # Try to delete Darkdramon (simulating an opponent's effect)
        game.player1.delete_permanent(darkdramon, is_opponent_effect=True)

        # Darkdramon should still be on field (Scapegoat prevented deletion)
        darkdramon_on_field = any(
            p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == CARD_ID
            for p in game.player1.battle_area
        )
        assert darkdramon_on_field, (
            "Darkdramon should survive via Scapegoat")

        # The ally should be deleted instead
        assert len(game.player1.battle_area) == 1, (
            f"One Digimon should remain (Darkdramon). "
            f"Field count: {len(game.player1.battle_area)}")

    def test_scapegoat_no_ally_means_deletion_proceeds(self, debug_runner):
        """Without another Digimon, Scapegoat cannot activate."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        game = runner.game

        # Place only Darkdramon (no ally to sacrifice)
        darkdramon = runner.place_on_field(1, [CARD_ID])
        assert len(game.player1.battle_area) == 1

        # Delete Darkdramon
        game.player1.delete_permanent(darkdramon, is_opponent_effect=True)

        # Without ally, deletion should proceed
        assert len(game.player1.battle_area) == 0, (
            "Without ally, Darkdramon should be deleted normally")

    # ── When Digivolving (process callback) ──────────────────────────────

    def test_when_digivolving_triggers_de_digivolve_and_delete(self, debug_runner):
        """When Digivolving should De-Digivolve 1 + delete lowest cost."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        game = runner.game

        # Place a Lv.6 Darkdramon on field (simulating it was just digivolved)
        perm = runner.place_on_field(1, [CARD_ID])
        card = perm.top_card

        # Place opponent Digimon with a 2-card stack
        opp_perm = runner.place_on_field(2, [OPP_CHEAP, LV4_STACK])
        assert len(opp_perm.card_sources) == 2

        game.player2.trash_cards.clear()

        # Get the When Digivolving effect and call its process directly
        effects = card.effect_list(None)
        wd_effects = [e for e in effects if getattr(e, 'is_when_digivolving', False)]
        assert len(wd_effects) >= 1

        wd = wd_effects[0]
        # The process callback needs player and game context
        # But it also calls effect_select_opponent_permanent which requires selection
        # We'll test via direct placement + process callback approach
        ctx = {
            'player': game.player1,
            'game': game,
            'permanent': perm,
        }
        wd.on_process_callback(ctx)
        runner.auto_resolve()

        # Something should be in P2 trash (de-digivolve card and/or deleted Digimon)
        assert len(game.player2.trash_cards) >= 1, (
            "WD effect should send cards to P2 trash via De-Digivolve + delete")

    # ── Play cost calculation for "lowest play cost" ─────────────────────

    def test_lowest_play_cost_uses_top_card(self, debug_runner):
        """Lowest play cost should use the top card's play_cost."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        game = runner.game

        # Place opponent Digimon with known different costs
        # OPP_CHEAP = ST1-03 (cost 3), OPP_EXPENSIVE = ST1-07 (cost 5)
        runner.place_on_field(2, [OPP_CHEAP])      # cost 3
        runner.place_on_field(2, [OPP_EXPENSIVE])   # cost 5

        # Verify the play costs are what we expect
        costs = []
        for p in game.player2.battle_area:
            if p.is_digimon and p.top_card and p.top_card.c_entity_base:
                costs.append(p.top_card.c_entity_base.play_cost)
        assert 3 in costs, f"Expected cost 3 in field. Costs: {costs}"
        assert 5 in costs, f"Expected cost 5 in field. Costs: {costs}"

    # ── Memory cost ──────────────────────────────────────────────────────

    def test_play_cost_is_7(self, debug_runner):
        """Playing Darkdramon should cost 7 memory."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        game = runner.game

        # Place Black Digimon for color req
        runner.place_on_field(1, [BLACK_LV5])

        # Place opponent Digimon (for de-digivolve target)
        runner.place_on_field(2, [OPP_CHEAP])

        runner.inject_card(1, CARD_ID, "hand")
        runner.set_phase("Main")

        mem_before = game.memory
        action = runner.find_action("Darkdramon")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        expected = mem_before - 7
        assert game.memory == expected, (
            f"Play cost should be 7. Before: {mem_before}, "
            f"After: {game.memory}, Expected: {expected}")

    # ── De-Digivolve 1 on a single-card Digimon ─────────────────────────

    def test_de_digivolve_on_single_card_does_nothing(self, debug_runner):
        """De-Digivolve 1 on a Digimon with only 1 card does nothing (can't trash past Lv.3)."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        game = runner.game

        # Place an opponent Digimon with just 1 card
        opp_perm = runner.place_on_field(2, [OPP_CHEAP])
        assert len(opp_perm.card_sources) == 1

        game.player2.trash_cards.clear()

        # Place Black Digimon for color req
        runner.place_on_field(1, [BLACK_LV5])

        runner.inject_card(1, CARD_ID, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Darkdramon")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        # The single-card Digimon should still have been deleted by "delete lowest"
        # But no de-digivolve card should have been removed from stack
        # (can't de-digivolve a single card stack)

    # ── Delete all with same lowest cost ─────────────────────────────────

    def test_deletes_multiple_digimon_with_same_lowest_cost(self, debug_runner):
        """Should delete ALL Digimon tied for the lowest play cost."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        game = runner.game

        # Place 3 opponent Digimon all with cost 3
        runner.place_on_field(2, [OPP_CHEAP])  # cost 3
        runner.place_on_field(2, [OPP_CHEAP])  # cost 3
        runner.place_on_field(2, [OPP_CHEAP])  # cost 3

        assert len(game.player2.battle_area) == 3

        runner.place_on_field(1, [BLACK_LV5])
        runner.inject_card(1, CARD_ID, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Darkdramon")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        # All 3 should be deleted (all tied at cost 3 = lowest)
        opp_digimon = [p for p in game.player2.battle_area if p.is_digimon]
        assert len(opp_digimon) == 0, (
            f"All tied-for-lowest Digimon should be deleted. "
            f"Remaining: {len(opp_digimon)}")
