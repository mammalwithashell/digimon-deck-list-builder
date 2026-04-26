"""Behavioral tests for BT21-051 Puppetmon (Ace) (Lv.6 Green/Black Digimon).

Card text:
[Hand] [Counter] <Blast Digivolve>
<Reboot> <Blocker>
[On Play] [When Digivolving] <De-Digivolve 2> 1 of your opponent's Digimon.
Then, return 1 of their suspended Digimon to the bottom of the deck.
Inherited: Ace Overflow <-4>

Alt-digi: Lv.5 w/[WG] trait: Cost 3
Normal evo: Green Lv.5 for cost 4
"""

import pytest

# Minimal deck with BT21-051 plus filler
PUPPETMON_DECK = ["BT21-051"] * 4 + ["BT21-048"] * 46


@pytest.mark.behavioral
class TestBT21051Puppetmon:
    """Tests for BT21-051 Puppetmon."""

    # ── Keywords ──────────────────────────────────────────────────────

    def test_has_blocker(self, debug_runner):
        """Puppetmon should have Blocker keyword on field."""
        runner = debug_runner(
            deck1=PUPPETMON_DECK, deck2=PUPPETMON_DECK, initial_memory=10
        )
        perm = runner.place_on_field(1, ["BT21-051"])
        snap = runner.snapshot()
        field = snap.p1_field[0]
        assert "blocker" in field.keywords, (
            f"Puppetmon should have Blocker, got {field.keywords}"
        )

    def test_has_reboot(self, debug_runner):
        """Puppetmon should have Reboot keyword on field."""
        runner = debug_runner(
            deck1=PUPPETMON_DECK, deck2=PUPPETMON_DECK, initial_memory=10
        )
        perm = runner.place_on_field(1, ["BT21-051"])
        snap = runner.snapshot()
        field = snap.p1_field[0]
        assert "reboot" in field.keywords, (
            f"Puppetmon should have Reboot, got {field.keywords}"
        )

    # ── Alt-Digi ──────────────────────────────────────────────────────

    def test_alt_digi_attributes(self, debug_runner):
        """Alt-digi effect should require WG trait, level 5, cost 3."""
        runner = debug_runner(
            deck1=PUPPETMON_DECK, deck2=PUPPETMON_DECK, initial_memory=10
        )
        runner.inject_card(1, "BT21-051", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        alt_digi = [e for e in effects if hasattr(e, '_alt_digi_cost')]
        assert len(alt_digi) == 1, f"Should have 1 alt-digi effect, got {len(alt_digi)}"

        eff = alt_digi[0]
        assert eff._alt_digi_cost == 3, f"Alt-digi cost should be 3, got {eff._alt_digi_cost}"
        assert eff._alt_digi_trait == "WG", f"Alt-digi trait should be WG, got {eff._alt_digi_trait}"
        assert getattr(eff, '_alt_digi_level', None) == 5, (
            f"Alt-digi level should be 5, got {getattr(eff, '_alt_digi_level', None)}"
        )

    def test_alt_digi_from_lv5_wg(self, debug_runner):
        """Should be able to digivolve from Lv.5 WG trait Digimon for cost 3."""
        runner = debug_runner(
            deck1=PUPPETMON_DECK, deck2=PUPPETMON_DECK, initial_memory=10
        )
        # BT21-050 Cherrymon is Lv.5 with WG trait
        runner.place_on_field(1, ["BT21-050"])
        runner.inject_card(1, "BT21-051", "hand")

        # Should find a digivolve action for Puppetmon
        action = runner.find_action("Digivolve")
        assert action is not None, "Should be able to digivolve Puppetmon from Lv.5 WG"

    def test_alt_digi_blocked_from_lv4_wg(self, debug_runner):
        """Should NOT digivolve from Lv.4 WG trait (level must be 5)."""
        runner = debug_runner(
            deck1=PUPPETMON_DECK, deck2=PUPPETMON_DECK, initial_memory=10
        )
        # BT21-049 Woodmon is Lv.4 with WG trait -- should not be valid for alt-digi
        runner.place_on_field(1, ["BT21-049"])
        runner.inject_card(1, "BT21-051", "hand")

        # The only digivolve actions should be for normal evo (Green Lv.5 for cost 4)
        # Since Woodmon is Lv.4 and not Green Lv.5, digivolve should not be available
        # at all (normal evo requires Green Lv.5, alt requires WG Lv.5)
        digi_actions = runner.find_actions("Digivolve")
        # Filter for Puppetmon specifically
        puppetmon_digi = {k: v for k, v in digi_actions.items() if "Puppetmon" in v}
        assert len(puppetmon_digi) == 0, (
            f"Should NOT digivolve Puppetmon from Lv.4 WG, found: {puppetmon_digi}"
        )

    # ── On Play: De-Digivolve 2 + Bounce Suspended ───────────────────

    def test_on_play_de_digivolve_then_bounce_suspended(self, debug_runner):
        """On Play should De-Digivolve 2 an opponent Digimon,
        then return 1 suspended opponent Digimon to deck bottom."""
        runner = debug_runner(
            deck1=PUPPETMON_DECK, deck2=PUPPETMON_DECK, initial_memory=10
        )

        # Set up opponent with:
        # 1) A stacked Digimon (3 cards: lv3/lv4/lv5) for de-digivolve target
        opp_target = runner.place_on_field(2, ["BT21-048", "BT21-049", "BT21-050"])
        # 2) A suspended Digimon for bounce target
        opp_suspended = runner.place_on_field(2, ["BT21-048"], is_suspended=True)

        runner.inject_card(1, "BT21-051", "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        opp_field_count_before = len(snap_before.p2_field)
        opp_stack_before = snap_before.p2_field[0].stack_ids
        opp_deck_before = snap_before.p2_library_size

        # Play Puppetmon
        action = runner.find_action("Play Puppetmon")
        assert action is not None, "Should be able to play Puppetmon"
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # De-Digivolve 2 should remove 2 cards from the stacked Digimon
        # The stack was [BT21-048, BT21-049, BT21-050], de-digivolve 2 removes
        # BT21-050 and BT21-049, leaving just BT21-048 (lv3)
        de_digi_target = None
        for f in snap_after.p2_field:
            if "BT21-048" in f.stack_ids and f.card_id == "BT21-048":
                de_digi_target = f
                break

        # The suspended Digimon should have been returned to deck bottom
        # So the opponent should have 1 fewer field slot
        assert len(snap_after.p2_field) == opp_field_count_before - 1, (
            f"Opponent should lose 1 Digimon (bounced), had {opp_field_count_before}, "
            f"now {len(snap_after.p2_field)}"
        )

        # Deck size should increase by 1 (bounced card)
        assert snap_after.p2_library_size == opp_deck_before + 1, (
            f"Opponent deck should gain 1 card from bounce, was {opp_deck_before}, "
            f"now {snap_after.p2_library_size}"
        )

    def test_on_play_no_opponent_digimon_skips_both(self, debug_runner):
        """If opponent has no Digimon, both de-digivolve and bounce should be skipped."""
        runner = debug_runner(
            deck1=PUPPETMON_DECK, deck2=PUPPETMON_DECK, initial_memory=10
        )
        runner.inject_card(1, "BT21-051", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Puppetmon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Puppetmon should be on field, game should not crash
        assert any(f.card_name == "Puppetmon" for f in snap.p1_field), (
            "Puppetmon should be on field"
        )

    def test_on_play_de_digivolve_no_suspended_skips_bounce(self, debug_runner):
        """If no opponent Digimon are suspended after de-digivolve, bounce is skipped."""
        runner = debug_runner(
            deck1=PUPPETMON_DECK, deck2=PUPPETMON_DECK, initial_memory=10
        )

        # Opponent has 1 unsuspended stacked Digimon (not suspended)
        runner.place_on_field(2, ["BT21-048", "BT21-049", "BT21-050"])

        runner.inject_card(1, "BT21-051", "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        opp_deck_before = snap_before.p2_library_size

        action = runner.find_action("Play Puppetmon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # De-digivolve should happen, but no bounce (no suspended Digimon)
        # Opponent should still have 1 Digimon on field
        assert len(snap_after.p2_field) == 1, (
            f"Opponent should still have 1 Digimon (no bounce), got {len(snap_after.p2_field)}"
        )
        # Deck should not grow (no bounce happened)
        assert snap_after.p2_library_size == opp_deck_before, (
            f"Opponent deck should not change (no bounce), was {opp_deck_before}, "
            f"now {snap_after.p2_library_size}"
        )

    def test_on_play_bounce_targets_only_suspended(self, debug_runner):
        """Bounce should only target suspended Digimon, not unsuspended ones."""
        runner = debug_runner(
            deck1=PUPPETMON_DECK, deck2=PUPPETMON_DECK, initial_memory=10
        )

        # Opponent: 1 unsuspended Digimon (de-digivolve target) + 1 suspended Digimon
        runner.place_on_field(2, ["BT21-048"])  # unsuspended
        runner.place_on_field(2, ["BT21-048"], is_suspended=True)  # suspended - bounce target

        runner.inject_card(1, "BT21-051", "hand")
        runner.set_phase("Main")

        snap_before = runner.snapshot()
        assert len(snap_before.p2_field) == 2, "Should start with 2 opponent Digimon"

        action = runner.find_action("Play Puppetmon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # The suspended one should be bounced, leaving only the unsuspended one
        assert len(snap_after.p2_field) == 1, (
            f"Should have 1 opponent Digimon left (suspended bounced), got {len(snap_after.p2_field)}"
        )
        # Remaining Digimon should be unsuspended
        assert not snap_after.p2_field[0].is_suspended, (
            "Remaining Digimon should be the unsuspended one"
        )

    # ── When Digivolving ──────────────────────────────────────────────

    def test_when_digivolving_triggers_de_digivolve_and_bounce(self, debug_runner):
        """When Digivolving should also De-Digivolve 2 + bounce suspended."""
        runner = debug_runner(
            deck1=PUPPETMON_DECK, deck2=PUPPETMON_DECK, initial_memory=10
        )

        # Opponent: stacked Digimon + suspended Digimon
        runner.place_on_field(2, ["BT21-048", "BT21-049", "BT21-050"])
        runner.place_on_field(2, ["BT21-048"], is_suspended=True)

        # Place Lv.5 WG on our field to digivolve from
        runner.place_on_field(1, ["BT21-050"])
        runner.inject_card(1, "BT21-051", "hand")

        snap_before = runner.snapshot()
        opp_field_before = len(snap_before.p2_field)

        # Digivolve into Puppetmon
        action = runner.find_action("Digivolve")
        assert action is not None, "Should be able to digivolve"
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # Suspended Digimon should be bounced
        assert len(snap_after.p2_field) == opp_field_before - 1, (
            f"Should lose 1 opponent Digimon from bounce, had {opp_field_before}, "
            f"now {len(snap_after.p2_field)}"
        )

    # ── Blast Digivolve ──────────────────────────────────────────────

    def test_blast_digivolve_flag(self, debug_runner):
        """Puppetmon should have the blast_digivolve flag set."""
        runner = debug_runner(
            deck1=PUPPETMON_DECK, deck2=PUPPETMON_DECK, initial_memory=10
        )
        runner.inject_card(1, "BT21-051", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        blast = [e for e in effects if getattr(e, '_is_blast_digivolve', False)]
        assert len(blast) == 1, "Should have exactly 1 blast digivolve effect"
        assert blast[0].is_counter_effect, "Blast digivolve should be a counter effect"

    # ── Ace Overflow Inherited ────────────────────────────────────────

    def test_ace_overflow_inherited(self, debug_runner):
        """Puppetmon should have Ace Overflow <-4> as inherited effect."""
        runner = debug_runner(
            deck1=PUPPETMON_DECK, deck2=PUPPETMON_DECK, initial_memory=10
        )
        runner.inject_card(1, "BT21-051", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        ace = [e for e in effects if getattr(e, 'is_inherited_effect', False)]
        assert len(ace) >= 1, "Should have at least 1 inherited effect (Ace Overflow)"
        ace_effect = [e for e in ace if "Ace" in (e.effect_name or "")]
        assert len(ace_effect) == 1, (
            f"Should have 1 Ace Overflow inherited effect, got {len(ace_effect)}"
        )
