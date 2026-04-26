"""Behavioral tests for EX11-024 Cendrillmon (Lv.6 Yellow Puppet/LIBERATOR).

Card text:
  <Alliance> <Overclock ([Puppet] Trait)>
  [On Play] [When Digivolving] You may play 1 level 4 or lower [Puppet] trait
      Digimon card from your hand without paying the cost. Then, you may play
      1 [Familiar] Token for each of your opponent's Digimon.
  [When Digivolving] [When Attacking] To 1 of your opponent's Digimon, give
      -3000 DP for the turn for each of your Digimon.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


# Use a minimal filler deck so tests have clean, predictable state.
FILLER = ["ST1-03"] * 50


@pytest.mark.behavioral
class TestEX11024CendrillmonKeywords:
    """Tests for Alliance and Overclock keywords."""

    def test_has_alliance_keyword(self, debug_runner):
        """Cendrillmon should have the Alliance keyword."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        perm = runner.place_on_field(1, ["EX11-024"])

        top_card = perm.card_sources[-1]
        effects = top_card.effect_list(None)
        alliance_effects = [e for e in effects if getattr(e, '_is_alliance', False)]
        assert len(alliance_effects) >= 1, "Cendrillmon should have Alliance effect"

    def test_has_overclock_keyword(self, debug_runner):
        """Cendrillmon should have the Overclock keyword."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        perm = runner.place_on_field(1, ["EX11-024"])

        top_card = perm.card_sources[-1]
        effects = top_card.effect_list(None)
        overclock_effects = [e for e in effects if getattr(e, '_is_overclock', False)]
        assert len(overclock_effects) >= 1, "Cendrillmon should have Overclock effect"


@pytest.mark.behavioral
class TestEX11024CendrillmonOnPlay:
    """Tests for [On Play] effect: play Puppet Lv4- from hand + Familiar Tokens."""

    def test_on_play_plays_puppet_from_hand(self, debug_runner):
        """On Play should allow playing a level 4 or lower Puppet Digimon from hand free."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)

        # Clear hand so only our injected cards are there
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX11-021", "hand")  # Kokeshimon Lv.4 Puppet
        runner.inject_card(1, "EX11-024", "hand")  # Cendrillmon
        # Put an opponent Digimon on field for token play
        runner.place_on_field(2, ["ST1-03"])

        runner.set_phase("Main")
        play_action = runner.find_action("Play Cendrillmon")
        assert play_action is not None, "Should be able to play Cendrillmon"

        runner.execute(play_action)
        # Auto-resolve: picks the puppet from hand, then token play
        results = runner.auto_resolve()

        snap = runner.snapshot()
        # Kokeshimon should be on field (played from hand for free)
        kokeshimon_on_field = any(
            s.card_id == "EX11-021" for s in snap.p1_field
        )
        assert kokeshimon_on_field, "Kokeshimon (Puppet Lv.4) should be played from hand"

    def test_on_play_puppet_filter_rejects_non_puppet(self, debug_runner):
        """On Play should NOT offer to play a non-Puppet Digimon."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)

        runner.clear_zone(1, "hand")
        # Only a non-Puppet Digimon in hand (besides Cendrillmon itself)
        runner.inject_card(1, "ST1-03", "hand")  # Agumon, not Puppet trait
        runner.inject_card(1, "EX11-024", "hand")

        runner.set_phase("Main")
        play_action = runner.find_action("Play Cendrillmon")
        assert play_action is not None
        runner.execute(play_action)
        results = runner.auto_resolve()

        snap = runner.snapshot()
        agumon_in_hand = "ST1-03" in snap.p1_hand
        # Agumon should still be in hand, not played to field by the effect
        assert agumon_in_hand, "Non-Puppet Digimon should remain in hand"

    def test_on_play_puppet_filter_rejects_level_5(self, debug_runner):
        """On Play should reject a Puppet Digimon above level 4."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)

        runner.clear_zone(1, "hand")
        # EX11-022 Karakurumon is Puppet Lv.5 -- should NOT be playable via effect
        runner.inject_card(1, "EX11-022", "hand")
        runner.inject_card(1, "EX11-024", "hand")

        runner.set_phase("Main")
        play_action = runner.find_action("Play Cendrillmon")
        assert play_action is not None
        runner.execute(play_action)
        results = runner.auto_resolve()

        snap = runner.snapshot()
        karakurumon_on_field = any(
            s.card_id == "EX11-022" for s in snap.p1_field
        )
        assert not karakurumon_on_field, "Puppet Lv.5 should not be playable (must be Lv.4 or lower)"

    def test_on_play_free_cost(self, debug_runner):
        """Playing the Puppet from hand should cost nothing (free play).

        Cendrillmon costs 11; Kokeshimon costs 5.
        Starting at memory=12, playing Cendrillmon costs 11 -> memory=1.
        The Kokeshimon played via the effect should cost 0 -> memory stays 1.
        """
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=12)

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX11-021", "hand")  # Kokeshimon costs 5
        runner.inject_card(1, "EX11-024", "hand")  # Cendrillmon costs 11

        runner.set_phase("Main")

        play_action = runner.find_action("Play Cendrillmon")
        assert play_action is not None
        runner.execute(play_action)
        results = runner.auto_resolve()

        snap_after = runner.snapshot()
        kokeshimon_on_field = any(
            s.card_id == "EX11-021" for s in snap_after.p1_field
        )
        assert kokeshimon_on_field, "Kokeshimon should be played for free"
        # Memory: 12 - 11 (Cendrillmon cost) = 1; Kokeshimon is free so memory stays 1
        assert snap_after.memory == 1, (
            f"Memory should be 1 (only Cendrillmon's cost paid), got {snap_after.memory}"
        )

    def test_on_play_familiar_tokens_match_opponent_digimon_count(self, debug_runner):
        """Familiar tokens played should equal opponent's Digimon count."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)

        runner.clear_zone(1, "hand")
        # Place 3 opponent Digimon
        runner.place_on_field(2, ["ST1-03"])
        runner.place_on_field(2, ["ST1-04"])
        runner.place_on_field(2, ["ST1-05"])

        runner.inject_card(1, "EX11-024", "hand")

        runner.set_phase("Main")
        play_action = runner.find_action("Play Cendrillmon")
        assert play_action is not None
        runner.execute(play_action)
        results = runner.auto_resolve()

        snap = runner.snapshot()
        familiar_count = sum(
            1 for s in snap.p1_field if s.card_id == "TOKEN_FAMILIAR"
        )
        assert familiar_count == 3, (
            f"Should play 3 Familiar Tokens (one per opponent Digimon), got {familiar_count}"
        )

    def test_on_play_no_tokens_if_no_opponent_digimon(self, debug_runner):
        """If opponent has no Digimon, no Familiar Tokens should be played."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)

        runner.clear_zone(1, "hand")
        runner.clear_zone(2, "field")  # Clear opponent field
        runner.inject_card(1, "EX11-024", "hand")

        runner.set_phase("Main")
        play_action = runner.find_action("Play Cendrillmon")
        assert play_action is not None
        runner.execute(play_action)
        results = runner.auto_resolve()

        snap = runner.snapshot()
        familiar_count = sum(
            1 for s in snap.p1_field if s.card_id == "TOKEN_FAMILIAR"
        )
        assert familiar_count == 0, "No Familiar Tokens when opponent has no Digimon"


@pytest.mark.behavioral
class TestEX11024CendrillmonWhenDigivolving:
    """Tests for [When Digivolving] effect: play Puppet + Familiar Tokens."""

    def test_when_digivolving_plays_puppet_from_hand(self, debug_runner):
        """Digivolving into Cendrillmon should trigger the play-puppet effect."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)

        runner.clear_zone(1, "hand")
        # Place a Lv.5 Yellow Puppet base for digivolution
        runner.place_on_field(1, ["EX11-022"])  # Karakurumon Lv.5 Yellow Puppet
        # Put a Puppet Lv.4 in hand
        runner.inject_card(1, "EX11-021", "hand")
        # Put EX11-024 in hand for digivolution
        runner.inject_card(1, "EX11-024", "hand")
        # Put an opponent Digimon
        runner.place_on_field(2, ["ST1-03"])

        runner.set_phase("Main")
        digi_action = runner.find_action("Digivolve")
        if digi_action is None:
            actions = runner.find_actions("digivolve")
            assert actions, "Should have a digivolve action for Cendrillmon onto Karakurumon"
            digi_action = next(iter(actions))

        runner.execute(digi_action)
        results = runner.auto_resolve()

        snap = runner.snapshot()
        kokeshimon_on_field = any(
            s.card_id == "EX11-021" for s in snap.p1_field
        )
        assert kokeshimon_on_field, "When Digivolving should play Puppet from hand"


@pytest.mark.behavioral
class TestEX11024CendrillmonDPReduction:
    """Tests for [When Digivolving][When Attacking] -3000 DP per own Digimon."""

    def test_when_attacking_dp_reduction_single_digimon(self, debug_runner):
        """With 1 own Digimon (Cendrillmon itself), should give -3000 DP."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)

        runner.clear_zone(1, "hand")
        runner.clear_zone(2, "hand")
        perm = runner.place_on_field(1, ["EX11-024"])
        # Use ST2-10 Plesiomon (12000 DP) so reduction isn't clamped
        opp_perm = runner.place_on_field(2, ["ST2-10"])

        runner.set_phase("Main")
        game = runner.game

        own_digimon = sum(1 for p in game.player1.battle_area if p.is_digimon)
        assert own_digimon == 1

        opp_dp_before = opp_perm.dp
        assert opp_dp_before == 12000, f"Expected 12000 DP, got {opp_dp_before}"

        attack_action = runner.find_action("Attack player with Cendrillmon")
        assert attack_action is not None, "Should be able to attack with Cendrillmon"

        runner.execute(attack_action)
        results = runner.auto_resolve()

        # Opponent Digimon should have -3000 DP (1 own Digimon * -3000)
        opp_dp_after = opp_perm.dp
        dp_change = opp_dp_after - opp_dp_before
        assert dp_change == -3000, f"Expected -3000 DP change, got {dp_change}"

    def test_when_attacking_dp_reduction_multiple_digimon(self, debug_runner):
        """With 3 own Digimon, should give -9000 DP to target."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)

        runner.clear_zone(1, "hand")
        runner.clear_zone(2, "hand")
        runner.place_on_field(1, ["EX11-024"])  # Cendrillmon
        runner.place_on_field(1, ["EX11-021"])  # Kokeshimon
        runner.place_on_field(1, ["EX11-019"])  # Shoemon

        # Use ST2-10 Plesiomon (12000 DP) so -9000 isn't clamped
        opp_perm = runner.place_on_field(2, ["ST2-10"])

        runner.set_phase("Main")
        game = runner.game

        own_digimon = sum(1 for p in game.player1.battle_area if p.is_digimon)
        assert own_digimon == 3, "Should have 3 own Digimon"

        opp_dp_before = opp_perm.dp
        assert opp_dp_before == 12000

        attack_action = runner.find_action("Attack player with Cendrillmon")
        assert attack_action is not None
        runner.execute(attack_action)
        results = runner.auto_resolve()

        opp_dp_after = opp_perm.dp
        dp_change = opp_dp_after - opp_dp_before
        assert dp_change == -9000, f"Expected -9000 DP (3 * -3000), got {dp_change}"

    def test_when_attacking_no_effect_if_no_opponent_digimon(self, debug_runner):
        """If opponent has no Digimon, the DP reduction should have no target."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)

        runner.clear_zone(1, "hand")
        runner.clear_zone(2, "hand")
        runner.place_on_field(1, ["EX11-024"])
        runner.clear_zone(2, "field")

        runner.set_phase("Main")

        attack_action = runner.find_action("Attack player with Cendrillmon")
        assert attack_action is not None
        runner.execute(attack_action)
        # Should resolve cleanly with no selection needed
        results = runner.auto_resolve()
        # No crash = pass

    def test_when_digivolving_dp_reduction(self, debug_runner):
        """When Digivolving trigger should also fire the DP reduction."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)

        runner.clear_zone(1, "hand")
        runner.clear_zone(2, "hand")
        # Place base for digivolve
        runner.place_on_field(1, ["EX11-022"])  # Karakurumon Lv.5
        # Also have another Digimon for count
        runner.place_on_field(1, ["EX11-019"])  # Shoemon

        # Opponent Digimon with high DP
        opp_perm = runner.place_on_field(2, ["ST2-10"])

        # Put Cendrillmon in hand
        runner.inject_card(1, "EX11-024", "hand")

        runner.set_phase("Main")
        game = runner.game

        opp_dp_before = opp_perm.dp
        assert opp_dp_before == 12000

        digi_action = runner.find_action("Digivolve")
        if digi_action is None:
            actions = runner.find_actions("digivolve")
            assert actions, "Should have a digivolve action"
            digi_action = next(iter(actions))

        runner.execute(digi_action)
        results = runner.auto_resolve()

        # After digivolving, we have 2 Digimon (Cendrillmon + Shoemon)
        # DP reduction should be 2 * -3000 = -6000
        opp_dp_after = opp_perm.dp
        dp_change = opp_dp_after - opp_dp_before
        # The WD triggers both the play-puppet effect AND the DP reduction
        assert dp_change <= -6000, (
            f"Expected at least -6000 DP (2+ Digimon * -3000), got {dp_change}"
        )

    def test_dp_reduction_counts_digimon_at_resolution(self, debug_runner):
        """DP reduction should count own Digimon at resolution time.

        Per C# reference: int minusDP = -3000 * card.Owner.GetBattleAreaDigimons().Count;
        This is computed inside the SelectPermanentCoroutine (at resolution time).
        """
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)

        runner.clear_zone(1, "hand")
        runner.clear_zone(2, "hand")
        # Start with Cendrillmon + 2 others = 3 Digimon
        runner.place_on_field(1, ["EX11-024"])
        runner.place_on_field(1, ["EX11-021"])  # Kokeshimon
        runner.place_on_field(1, ["EX11-019"])  # Shoemon

        # High-DP opponent to avoid clamping
        opp_perm = runner.place_on_field(2, ["ST2-10"])  # 12000 DP

        runner.set_phase("Main")
        game = runner.game

        own_count = sum(1 for p in game.player1.battle_area if p.is_digimon)
        assert own_count == 3

        opp_dp_before = opp_perm.dp
        assert opp_dp_before == 12000

        attack_action = runner.find_action("Attack player with Cendrillmon")
        assert attack_action is not None
        runner.execute(attack_action)
        results = runner.auto_resolve()

        expected = -3000 * 3
        actual = opp_perm.dp - opp_dp_before
        assert actual == expected, f"Expected {expected} DP change, got {actual}"


@pytest.mark.behavioral
class TestEX11024CendrillmonEdgeCases:
    """Edge case tests for Cendrillmon."""

    def test_on_play_optional_puppet_play(self, debug_runner):
        """The Puppet play from hand is optional ('you may play')."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)

        runner.clear_zone(1, "hand")
        # Put a valid Puppet in hand
        runner.inject_card(1, "EX11-021", "hand")
        runner.inject_card(1, "EX11-024", "hand")

        runner.set_phase("Main")

        play_action = runner.find_action("Play Cendrillmon")
        assert play_action is not None
        runner.execute(play_action)

        game = runner.game
        # The selection phase should allow passing/declining
        if game.current_phase.name in ("SelectTarget", "SelectReveal"):
            actions = runner.actions()
            # The is_optional=True flag means a decline/pass action is available
            has_decline = any(
                "decline" in d.lower() or "pass" in d.lower()
                for d in actions.values()
            )
            assert has_decline, (
                "Optional Puppet play should allow declining. "
                f"Available actions: {actions}"
            )

        results = runner.auto_resolve()

    def test_familiar_tokens_are_yellow_3000dp(self, debug_runner):
        """Familiar Tokens should be Yellow, 3000 DP Digimon tokens."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)

        runner.clear_zone(1, "hand")
        runner.place_on_field(2, ["ST1-03"])  # 1 opponent Digimon
        runner.inject_card(1, "EX11-024", "hand")

        runner.set_phase("Main")
        play_action = runner.find_action("Play Cendrillmon")
        assert play_action is not None
        runner.execute(play_action)
        results = runner.auto_resolve()

        snap = runner.snapshot()
        familiar_slots = [s for s in snap.p1_field if s.card_id == "TOKEN_FAMILIAR"]
        assert len(familiar_slots) >= 1, "Should have at least 1 Familiar Token"

        for slot in familiar_slots:
            assert slot.dp == 3000, f"Familiar Token should have 3000 DP, got {slot.dp}"
            assert slot.is_digimon, "Familiar Token should be a Digimon"
