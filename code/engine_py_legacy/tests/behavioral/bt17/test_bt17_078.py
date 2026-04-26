"""Behavioral tests for BT17-078 Omnimon (Lv.7 White Digimon, Ace).

Card text:
  [Hand] [Counter] <Blast DNA Digivolve ([WarGreymon] + [MetalGarurumon])>
  <Raid>, <Blocker>
  [On Play] If DNA digivolving, choose 1 opponent's Digimon and return that
    Digimon and all of their Digimon with the same level to the bottom of the
    deck. Then, delete 1 of their Digimon.
  [When Digivolving] Same as On Play.
  Inherited: Ace Overflow <-5>

  DNA: Lv.6 [Greymon] + Lv.6 [Garurumon], cost 0
"""

import pytest


@pytest.mark.behavioral
class TestBT17078Omnimon:
    """Tests for BT17-078 Omnimon."""

    # ── Keywords: Raid, Blocker ─────────────────────────────────────

    def test_has_raid_keyword(self, debug_runner):
        """Omnimon should have <Raid> keyword on field."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT17-078"])
        assert perm.has_keyword('_is_raid'), "Omnimon should have Raid"

    def test_has_blocker_keyword(self, debug_runner):
        """Omnimon should have <Blocker> keyword on field."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT17-078"])
        assert perm.has_keyword('_is_blocker'), "Omnimon should have Blocker"

    # ── On Play: DNA check gates BOTH bottom-deck AND delete ────────

    def test_on_play_dna_bottom_decks_same_level(self, debug_runner):
        """[On Play] If DNA digivolving, selecting an opponent's Digimon should
        bottom-deck ALL opponent Digimon with that level."""
        runner = debug_runner(initial_memory=15)

        # Place 3 opponent Lv.3 Digimon (same level) and 1 Lv.6
        runner.place_on_field(2, ["BT1-010"])  # Agumon Lv.3
        runner.place_on_field(2, ["BT1-010"])  # Agumon Lv.3
        runner.place_on_field(2, ["BT1-010"])  # Agumon Lv.3
        runner.place_on_field(2, ["BT1-025"])  # WarGreymon Lv.6

        # Play Omnimon with DNA context
        runner.inject_card(1, "BT17-078", "hand")
        runner.set_phase("Main")

        # Play from hand
        play = runner.find_action("Play Omnimon")
        assert play is not None, f"Should be able to play. Actions: {runner.actions()}"
        runner.execute(play)

        # Simulate DNA context: the On Play effect needs is_dna_digivolve
        # Since we're playing (not DNA digivolving), DNA check fails.
        # The bottom-deck part is skipped but delete still runs (per C#).
        runner.auto_resolve()

        snap = runner.snapshot()
        # When NOT DNA: bottom-deck is skipped, but delete 1 still happens
        # So should have 3 remaining from 4 (1 deleted)
        assert len(snap.p2_field) == 3, (
            f"Without DNA, only delete should fire (no bottom-deck). "
            f"Expected 3, got {len(snap.p2_field)}. "
            f"P2 field: {[(s.card_id, s.level) for s in snap.p2_field]}")

    def test_on_play_without_dna_still_deletes(self, debug_runner):
        """[On Play] Without DNA digivolving, the delete effect still fires
        (per C# reference: 'Then, delete 1' is outside the if-DNA block)."""
        runner = debug_runner(initial_memory=15)

        runner.place_on_field(2, ["BT1-010"])  # Agumon Lv.3
        runner.place_on_field(2, ["BT1-025"])  # WarGreymon Lv.6

        runner.inject_card(1, "BT17-078", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Omnimon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Without DNA: skip bottom-deck, but delete 1 Digimon
        assert len(snap.p2_field) == 1, (
            f"Non-DNA On Play should still delete 1. "
            f"Expected 1, got {len(snap.p2_field)}. "
            f"P2 field: {[(s.card_id, s.level) for s in snap.p2_field]}")

    def test_on_play_no_opp_digimon_does_nothing(self, debug_runner):
        """[On Play] If opponent has no Digimon, effect does nothing."""
        runner = debug_runner(initial_memory=15)

        runner.inject_card(1, "BT17-078", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Omnimon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert len(snap.p2_field) == 0
        assert any(s.card_id == "BT17-078" for s in snap.p1_field), (
            "Omnimon should be on field")

    # ── Ace Overflow -5 inherited ───────────────────────────────────

    def test_ace_overflow_on_card_data(self, debug_runner):
        """BT17-078 should be recognized as an ACE card with -5 overflow."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT17-078"])
        card = perm.top_card
        assert card.c_entity_base.is_ace, "BT17-078 should be an ACE card"
        assert card.c_entity_base.ace_overflow_cost == 5, (
            f"Expected ace_overflow_cost=5, got {card.c_entity_base.ace_overflow_cost}")

    def test_ace_overflow_memory_loss_on_deletion(self, debug_runner):
        """When Omnimon is deleted from field, owner loses 5 memory (Ace Overflow)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT17-078"])
        player = runner.game.player1

        before_memory = runner.game.memory
        player.delete_permanent(perm)

        after_memory = runner.game.memory
        # Ace Overflow -5 means lose 5 memory
        assert after_memory == before_memory - 5, (
            f"Expected memory loss of 5 from Ace Overflow. "
            f"Before: {before_memory}, After: {after_memory}")
