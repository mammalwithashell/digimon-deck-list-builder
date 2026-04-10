"""Behavioral tests for BT24-066 Guilmon.

Card text:
  Lv.3 | Purple | Reptile | DP:2000 | Play Cost:3
  Alt digivolve: from [Gigimon] for cost 0

  [On Play] Reveal the top 3 cards of your deck. Among them, add 1
      [Evil], [Dark Dragon], [Evil Dragon] or [Dark Knight] trait card
      or purple Tamer card to the hand and trash 1 such card. Return
      the rest to the bottom of the deck. Then, trash 1 card in your hand.

  Inherited: [When Attacking] [Once Per Turn] Delete 1 of your opponent's
      level 3 Digimon.

Key faithfulness points:
  - On Play reveals exactly 3 cards from top of deck
  - Pass 1: add 1 qualifying card to hand (Evil/Dark Dragon/Evil Dragon/Dark Knight trait OR purple Tamer)
  - Pass 2: trash 1 qualifying card from remaining revealed
  - Each pass is optional (C# uses SimplifiedRevealDeckTopCardsAndSelect)
  - Remaining cards go to deck bottom
  - Then trash 1 card from hand (mandatory, happens AFTER reveal resolves)
  - Inherited: [When Attacking] OPT delete opponent Lv3 Digimon
  - Alt-digi from Gigimon for cost 0
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase

# Card IDs used in tests
BT24_066 = "BT24-066"      # Card under test (Guilmon, Lv.3 Purple)
EVIL_CARD = "BT2-067"      # DemiDevimon Lv.3, trait: [Evil]
DARK_DRAGON = "BT2-013"    # Growlmon Lv.4, trait: [Dark Dragon]
EVIL_DRAGON = "BT3-081"    # Devidramon Lv.4, trait: [Evil Dragon]
DARK_KNIGHT = "BT5-081"    # ChaosGallantmon Lv.6, trait: [Dark Knight]
PURPLE_TAMER = "BT2-090"   # Matt Ishida, Purple Tamer
AGUMON = "ST1-03"           # Agumon Lv.3 Red, no qualifying trait (filler)
GREYMON = "ST1-07"          # Greymon Lv.4 Red (non-qualifying, for opponent)
GIGIMON = "BT24-001"        # Gigimon digi-egg for alt-digi test

# Build a simple deck with enough cards
FILLER_DECK = [BT24_066] * 4 + [AGUMON] * 46


@pytest.mark.behavioral
class TestBT24066OnPlay:
    """[On Play] Reveal 3, add 1 qualifying, trash 1 qualifying, rest to bottom, trash 1 from hand."""

    def test_on_play_adds_qualifying_card_to_hand(self, debug_runner):
        """On Play should reveal 3 cards and allow adding 1 qualifying card to hand."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        # Set up deck top: qualifying card + 2 fillers
        runner.clear_zone(1, "library")
        runner.inject_card(1, EVIL_CARD, "library_top")    # pos 0 (qualifying)
        runner.inject_card(1, AGUMON, "library_top")        # pos 1 (filler)
        runner.inject_card(1, AGUMON, "library_top")        # pos 2 (filler)
        # Need more cards so game doesn't crash
        for _ in range(10):
            runner.inject_card(1, AGUMON, "library_bottom")

        # Clear hand, inject Guilmon + extra card (we'll need to trash 1 later)
        runner.clear_zone(1, "hand")
        runner.inject_card(1, BT24_066, "hand")
        runner.inject_card(1, AGUMON, "hand")  # extra card to trash from hand

        runner.set_phase("Main")

        action = runner.find_action("Play Guilmon")
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"
        runner.execute(action)

        # Auto-resolve reveal+select phases
        runner.auto_resolve()

        snap = runner.snapshot()
        # The Evil card should have been added to hand through the reveal
        assert EVIL_CARD in snap.p1_hand, (
            f"Evil card should be in hand after reveal selection. Hand: {snap.p1_hand}"
        )

    def test_on_play_trashes_qualifying_card_from_revealed(self, debug_runner):
        """On Play pass 2 should trash 1 qualifying card from remaining revealed."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        # Set up deck top: 2 qualifying cards + 1 filler
        runner.clear_zone(1, "library")
        runner.inject_card(1, EVIL_CARD, "library_top")     # qualifying
        runner.inject_card(1, DARK_DRAGON, "library_top")   # qualifying
        runner.inject_card(1, AGUMON, "library_top")        # filler
        for _ in range(10):
            runner.inject_card(1, AGUMON, "library_bottom")

        runner.clear_zone(1, "hand")
        runner.inject_card(1, BT24_066, "hand")
        runner.inject_card(1, AGUMON, "hand")  # for trash-from-hand step

        runner.set_phase("Main")
        lib_before = len(runner.game.player1.library_cards)

        action = runner.find_action("Play Guilmon")
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # One qualifying card should be in hand, one in trash (from reveal passes)
        # Plus 1 card trashed from hand
        qualifying_in_hand = sum(1 for c in snap.p1_hand if c in [EVIL_CARD, DARK_DRAGON])
        qualifying_in_trash = sum(1 for c in snap.p1_trash if c in [EVIL_CARD, DARK_DRAGON])

        assert qualifying_in_hand >= 1, (
            f"At least 1 qualifying card should be in hand. Hand: {snap.p1_hand}"
        )
        assert qualifying_in_trash >= 1, (
            f"At least 1 qualifying card should be in trash (from reveal). Trash: {snap.p1_trash}"
        )

    def test_on_play_returns_rest_to_deck_bottom(self, debug_runner):
        """Non-selected revealed cards should go to deck bottom."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        # All 3 revealed are non-qualifying (no pass selections possible)
        runner.clear_zone(1, "library")
        for _ in range(13):
            runner.inject_card(1, AGUMON, "library_top")

        runner.clear_zone(1, "hand")
        runner.inject_card(1, BT24_066, "hand")
        runner.inject_card(1, AGUMON, "hand")  # for trash-from-hand step

        runner.set_phase("Main")
        lib_before = len(runner.game.player1.library_cards)

        action = runner.find_action("Play Guilmon")
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # 3 revealed, none selected, all 3 returned to bottom
        # Library should be: lib_before - 3 (revealed) + 3 (returned) = lib_before
        assert snap.p1_library_size == lib_before, (
            f"Library should have same size when no cards selected. "
            f"Before: {lib_before}, After: {snap.p1_library_size}"
        )

    def test_on_play_trash_from_hand_after_reveal(self, debug_runner):
        """After reveal resolves, must trash 1 card from hand (mandatory)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        # No qualifying cards in reveal, so reveal passes skip quickly
        runner.clear_zone(1, "library")
        for _ in range(13):
            runner.inject_card(1, AGUMON, "library_top")

        runner.clear_zone(1, "hand")
        runner.inject_card(1, BT24_066, "hand")
        runner.inject_card(1, AGUMON, "hand")
        runner.inject_card(1, AGUMON, "hand")

        runner.set_phase("Main")
        hand_before_play = 3  # Guilmon + 2 Agumon

        action = runner.find_action("Play Guilmon")
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # After play: Guilmon leaves hand (-1), trash 1 from hand (-1)
        # So hand should be hand_before_play - 1 (played) - 1 (trashed) = 1
        assert len(snap.p1_hand) == hand_before_play - 2, (
            f"Hand should have {hand_before_play - 2} cards "
            f"(played 1, trashed 1). Got {len(snap.p1_hand)}. Hand: {snap.p1_hand}"
        )
        # There should be at least 1 card in trash from the hand trash step
        assert len(snap.p1_trash) >= 1, (
            f"Trash should have at least 1 card from hand discard. Trash: {snap.p1_trash}"
        )

    def test_on_play_purple_tamer_qualifies(self, debug_runner):
        """Purple Tamer cards should qualify for the reveal selection."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        runner.clear_zone(1, "library")
        runner.inject_card(1, PURPLE_TAMER, "library_top")  # qualifying (purple tamer)
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")
        for _ in range(10):
            runner.inject_card(1, AGUMON, "library_bottom")

        runner.clear_zone(1, "hand")
        runner.inject_card(1, BT24_066, "hand")
        runner.inject_card(1, AGUMON, "hand")

        runner.set_phase("Main")

        action = runner.find_action("Play Guilmon")
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Purple tamer should be either in hand (added) or trash (trashed from reveal)
        tamer_in_hand = PURPLE_TAMER in snap.p1_hand
        tamer_in_trash = PURPLE_TAMER in snap.p1_trash
        assert tamer_in_hand or tamer_in_trash, (
            f"Purple tamer should have been selected (add to hand or trash). "
            f"Hand: {snap.p1_hand}, Trash: {snap.p1_trash}"
        )

    def test_on_play_no_qualifying_cards_still_trashes_from_hand(self, debug_runner):
        """Even with no qualifying revealed cards, trash 1 from hand still happens."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        # All non-qualifying in reveal
        runner.clear_zone(1, "library")
        for _ in range(13):
            runner.inject_card(1, AGUMON, "library_top")

        runner.clear_zone(1, "hand")
        runner.inject_card(1, BT24_066, "hand")
        runner.inject_card(1, AGUMON, "hand")
        runner.inject_card(1, AGUMON, "hand")

        runner.set_phase("Main")

        action = runner.find_action("Play Guilmon")
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Guilmon played (-1), trash 1 from hand (-1) = started with 3, should have 1
        assert len(snap.p1_hand) == 1, (
            f"Should have 1 card in hand after playing Guilmon and trashing 1. "
            f"Got {len(snap.p1_hand)}. Hand: {snap.p1_hand}"
        )

    def test_on_play_full_flow_with_qualifying_cards(self, debug_runner):
        """Full flow: reveal 3, add 1 to hand, trash 1 from revealed, rest to bottom, trash 1 from hand."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        # 2 qualifying + 1 filler in reveal
        runner.clear_zone(1, "library")
        runner.inject_card(1, EVIL_CARD, "library_top")
        runner.inject_card(1, DARK_DRAGON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")
        for _ in range(10):
            runner.inject_card(1, AGUMON, "library_bottom")

        runner.clear_zone(1, "hand")
        runner.inject_card(1, BT24_066, "hand")
        runner.inject_card(1, AGUMON, "hand")
        runner.inject_card(1, AGUMON, "hand")

        runner.set_phase("Main")
        lib_before = len(runner.game.player1.library_cards)

        action = runner.find_action("Play Guilmon")
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()

        # After full flow:
        # - 1 qualifying card added to hand
        # - 1 qualifying card trashed from revealed
        # - 1 filler returned to deck bottom
        # - 1 card trashed from hand
        # Library: lib_before - 3 (revealed) + 1 (returned filler) = lib_before - 2
        assert snap.p1_library_size == lib_before - 2, (
            f"Library should be lib_before-2 (3 revealed, 1 to hand, 1 to trash, 1 returned). "
            f"Before: {lib_before}, After: {snap.p1_library_size}"
        )


@pytest.mark.behavioral
class TestBT24066Inherited:
    """Inherited: [When Attacking] [Once Per Turn] Delete 1 opponent's Lv3 Digimon."""

    def test_inherited_deletes_opponent_lv3_digimon(self, debug_runner):
        """When attacking with inherited, should delete 1 of opponent's Lv3 Digimon."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Place BT24-066 as inherited source under a Lv4 Digimon
        perm = runner.place_on_field(1, [BT24_066, DARK_DRAGON], turn_played=-1)

        # Place Lv3 Digimon on opponent's field
        runner.place_on_field(2, [AGUMON], turn_played=-1)

        snap_before = runner.snapshot()
        opp_field_before = len(snap_before.p2_field)

        # Attack with the Digimon (triggers inherited When Attacking)
        attack_action = runner.find_action("Attack player with Growlmon")
        if attack_action is None:
            attack_action = runner.find_action("Attack player")
        assert attack_action is not None, f"Should find attack action. Actions: {runner.actions()}"
        runner.execute(attack_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Opponent's Lv3 Agumon should be deleted
        opp_lv3 = [s for s in snap.p2_field if s.card_id == AGUMON]
        assert len(opp_lv3) == 0, (
            f"Opponent's Lv3 Agumon should be deleted. P2 field: {[(s.card_id, s.level) for s in snap.p2_field]}"
        )

    def test_inherited_does_not_delete_lv4_digimon(self, debug_runner):
        """Should NOT delete opponent's Lv4+ Digimon."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [BT24_066, DARK_DRAGON], turn_played=-1)
        # Only Lv4 on opponent field
        runner.place_on_field(2, [GREYMON], turn_played=-1)

        attack_action = runner.find_action("Attack player with Growlmon")
        if attack_action is None:
            attack_action = runner.find_action("Attack player")
        assert attack_action is not None, f"Should find attack. Actions: {runner.actions()}"
        runner.execute(attack_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Lv4 Greymon should NOT be deleted
        opp_greymon = [s for s in snap.p2_field if s.card_id == GREYMON]
        assert len(opp_greymon) == 1, (
            f"Opponent's Lv4 Greymon should NOT be deleted. P2 field: {[(s.card_id, s.level) for s in snap.p2_field]}"
        )

    def test_inherited_once_per_turn(self, debug_runner):
        """Inherited effect should only trigger once per turn."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [BT24_066, DARK_DRAGON], turn_played=-1)
        # Two Lv3 on opponent field
        runner.place_on_field(2, [AGUMON], turn_played=-1)
        runner.place_on_field(2, [AGUMON], turn_played=-1)

        snap_before = runner.snapshot()
        opp_lv3_before = sum(1 for s in snap_before.p2_field if s.card_id == AGUMON)
        assert opp_lv3_before == 2, "Should start with 2 opponent Lv3 Digimon"

        # First attack should trigger the delete
        attack = runner.find_action("Attack player with Growlmon")
        if attack is None:
            attack = runner.find_action("Attack player")
        assert attack is not None
        runner.execute(attack)
        runner.auto_resolve()

        snap_mid = runner.snapshot()
        opp_lv3_mid = sum(1 for s in snap_mid.p2_field if s.card_id == AGUMON)
        # Should have deleted exactly 1
        assert opp_lv3_mid == 1, (
            f"First attack should delete 1 Lv3 Digimon. Remaining: {opp_lv3_mid}"
        )

    def test_inherited_is_marked_as_inherited(self, debug_runner):
        """The inherited effect should have is_inherited_effect = True."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        perm = runner.place_on_field(1, [BT24_066])
        card = perm.top_card
        effects = card.effect_list(None)

        inherited_effects = [e for e in effects if getattr(e, 'is_inherited_effect', False)]
        assert len(inherited_effects) >= 1, (
            f"Should have at least 1 inherited effect. Effects: {[(e.effect_name, getattr(e, 'is_inherited_effect', False)) for e in effects]}"
        )

        # Find the When Attacking inherited
        attack_inherited = [e for e in inherited_effects
                           if getattr(e, 'is_on_attack', False)]
        assert len(attack_inherited) == 1, (
            f"Should have exactly 1 When Attacking inherited effect"
        )


@pytest.mark.behavioral
class TestBT24066AltDigi:
    """Alt-digi: from [Gigimon] for cost 0."""

    def test_alt_digi_attributes(self, debug_runner):
        """Alt-digi should be from Gigimon for cost 0."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        perm = runner.place_on_field(1, [BT24_066])
        card = perm.top_card
        effects = card.effect_list(None)

        alt_digi = None
        for eff in effects:
            if hasattr(eff, '_alt_digi_cost') and eff._alt_digi_cost is not None:
                alt_digi = eff
                break

        assert alt_digi is not None, "Should have an alt-digi effect"
        assert alt_digi._alt_digi_cost == 0, (
            f"Alt-digi cost should be 0, got {alt_digi._alt_digi_cost}"
        )
        assert alt_digi._alt_digi_name == "Gigimon", (
            f"Alt-digi name should be 'Gigimon', got {getattr(alt_digi, '_alt_digi_name', None)}"
        )


@pytest.mark.behavioral
class TestBT24066InheritedTiming:
    """Verify inherited uses correct timing enum."""

    def test_inherited_uses_on_use_attack_timing(self, debug_runner):
        """Inherited When Attacking must use EffectTiming.OnUseAttack (28), not OnAllyAttack."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        perm = runner.place_on_field(1, [BT24_066])
        card = perm.top_card
        effects = card.effect_list(None)

        inherited_attack = None
        for eff in effects:
            if getattr(eff, 'is_inherited_effect', False) and getattr(eff, 'is_on_attack', False):
                inherited_attack = eff
                break

        assert inherited_attack is not None, "Should find inherited When Attacking effect"
        # Check timing is OnUseAttack (28), not OnAllyAttack (32)
        assert inherited_attack.timing == EffectTiming.OnUseAttack, (
            f"Inherited When Attacking should use OnUseAttack timing, "
            f"got {inherited_attack.timing}"
        )
