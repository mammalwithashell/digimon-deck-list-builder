"""Behavioral tests for AD1-025 Omnimon (Lv.7 Red/Yellow Digimon).

Card text:
  <Raid>
  <Blocker>
  <Partition ([WarGreymon] & [MetalGarurumon])> (When this Digimon with each of
      the specified digivolution cards would leave the battle area other than by
      your own effects or by battle, you may play 1 each of the specified cards
      without paying the costs.)
  [On Play] [When Digivolving] Return all of your opponent's Digimon with as many
      or fewer digivolution cards as this Digimon to the bottom of the deck. Then,
      delete 1 of your opponent's Digimon.
  [All Turns] [Once Per Turn] When any of your opponent's Digimon leave the battle
      area, trash 1 of their Option cards in the battle area and trash their top
      security card.
"""

import pytest


@pytest.mark.behavioral
class TestAD1025OmnimonKeywords:
    """Tests for Raid and Blocker keywords."""

    def test_has_raid_keyword(self, debug_runner):
        """Omnimon should have the Raid keyword."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["AD1-025"])

        assert perm.has_keyword('_is_raid'), (
            "Omnimon should have Raid keyword")

    def test_has_blocker_keyword(self, debug_runner):
        """Omnimon should have the Blocker keyword."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["AD1-025"])

        assert perm.has_keyword('_is_blocker'), (
            "Omnimon should have Blocker keyword")


@pytest.mark.behavioral
class TestAD1025OnPlayWhenDigivolving:
    """Tests for [On Play] [When Digivolving] bottom-deck + delete effect."""

    def test_on_play_bottom_decks_opponent_digimon_with_fewer_evo(self, debug_runner):
        """On Play returns all opponent Digimon with <= evo cards to deck bottom."""
        runner = debug_runner(initial_memory=20)

        # Opponent field:
        # - Digimon A: 0 digi-cards (just top card, should be bounced)
        # - Digimon B: 0 digi-cards (should be bounced)
        runner.place_on_field(2, ["BT1-009"])  # 0 digi-cards
        runner.place_on_field(2, ["BT1-009"])  # 0 digi-cards

        runner.inject_card(1, "AD1-025", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Omnimon")
        assert play is not None, (
            f"Should be able to play Omnimon. Actions: {runner.actions()}")
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Omnimon has 0 evo cards (played, not digivolved). All opponent Digimon
        # with <= 0 evo cards should be bottom-decked. Both have 0 evo cards.
        # Then "delete 1" has no remaining targets.
        opp_digimon = [s for s in snap.p2_field if s.is_digimon]
        assert len(opp_digimon) == 0, (
            f"All opponent Digimon should be bottom-decked. Got: "
            f"{[(s.card_name, len(s.stack_ids) - 1) for s in snap.p2_field]}")

    def test_on_play_does_not_bottom_deck_digimon_with_more_evo(self, debug_runner):
        """On Play does NOT bottom-deck opponent Digimon with more evo cards."""
        runner = debug_runner(initial_memory=20)

        # Omnimon played flat = 0 evo cards
        # Opponent stacked Digimon: 1 evo card (should NOT be bounced)
        runner.place_on_field(2, ["BT1-015", "BT1-009"])  # 1 digi-card

        runner.inject_card(1, "AD1-025", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Omnimon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Opponent's stacked Digimon has 1 evo card > 0 (Omnimon), so it stays.
        # But then "delete 1" fires, which should target and delete it.
        # So 0 Digimon remain on opponent's field.
        opp_digimon = [s for s in snap.p2_field if s.is_digimon]
        assert len(opp_digimon) == 0, (
            f"Opponent Digimon should be deleted by 'then delete 1'. "
            f"Remaining: {[(s.card_name, len(s.stack_ids) - 1) for s in snap.p2_field]}")

    def test_on_play_then_delete_targets_remaining(self, debug_runner):
        """After bottom-decking, 'then delete 1' targets a remaining Digimon."""
        runner = debug_runner(initial_memory=20)

        # Opponent field:
        # - Digimon A: 0 evo-cards (will be bottom-decked)
        # - Digimon B: 2 evo-cards (stays, then gets deleted)
        runner.place_on_field(2, ["BT1-009"])  # 0 digi-cards
        runner.place_on_field(2, ["AD1-004", "BT1-015", "BT1-009"])  # 2 digi-cards

        runner.inject_card(1, "AD1-025", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Omnimon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # A has 0 evo cards <= 0 (Omnimon flat), so A is bottom-decked
        # B has 2 evo cards > 0, so B stays. Then "delete 1" targets B.
        opp_digimon = [s for s in snap.p2_field if s.is_digimon]
        assert len(opp_digimon) == 0, (
            f"All opponent Digimon should be gone (1 bottom-decked, 1 deleted). "
            f"Remaining: {[(s.card_name, len(s.stack_ids) - 1) for s in snap.p2_field]}")

    def test_digivolved_omnimon_uses_own_evo_count(self, debug_runner):
        """When digivolved, Omnimon's evo card count is used for comparison."""
        runner = debug_runner(initial_memory=5)

        # Omnimon digivolved on top of a Lv.6 = 1 evo card
        # Opponent Digimon A: 0 evo cards (should be bounced, 0 <= 1)
        # Opponent Digimon B: 1 evo card (should be bounced, 1 <= 1)
        # Opponent Digimon C: 2 evo cards (should NOT be bounced, 2 > 1)
        runner.place_on_field(2, ["BT1-009"])  # 0 digi-cards
        runner.place_on_field(2, ["BT1-015", "BT1-009"])  # 1 digi-card
        runner.place_on_field(2, ["AD1-004", "BT1-015", "BT1-009"])  # 2 digi-cards

        # Place Omnimon digivolved on a Lv.6 (bottom-to-top: WarGreymon, Omnimon)
        perm = runner.place_on_field(1, ["AD1-004", "AD1-025"])  # 1 digi-card under

        # Fire on-play effects manually to simulate play/digivolve trigger
        from engine_py_legacy.engine.data.enums import EffectTiming
        runner.game.execute_effects(
            EffectTiming.OnEnterFieldAnyone,
            {"played_card": perm.top_card})
        runner.auto_resolve()

        snap = runner.snapshot()
        # A (0 evo) and B (1 evo) should be bottom-decked (both <= 1)
        # C (2 evo) stays, then "delete 1" targets C
        opp_digimon = [s for s in snap.p2_field if s.is_digimon]
        assert len(opp_digimon) == 0, (
            f"Expected 0 opponent Digimon remaining (2 bottom-decked, 1 deleted). "
            f"Got: {[(s.card_name, len(s.stack_ids) - 1) for s in snap.p2_field]}")


@pytest.mark.behavioral
class TestAD1025AllTurnsOPT:
    """Tests for [All Turns] [Once Per Turn] opponent Digimon leaves effect."""

    def test_trashes_option_when_opponent_digimon_deleted(self, debug_runner):
        """When opponent Digimon is deleted, trash 1 opponent Option in battle area."""
        runner = debug_runner(initial_memory=5)

        runner.place_on_field(1, ["AD1-025"])

        # Place opponent Option card in battle area and a Digimon to delete
        runner.place_on_field(2, ["BT1-090"])  # Option card on field
        target = runner.place_on_field(2, ["BT1-009"])  # Digimon to delete

        before_snap = runner.snapshot()
        p2_options_before = [s for s in before_snap.p2_field if not s.is_digimon and not s.is_tamer]

        # Delete opponent's Digimon (by opponent's own effect to avoid interference)
        runner.game.player2.delete_permanent(target, is_opponent_effect=True)
        runner.auto_resolve()

        snap = runner.snapshot()
        # The option should have been trashed
        p2_options_after = [s for s in snap.p2_field if not s.is_digimon and not s.is_tamer]
        assert len(p2_options_after) < len(p2_options_before), (
            f"Opponent's Option should have been trashed. "
            f"Before: {len(p2_options_before)}, After: {len(p2_options_after)}")

    def test_trashes_security_when_opponent_digimon_deleted(self, debug_runner):
        """When opponent Digimon is deleted, trash opponent's top security card."""
        runner = debug_runner(initial_memory=5)

        runner.place_on_field(1, ["AD1-025"])
        target = runner.place_on_field(2, ["BT1-009"])

        before_sec = runner.snapshot().p2_security_count

        runner.game.player2.delete_permanent(target, is_opponent_effect=True)
        runner.auto_resolve()

        after_sec = runner.snapshot().p2_security_count
        assert after_sec == before_sec - 1, (
            f"Opponent should have lost 1 security. Before: {before_sec}, After: {after_sec}")

    def test_once_per_turn_limit(self, debug_runner):
        """Effect should only trigger once per turn."""
        runner = debug_runner(initial_memory=5)

        runner.place_on_field(1, ["AD1-025"])
        target1 = runner.place_on_field(2, ["BT1-009"])
        target2 = runner.place_on_field(2, ["BT1-009"])

        before_sec = runner.snapshot().p2_security_count

        # Delete first Digimon — effect triggers
        runner.game.player2.delete_permanent(target1, is_opponent_effect=True)
        runner.auto_resolve()

        after_first = runner.snapshot().p2_security_count
        assert after_first == before_sec - 1, (
            "Security should decrease by 1 after first deletion")

        # Delete second Digimon — effect should NOT trigger again (OPT)
        runner.game.player2.delete_permanent(target2, is_opponent_effect=True)
        runner.auto_resolve()

        after_second = runner.snapshot().p2_security_count
        assert after_second == after_first, (
            f"Security should NOT decrease again (OPT). After first: {after_first}, "
            f"After second: {after_second}")

    def test_does_not_trigger_on_own_digimon_leaving(self, debug_runner):
        """Effect should NOT trigger when own Digimon leaves (only opponent's)."""
        runner = debug_runner(initial_memory=5)

        runner.place_on_field(1, ["AD1-025"])
        own_target = runner.place_on_field(1, ["BT1-009"])  # OWN Digimon

        before_sec = runner.snapshot().p2_security_count

        # Delete P1's own Digimon
        runner.game.player1.delete_permanent(own_target, is_opponent_effect=False)
        runner.auto_resolve()

        after_sec = runner.snapshot().p2_security_count
        assert after_sec == before_sec, (
            f"Opponent security should NOT change when own Digimon is deleted. "
            f"Before: {before_sec}, After: {after_sec}")


@pytest.mark.behavioral
class TestAD1025Partition:
    """Tests for Partition ([WarGreymon] & [MetalGarurumon])."""

    def test_partition_plays_wargreymon_and_metalgarurumon_on_deletion(self, debug_runner):
        """When Omnimon (with both in evo) is deleted by opponent, play both from trash."""
        runner = debug_runner(initial_memory=5)

        # Omnimon with WarGreymon and MetalGarurumon in evo stack
        # place_on_field takes bottom-to-top: MetalGarurumon, WarGreymon, Omnimon
        perm = runner.place_on_field(1, ["AD1-014", "AD1-004", "AD1-025"])

        before_field_count = len(runner.snapshot().p1_field)

        # Delete by opponent effect (not own, not battle)
        runner.game.player1.delete_permanent(perm, is_opponent_effect=True)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Omnimon is gone but WarGreymon and MetalGarurumon should be played
        field_names = [s.card_name for s in snap.p1_field]
        assert "WarGreymon" in field_names, (
            f"WarGreymon should have been played via Partition. Field: {field_names}")
        assert "MetalGarurumon" in field_names, (
            f"MetalGarurumon should have been played via Partition. Field: {field_names}")
        assert "Omnimon" not in field_names, (
            f"Omnimon should NOT be on the field anymore. Field: {field_names}")

    def test_partition_does_not_trigger_on_own_effect_removal(self, debug_runner):
        """Partition should NOT trigger when removed by own effects."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["AD1-014", "AD1-004", "AD1-025"])

        # Delete by own effect (is_opponent_effect=False)
        runner.game.player1.delete_permanent(perm, is_opponent_effect=False)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Nothing should have been played via Partition
        field_names = [s.card_name for s in snap.p1_field]
        assert "WarGreymon" not in field_names, (
            f"WarGreymon should NOT be played (removed by own effect). Field: {field_names}")
        assert "MetalGarurumon" not in field_names, (
            f"MetalGarurumon should NOT be played (removed by own effect). Field: {field_names}")

    def test_partition_does_not_trigger_on_battle_removal(self, debug_runner):
        """Partition should NOT trigger when removed by battle."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["AD1-014", "AD1-004", "AD1-025"])

        # Delete by battle
        runner.game.player1.delete_permanent(perm, is_battle=True, removal_cause='battle')
        runner.auto_resolve()

        snap = runner.snapshot()
        field_names = [s.card_name for s in snap.p1_field]
        assert "WarGreymon" not in field_names, (
            f"WarGreymon should NOT be played (removed by battle). Field: {field_names}")

    def test_partition_requires_both_names_in_evo(self, debug_runner):
        """Partition needs BOTH WarGreymon AND MetalGarurumon in evo cards."""
        runner = debug_runner(initial_memory=5)

        # Only WarGreymon in evo, no MetalGarurumon (bottom-to-top)
        perm = runner.place_on_field(1, ["AD1-004", "AD1-025"])

        runner.game.player1.delete_permanent(perm, is_opponent_effect=True)
        runner.auto_resolve()

        snap = runner.snapshot()
        field_names = [s.card_name for s in snap.p1_field]
        assert "WarGreymon" not in field_names, (
            f"Partition should NOT fire without BOTH names. Field: {field_names}")
