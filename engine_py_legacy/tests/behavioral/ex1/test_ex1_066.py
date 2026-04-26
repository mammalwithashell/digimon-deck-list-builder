"""Behavioral tests for EX1-066 Analog Youth (Tamer, White/Colorless, Cost 2).

Card text:
  [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card among
      them to your hand. Trash the remaining cards.
  [All Turns] When one of your level 5 or higher Digimon with digivolution
      cards is deleted, you may suspend this Tamer. If you do, gain 1 memory,
      then hatch 1 Digi-Egg card to an empty slot in your breeding area.
  [Security] Play this card without paying the cost.

Key faithfulness points tested:
  - On Play: reveals top 3, agent selects 1 Digimon, remaining trashed
  - On Play: if no Digimon among top 3, all 3 trashed
  - On Play: if fewer than 3 cards in deck, reveals what's available
  - Deletion trigger: only fires for YOUR Digimon (not opponent's)
  - Deletion trigger: only fires for level 5+ Digimon
  - Deletion trigger: only fires for Digimon WITH digivolution cards
  - Deletion trigger: does NOT fire when tamer is already suspended
  - Deletion trigger: suspends tamer as cost, then gain 1 memory + hatch
  - Deletion trigger: hatch only if breeding area is empty and eggs available
  - Security: plays self without paying cost
"""

import pytest

# Card IDs used in tests
EX1_066 = "EX1-066"      # Card under test (Analog Youth, Tamer, White, Cost 2)
ST1_03 = "ST1-03"        # Agumon (Lv.3 Red Digimon) - filler / top-of-deck Digimon
ST1_07 = "ST1-07"        # Greymon (Lv.4 Red Digimon) - digivolution source
ST1_09 = "ST1-09"        # MetalGreymon (Lv.5 Red Digimon) - deletion trigger target
ST1_02 = "ST1-02"        # Generic filler
BT1_001 = "BT1-001"      # Yokomon (Digi-Egg, Lv.2 Red)

FILLER = [ST1_03] * 4 + [ST1_02] * 50


# ── On Play: Reveal top 3, add 1 Digimon, trash rest ────────────────────


@pytest.mark.behavioral
class TestEX1066OnPlay:
    """Tests for [On Play] reveal top 3, add 1 Digimon to hand, trash rest."""

    def test_on_play_reveals_top_3_and_adds_digimon_to_hand(self, debug_runner):
        """On Play should reveal top 3 cards, let player select 1 Digimon,
        add it to hand, and trash the remaining 2."""
        runner = debug_runner(
            deck1=[EX1_066] * 4 + [ST1_03] * 46,
            deck2=FILLER,
            initial_memory=3,
        )
        runner.set_phase("Main")
        game = runner.game

        # Record top 3 cards of deck before playing
        top3_ids = [c.c_entity_base.card_id for c in game.player1.library_cards[:3]]
        hand_before = len(game.player1.hand_cards)
        deck_before = len(game.player1.library_cards)
        trash_before = len(game.player1.trash_cards)

        # Play Analog Youth
        action = runner.find_action("Analog Youth")
        assert action is not None, (
            f"Should find play action for Analog Youth. Available: {runner.actions()}"
        )
        runner.execute(action)

        # Auto-resolve the reveal selection (picks first legal)
        runner.auto_resolve()

        # After resolution:
        # - 1 Digimon from top 3 should be in hand
        # - 2 remaining should be in trash
        # - Deck should be smaller by 3 (top 3 removed)
        # - EX1-066 was also removed from hand to be played on field
        hand_after = len(game.player1.hand_cards)
        trash_after = len(game.player1.trash_cards)
        deck_after = len(game.player1.library_cards)

        # We had 'hand_before' cards, played 1 (EX1-066 to field), gained 1 from reveal
        # Net hand change: -1 (played) + 1 (selected) = 0
        assert hand_after == hand_before, (
            f"Hand should stay same size (played 1, gained 1). "
            f"Before: {hand_before}, After: {hand_after}"
        )

        # Trash should have gained exactly 2 cards (the remaining revealed)
        assert trash_after == trash_before + 2, (
            f"Trash should gain 2 cards from revealed. "
            f"Before: {trash_before}, After: {trash_after}"
        )

        # Deck should have shrunk by 3 (the top 3 revealed were removed)
        assert deck_after == deck_before - 3, (
            f"Deck should shrink by 3 (revealed cards removed). "
            f"Before: {deck_before}, After: {deck_after}"
        )

    def test_on_play_no_digimon_in_top_3_trashes_all(self, debug_runner):
        """If top 3 cards have no Digimon, all 3 should be trashed."""
        runner = debug_runner(
            deck1=[EX1_066] * 4 + [ST1_03] * 46,
            deck2=FILLER,
            initial_memory=3,
        )
        game = runner.game
        runner.set_phase("Main")

        # Inject EX1-066 into hand for playing
        runner.inject_card(1, EX1_066, "hand")

        # Replace top 3 library cards with Tamers (non-Digimon) by injecting
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        # Clear the library and rebuild with 3 tamers on top + filler below
        lib_backup = list(game.player1.library_cards)
        game.player1.library_cards.clear()
        for _ in range(3):
            tcs = db.create_card_source(EX1_066, game.player1)
            game.player1.library_cards.append(tcs)
        game.player1.library_cards.extend(lib_backup)

        # Verify top 3 are tamers
        for i in range(3):
            c = game.player1.library_cards[i]
            assert not getattr(c, 'is_digimon', False), (
                f"Top card {i} should be a Tamer, not Digimon"
            )

        trash_before = len(game.player1.trash_cards)

        action = runner.find_action("Analog Youth")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        trash_after = len(game.player1.trash_cards)

        # All 3 non-Digimon cards trashed, no card added to hand
        assert trash_after == trash_before + 3, (
            f"All 3 revealed non-Digimon cards should be trashed. "
            f"Before: {trash_before}, After: {trash_after}"
        )

    def test_on_play_fewer_than_3_cards_in_deck(self, debug_runner):
        """If deck has fewer than 3 cards, reveal what's available."""
        runner = debug_runner(
            deck1=[EX1_066] * 4 + [ST1_03] * 46,
            deck2=FILLER,
            initial_memory=3,
        )
        game = runner.game
        runner.set_phase("Main")

        # Clear the library and inject just 1 Digimon card
        game.player1.library_cards.clear()
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(ST1_03, game.player1)
        game.player1.library_cards.append(cs)

        deck_before = len(game.player1.library_cards)
        assert deck_before == 1

        action = runner.find_action("Analog Youth")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # With only 1 card (a Digimon), it should be added to hand
        # Deck should now be empty
        assert len(game.player1.library_cards) == 0, (
            "Deck should be empty after revealing the only card"
        )


# ── All Turns: Deletion trigger ─────────────────────────────────────────


@pytest.mark.behavioral
class TestEX1066DeletionTrigger:
    """Tests for [All Turns] deletion trigger: Lv5+ with digi cards deleted,
    suspend tamer, gain 1 memory, hatch."""

    def test_triggers_on_own_lv5_digimon_with_digi_cards_deleted(self, debug_runner):
        """Deletion of own Lv5+ Digimon with digivolution cards should trigger:
        tamer suspends, gain 1 memory, hatch (if empty breeding area + eggs)."""
        runner = debug_runner(
            deck1=[EX1_066] * 4 + [BT1_001] * 4 + [ST1_03] * 42,
            deck2=FILLER,
            initial_memory=0,
        )
        game = runner.game

        # Place EX1-066 tamer on P1's field (unsuspended)
        tamer_perm = runner.place_on_field(1, [EX1_066])
        assert not tamer_perm.is_suspended, "Tamer should start unsuspended"

        # Place a Lv5 Digimon WITH digivolution cards (stack: ST1-07 under ST1-09)
        digimon_perm = runner.place_on_field(1, [ST1_07, ST1_09])
        assert digimon_perm.level == 5
        assert not digimon_perm.has_no_digivolution_cards, (
            "Stacked Digimon should have digivolution cards"
        )

        # Ensure breeding area is empty and digi-eggs exist
        assert game.player1.breeding_area is None, "Breeding area should be empty"
        assert len(game.player1.digitama_library_cards) > 0, "Should have digi-eggs"

        memory_before = game.memory

        # Delete the Lv5 Digimon
        game.player1.delete_permanent(digimon_perm)

        # Check: tamer should be suspended
        assert tamer_perm.is_suspended, (
            "Tamer should be suspended after deletion trigger"
        )

        # Check: gained 1 memory
        assert game.memory == memory_before + 1, (
            f"Should gain 1 memory. Before: {memory_before}, After: {game.memory}"
        )

        # Check: hatched (breeding area should now have a Digimon)
        assert game.player1.breeding_area is not None, (
            "Breeding area should have a hatched Digi-Egg"
        )

    def test_does_not_trigger_for_lv4_digimon(self, debug_runner):
        """Deletion of Lv4 Digimon (below Lv5) should NOT trigger the effect."""
        runner = debug_runner(
            deck1=[EX1_066] * 4 + [BT1_001] * 4 + [ST1_03] * 42,
            deck2=FILLER,
            initial_memory=0,
        )
        game = runner.game

        tamer_perm = runner.place_on_field(1, [EX1_066])

        # Place a Lv4 Digimon with digi cards (stack: ST1-03 under ST1-07)
        digimon_perm = runner.place_on_field(1, [ST1_03, ST1_07])
        assert digimon_perm.level == 4

        memory_before = game.memory

        game.player1.delete_permanent(digimon_perm)

        # Tamer should NOT be suspended (Lv4 < Lv5 requirement)
        assert not tamer_perm.is_suspended, (
            "Tamer should NOT suspend for Lv4 deletion"
        )
        assert game.memory == memory_before, (
            "Should NOT gain memory for Lv4 deletion"
        )

    def test_does_not_trigger_for_digimon_without_digi_cards(self, debug_runner):
        """Deletion of Lv5 Digimon WITHOUT digivolution cards (single card)
        should NOT trigger the effect."""
        runner = debug_runner(
            deck1=[EX1_066] * 4 + [BT1_001] * 4 + [ST1_03] * 42,
            deck2=FILLER,
            initial_memory=0,
        )
        game = runner.game

        tamer_perm = runner.place_on_field(1, [EX1_066])

        # Place a Lv5 Digimon WITHOUT digivolution cards (single card in stack)
        digimon_perm = runner.place_on_field(1, [ST1_09])
        assert digimon_perm.level == 5
        assert digimon_perm.has_no_digivolution_cards, (
            "Single-card Digimon should have no digivolution cards"
        )

        memory_before = game.memory

        game.player1.delete_permanent(digimon_perm)

        assert not tamer_perm.is_suspended, (
            "Tamer should NOT suspend for Digimon without digi cards"
        )
        assert game.memory == memory_before, (
            "Should NOT gain memory for Digimon without digi cards"
        )

    def test_does_not_trigger_for_opponent_digimon_deleted(self, debug_runner):
        """Deletion of OPPONENT's Lv5+ Digimon should NOT trigger the effect.
        The card says 'one of your' Digimon."""
        runner = debug_runner(
            deck1=[EX1_066] * 4 + [BT1_001] * 4 + [ST1_03] * 42,
            deck2=[ST1_09] * 4 + [ST1_07] * 4 + [ST1_03] * 42,
            initial_memory=0,
        )
        game = runner.game

        tamer_perm = runner.place_on_field(1, [EX1_066])

        # Place a Lv5 with digi cards on OPPONENT's field
        opp_digimon = runner.place_on_field(2, [ST1_07, ST1_09])
        assert opp_digimon.level == 5
        assert not opp_digimon.has_no_digivolution_cards

        memory_before = game.memory

        game.player2.delete_permanent(opp_digimon)

        assert not tamer_perm.is_suspended, (
            "Tamer should NOT suspend for opponent's Digimon deletion"
        )
        assert game.memory == memory_before, (
            "Should NOT gain memory for opponent's Digimon deletion"
        )

    def test_does_not_trigger_when_tamer_already_suspended(self, debug_runner):
        """If the tamer is already suspended, the effect should NOT trigger
        (suspend is the cost, and already-suspended tamer can't pay it)."""
        runner = debug_runner(
            deck1=[EX1_066] * 4 + [BT1_001] * 4 + [ST1_03] * 42,
            deck2=FILLER,
            initial_memory=0,
        )
        game = runner.game

        tamer_perm = runner.place_on_field(1, [EX1_066])
        tamer_perm.suspend()  # Pre-suspend the tamer
        assert tamer_perm.is_suspended

        digimon_perm = runner.place_on_field(1, [ST1_07, ST1_09])

        memory_before = game.memory

        game.player1.delete_permanent(digimon_perm)

        # Memory should NOT change since tamer was already suspended
        assert game.memory == memory_before, (
            "Should NOT gain memory when tamer is already suspended"
        )

    def test_no_hatch_if_breeding_area_occupied(self, debug_runner):
        """If breeding area is already occupied, should still suspend + gain memory
        but NOT hatch."""
        runner = debug_runner(
            deck1=[EX1_066] * 4 + [BT1_001] * 4 + [ST1_03] * 42,
            deck2=FILLER,
            initial_memory=0,
        )
        game = runner.game

        tamer_perm = runner.place_on_field(1, [EX1_066])

        # Fill breeding area first
        runner.place_in_breeding(1, [BT1_001])
        assert game.player1.breeding_area is not None, "Breeding area should be occupied"
        eggs_before = len(game.player1.digitama_library_cards)

        digimon_perm = runner.place_on_field(1, [ST1_07, ST1_09])

        memory_before = game.memory

        game.player1.delete_permanent(digimon_perm)

        # Should still suspend and gain memory
        assert tamer_perm.is_suspended, (
            "Tamer should suspend even if breeding area is occupied"
        )
        assert game.memory == memory_before + 1, (
            "Should gain 1 memory even if breeding area is occupied"
        )

        # But should NOT hatch (breeding area was already occupied)
        assert len(game.player1.digitama_library_cards) == eggs_before, (
            "Should NOT hatch additional egg when breeding area is occupied"
        )

    def test_no_hatch_if_no_eggs_available(self, debug_runner):
        """If no digi-eggs available, should still suspend + gain memory but NOT hatch."""
        runner = debug_runner(
            deck1=[EX1_066] * 4 + [ST1_03] * 46,  # No digi-eggs in deck
            deck2=FILLER,
            initial_memory=0,
        )
        game = runner.game

        tamer_perm = runner.place_on_field(1, [EX1_066])

        # Ensure no eggs
        game.player1.digitama_library_cards.clear()
        assert len(game.player1.digitama_library_cards) == 0

        digimon_perm = runner.place_on_field(1, [ST1_07, ST1_09])

        memory_before = game.memory

        game.player1.delete_permanent(digimon_perm)

        assert tamer_perm.is_suspended, (
            "Tamer should suspend even with no eggs available"
        )
        assert game.memory == memory_before + 1, (
            "Should gain 1 memory even with no eggs"
        )
        assert game.player1.breeding_area is None, (
            "Breeding area should remain empty (no eggs to hatch)"
        )

    def test_triggers_on_both_turns(self, debug_runner):
        """[All Turns] means this should trigger during both player's turns."""
        runner = debug_runner(
            deck1=[EX1_066] * 4 + [BT1_001] * 4 + [ST1_03] * 42,
            deck2=FILLER,
            initial_memory=0,
        )
        game = runner.game

        tamer_perm = runner.place_on_field(1, [EX1_066])

        # Switch to P2's turn (simulate opponent's turn)
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        digimon_perm = runner.place_on_field(1, [ST1_07, ST1_09])

        memory_before = game.memory

        game.player1.delete_permanent(digimon_perm)

        # Should still trigger on opponent's turn
        assert tamer_perm.is_suspended, (
            "Tamer should suspend on opponent's turn too ([All Turns])"
        )
        assert game.memory == memory_before + 1, (
            "Should gain 1 memory on opponent's turn"
        )


# ── Security: Play this card without paying the cost ─────────────────────


@pytest.mark.behavioral
class TestEX1066Security:
    """Tests for [Security] Play this card without paying the cost."""

    def test_security_effect_exists(self, debug_runner):
        """EX1-066 should have a SecuritySkill effect."""
        runner = debug_runner(
            deck1=[EX1_066] * 4 + [ST1_03] * 46,
            deck2=FILLER,
        )
        game = runner.game
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.enums import EffectTiming
        db = CardDatabase()
        cs = db.create_card_source(EX1_066, game.player1)
        all_effects = cs.effect_list(None)
        sec_effects = [e for e in all_effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_effects) >= 1, (
            "EX1-066 should have at least 1 SecuritySkill effect"
        )

    def test_security_effect_plays_card(self, debug_runner):
        """When triggered from security, should play this Tamer to field free."""
        runner = debug_runner(
            deck1=[EX1_066] * 4 + [ST1_03] * 46,
            deck2=FILLER,
        )
        game = runner.game

        # Create a card source for EX1-066 and put it in security
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(EX1_066, game.player1)
        game.player1.security_cards.append(cs)

        field_before = len(game.player1.battle_area)

        # Simulate the security effect firing
        from engine_py_legacy.engine.data.enums import EffectTiming
        all_effects = cs.effect_list(None)
        sec_effects = [e for e in all_effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_effects) >= 1

        sec_effect = sec_effects[0]
        sec_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'card': cs,
        })

        # Tamer should now be on field
        assert len(game.player1.battle_area) == field_before + 1, (
            "Security effect should play the Tamer onto the field"
        )
