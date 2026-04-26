"""Behavioral tests for BT15-050 Cherrymon | Lv.5 | Green | Vegetation

Card text:
[On Play] Reveal the top 4 cards of your deck. Add 2 level 6 or higher cards
    among them to the hand. Return the rest to the bottom of the deck.
[End of Your Turn] By deleting 1 of your Digimon, you may play 1 Digimon card
    with the [Dark Masters] trait from your hand to an empty space in your
    breeding area without paying the cost.
Inherited: <Piercing>

Key issues tested:
- On Play reveal + select up to 2 Lv6+ to hand
- End of Turn: delete-as-cost -> play Dark Masters Digimon to BREEDING (not battle area)
- Breeding area must be empty for the play
- Inherited Piercing keyword
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


# --- Helpers ---

FILLER = ["ST1-03"] * 4 + ["BT1-010"] * 46  # 50-card filler deck


def _make_deck_with(*card_ids):
    """Build a 50-card deck starting with the given card IDs, padded with filler."""
    deck = list(card_ids)
    while len(deck) < 50:
        deck.append("ST1-03")
    return deck


@pytest.mark.behavioral
class TestBT15050CherrymonStructure:
    """Effect structure and timing tests."""

    def test_has_three_effects(self, debug_runner):
        """Script should produce 3 effects: OnPlay reveal, EndOfTurn play, inherited Piercing."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-050"])

        top_card = perm.top_card
        all_effects = top_card.effect_list(None)
        assert len(all_effects) >= 3, f"Expected at least 3 effects, got {len(all_effects)}"

    def test_effect0_on_play_timing(self, debug_runner):
        """Effect 0 should be OnEnterFieldAnyone with is_on_play."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-050"])

        effects = perm.top_card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and getattr(e, 'is_on_play', False)]
        assert len(on_play_effects) >= 1, "Should have OnPlay effect"

    def test_effect1_end_of_turn_timing(self, debug_runner):
        """Effect 1 should be OnEndTurn and optional."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-050"])

        effects = perm.top_card.effect_list(None)
        eot_effects = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(eot_effects) >= 1, "Should have OnEndTurn effect"
        assert eot_effects[0].is_optional, "End of Turn effect should be optional"

    def test_effect2_inherited_piercing(self, debug_runner):
        """Inherited effect should grant Piercing."""
        runner = debug_runner(initial_memory=5)
        # Place BT15-050 under a Lv.6 so inherited effect is available
        perm = runner.place_on_field(1, ["BT15-050", "BT15-052"])

        effects = perm.card_sources[0].effect_list(None)  # BT15-050 is bottom
        inherited = [e for e in effects if getattr(e, 'is_inherited_effect', False)]
        piercing = [e for e in inherited if getattr(e, '_is_piercing', False)]
        assert len(piercing) >= 1, "Should have inherited Piercing effect"


@pytest.mark.behavioral
class TestBT15050OnPlayReveal:
    """[On Play] Reveal top 4, add up to 2 Lv6+ to hand, rest to bottom."""

    def test_on_play_reveals_4_and_adds_lv6_to_hand(self, debug_runner):
        """On Play should reveal 4 cards and allow selecting Lv6+ cards to hand."""
        # Build deck with known top cards: 2 Lv6+ and 2 Lv3
        deck = _make_deck_with(
            "BT15-050",  # This card (will be played from hand)
            "BT15-052",  # Lv.6 Puppetmon (Dark Masters) - top of deck after draw
            "BT15-031",  # Lv.6 MetalSeadramon (Dark Masters)
            "ST1-03",    # Lv.3 Agumon
            "ST1-03",    # Lv.3 Agumon
        )
        runner = debug_runner(deck1=deck, deck2=FILLER, initial_memory=10)
        game = runner.game
        p1 = game.player1

        # Clear hand, put known cards on top of library
        p1.hand_cards.clear()
        p1.library_cards.clear()
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        # Set up library: Lv6, Lv6, Lv3, Lv3, then padding
        top_cards = ["BT15-052", "BT15-031", "ST1-03", "ST1-03"]
        for cid in top_cards:
            cs = db.create_card_source(cid, p1)
            p1.library_cards.append(cs)
        # Add padding
        for _ in range(40):
            cs = db.create_card_source("ST1-03", p1)
            p1.library_cards.append(cs)

        hand_before = len(p1.hand_cards)
        lib_before = len(p1.library_cards)

        # Place Cherrymon on field and trigger On Play effect
        perm = runner.place_on_field(1, ["BT15-050"])
        effects = perm.top_card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone
                   and getattr(e, 'is_on_play', False)][0]

        on_play.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # Should have added up to 2 Lv6+ cards to hand
        hand_after = len(p1.hand_cards)
        assert hand_after >= hand_before + 1, \
            f"Should have added at least 1 Lv6+ card to hand (before={hand_before}, after={hand_after})"
        assert hand_after <= hand_before + 2, \
            f"Should have added at most 2 cards to hand"

    def test_on_play_rest_to_deck_bottom(self, debug_runner):
        """Cards not selected should go to deck bottom."""
        deck = _make_deck_with("BT15-050")
        runner = debug_runner(deck1=deck, deck2=FILLER, initial_memory=10)
        game = runner.game
        p1 = game.player1

        p1.hand_cards.clear()
        p1.library_cards.clear()
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        # All Lv3 (none qualify) - all should go to bottom
        for _ in range(10):
            cs = db.create_card_source("ST1-03", p1)
            p1.library_cards.append(cs)

        lib_before = len(p1.library_cards)

        perm = runner.place_on_field(1, ["BT15-050"])
        effects = perm.top_card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone
                   and getattr(e, 'is_on_play', False)][0]

        on_play.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # No Lv6+ to pick, all 4 should be returned to bottom
        assert len(p1.hand_cards) == 0, "No Lv6+ cards to add to hand"
        # Library should have lost 4 (revealed) but gained 4 back (returned to bottom)
        assert len(p1.library_cards) == lib_before, \
            f"Library should be same size (all returned to bottom)"


@pytest.mark.behavioral
class TestBT15050EndOfTurnBreedingPlay:
    """[End of Your Turn] Delete own Digimon -> play Dark Masters to BREEDING area."""

    def test_end_of_turn_plays_to_breeding_not_battle(self, debug_runner):
        """CRITICAL: The played Dark Masters Digimon must go to BREEDING area, not battle area."""
        deck = _make_deck_with("BT15-050", "BT15-052")
        runner = debug_runner(deck1=deck, deck2=FILLER, initial_memory=10)
        game = runner.game
        p1 = game.player1

        # Place Cherrymon on field (it has the End of Turn effect)
        perm_cherrymon = runner.place_on_field(1, ["BT15-050"])
        # Place a sacrificial Digimon (Agumon) to delete as cost
        perm_sacrifice = runner.place_on_field(1, ["ST1-03"])

        # Put a Dark Masters Digimon (Puppetmon) in hand
        runner.inject_card(1, "BT15-052", "hand")

        # Ensure breeding area is empty
        p1.breeding_area = None

        hand_dm_count = sum(
            1 for c in p1.hand_cards
            if any('Dark Masters' in t for t in (getattr(c, 'card_traits', []) or []))
        )
        assert hand_dm_count >= 1, "Should have a Dark Masters card in hand"

        # Get and execute the End of Turn effect
        effects = perm_cherrymon.top_card.effect_list(None)
        eot_effects = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(eot_effects) >= 1, "Should have OnEndTurn effect"
        eot = eot_effects[0]

        eot.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm_cherrymon,
        })

        # Step 1: Select a Digimon to delete (pick the sacrifice, not Cherrymon)
        # Find the action for the sacrifice Digimon (slot 1 = action 101)
        sacrifice_action = runner.find_action("Agumon")
        if sacrifice_action is None:
            # Fallback: pick the second field action (slot 1)
            legal = runner.action_mask()
            field_actions = [a for a in legal if a >= 100 and a != 62]
            sacrifice_action = field_actions[-1] if field_actions else legal[-1]
        runner.execute(sacrifice_action)

        # Step 2: Select the Dark Masters Digimon from hand to play
        puppetmon_action = runner.find_action("Puppetmon")
        if puppetmon_action is None:
            legal = runner.action_mask()
            hand_actions = [a for a in legal if a != 62]
            puppetmon_action = hand_actions[0] if hand_actions else legal[0]
        runner.execute(puppetmon_action)

        # Auto-resolve any remaining phases
        runner.auto_resolve()

        # The Dark Masters Digimon should be in breeding area, NOT battle area
        assert p1.breeding_area is not None, \
            "Dark Masters Digimon should have been played to BREEDING area"
        breeding_name = p1.breeding_area.top_card.card_names[0] if p1.breeding_area.top_card else None
        assert breeding_name is not None and "Puppetmon" in breeding_name, \
            f"Breeding area should contain Puppetmon, got: {breeding_name}"

        # Verify it was NOT added to battle area (net field change: -1 from deletion)
        remaining_names = [
            p.top_card.card_names[0] if p.top_card else "?" for p in p1.battle_area
        ]
        puppetmon_in_battle = any("Puppetmon" in n for n in remaining_names)
        assert not puppetmon_in_battle, \
            f"Puppetmon should NOT be in battle area, but found: {remaining_names}"

    def test_end_of_turn_deletes_own_digimon_as_cost(self, debug_runner):
        """The effect should delete 1 own Digimon as cost before playing."""
        deck = _make_deck_with("BT15-050", "BT15-052")
        runner = debug_runner(deck1=deck, deck2=FILLER, initial_memory=10)
        game = runner.game
        p1 = game.player1

        perm_cherrymon = runner.place_on_field(1, ["BT15-050"])
        perm_sacrifice = runner.place_on_field(1, ["ST1-03"])
        runner.inject_card(1, "BT15-052", "hand")
        p1.breeding_area = None

        effects = perm_cherrymon.top_card.effect_list(None)
        eot = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        trash_before = len(p1.trash_cards)

        eot.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm_cherrymon,
        })

        # Step 1: Select the sacrificial Digimon to delete
        sacrifice_action = runner.find_action("Agumon")
        if sacrifice_action is None:
            legal = runner.action_mask()
            field_actions = [a for a in legal if a >= 100 and a != 62]
            sacrifice_action = field_actions[-1] if field_actions else legal[-1]
        runner.execute(sacrifice_action)

        # Step 2: Select the Dark Masters Digimon from hand (or decline)
        runner.auto_resolve()

        # The sacrificial Digimon should have been deleted (sent to trash)
        trash_after = len(p1.trash_cards)
        assert trash_after > trash_before, \
            "A Digimon should have been deleted (sent to trash) as cost"

    def test_end_of_turn_blocked_when_breeding_occupied(self, debug_runner):
        """If breeding area is NOT empty, should not play the Dark Masters Digimon there."""
        deck = _make_deck_with("BT15-050", "BT15-052")
        runner = debug_runner(deck1=deck, deck2=FILLER, initial_memory=10)
        game = runner.game
        p1 = game.player1

        perm_cherrymon = runner.place_on_field(1, ["BT15-050"])
        perm_sacrifice = runner.place_on_field(1, ["ST1-03"])
        runner.inject_card(1, "BT15-052", "hand")

        # Occupy the breeding area
        runner.place_in_breeding(1, ["ST1-03"])
        assert p1.breeding_area is not None, "Breeding should be occupied"

        effects = perm_cherrymon.top_card.effect_list(None)
        eot = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        eot.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm_cherrymon,
        })

        # If the script correctly checks breeding, it should either:
        # a) Not offer the play at all (breeding check before deletion), OR
        # b) Offer deletion but then not offer play (breeding check after deletion)
        # In both cases, resolve all remaining selections
        from engine_py_legacy.engine.data.enums import GamePhase
        while game.current_phase not in (GamePhase.Main, GamePhase.Breeding, GamePhase.End):
            legal = runner.action_mask()
            if not legal:
                break
            # Pick non-decline action if available (to ensure deletion happens)
            non_decline = [a for a in legal if a != 62]
            if non_decline:
                runner.execute(non_decline[0])
            else:
                runner.execute(legal[0])

        # Dark Masters card should NOT have been played to battle area
        puppetmon_in_battle = any(
            p.top_card and "Puppetmon" in p.top_card.card_names[0]
            for p in p1.battle_area
        )
        assert not puppetmon_in_battle, \
            "Puppetmon should NOT be in battle area when breeding is occupied"

        # Breeding area should still have the original occupant, not Puppetmon
        if p1.breeding_area and p1.breeding_area.top_card:
            assert "Puppetmon" not in p1.breeding_area.top_card.card_names[0], \
                "Puppetmon should NOT overwrite the existing breeding area occupant"

    def test_end_of_turn_no_digimon_to_delete(self, debug_runner):
        """If player has no Digimon to delete, effect should not proceed."""
        deck = _make_deck_with("BT15-050", "BT15-052")
        runner = debug_runner(deck1=deck, deck2=FILLER, initial_memory=10)
        game = runner.game
        p1 = game.player1

        # Only place a tamer (not a digimon) — or clear field entirely
        # Clear the battle area completely
        p1.battle_area.clear()
        runner.inject_card(1, "BT15-052", "hand")
        p1.breeding_area = None

        # The effect requires a Digimon to delete
        # We need the effect from a Cherrymon card — but we can still call process manually
        perm_cherrymon = runner.place_on_field(1, ["BT15-050"])
        # Now battle area has only Cherrymon itself

        effects = perm_cherrymon.top_card.effect_list(None)
        eot = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        hand_before = len(p1.hand_cards)

        eot.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm_cherrymon,
        })
        runner.auto_resolve()

        # Even if it deletes itself, the Dark Masters should go to breeding (or nowhere)
        # Key: the Dark Masters card should not end up in battle area
        puppetmon_in_battle = any(
            p.top_card and "Puppetmon" in p.top_card.card_names[0]
            for p in p1.battle_area
        )
        assert not puppetmon_in_battle, \
            "Puppetmon should NOT be in battle area"

    def test_end_of_turn_only_dark_masters_digimon(self, debug_runner):
        """Should only allow playing Digimon with [Dark Masters] trait."""
        deck = _make_deck_with("BT15-050")
        runner = debug_runner(deck1=deck, deck2=FILLER, initial_memory=10)
        game = runner.game
        p1 = game.player1

        perm_cherrymon = runner.place_on_field(1, ["BT15-050"])
        perm_sacrifice = runner.place_on_field(1, ["ST1-03"])
        # Only non-Dark-Masters Digimon in hand
        runner.inject_card(1, "ST1-03", "hand")
        p1.breeding_area = None

        effects = perm_cherrymon.top_card.effect_list(None)
        eot = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        eot.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm_cherrymon,
        })

        # Resolve all selections (pick non-decline to allow deletion)
        from engine_py_legacy.engine.data.enums import GamePhase
        while game.current_phase not in (GamePhase.Main, GamePhase.Breeding, GamePhase.End):
            legal = runner.action_mask()
            if not legal:
                break
            non_decline = [a for a in legal if a != 62]
            if non_decline:
                runner.execute(non_decline[0])
            else:
                runner.execute(legal[0])

        # No Dark Masters in hand, so even if cost is paid, no play happens
        # Breeding should remain empty
        assert p1.breeding_area is None, \
            "Breeding should remain empty when no Dark Masters in hand"

    def test_end_of_turn_plays_without_paying_cost(self, debug_runner):
        """The Dark Masters Digimon should be played for free (no memory cost)."""
        deck = _make_deck_with("BT15-050", "BT15-052")
        runner = debug_runner(deck1=deck, deck2=FILLER, initial_memory=5)
        game = runner.game
        p1 = game.player1

        perm_cherrymon = runner.place_on_field(1, ["BT15-050"])
        perm_sacrifice = runner.place_on_field(1, ["ST1-03"])
        runner.inject_card(1, "BT15-052", "hand")
        p1.breeding_area = None

        memory_before = game.memory

        effects = perm_cherrymon.top_card.effect_list(None)
        eot = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        eot.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm_cherrymon,
        })

        # Step 1: Select a Digimon to delete
        sacrifice_action = runner.find_action("Agumon")
        if sacrifice_action is None:
            legal = runner.action_mask()
            field_actions = [a for a in legal if a >= 100 and a != 62]
            sacrifice_action = field_actions[-1] if field_actions else legal[-1]
        runner.execute(sacrifice_action)

        # Step 2: Select Puppetmon from hand
        puppetmon_action = runner.find_action("Puppetmon")
        if puppetmon_action is None:
            legal = runner.action_mask()
            hand_actions = [a for a in legal if a != 62]
            puppetmon_action = hand_actions[0] if hand_actions else legal[0]
        runner.execute(puppetmon_action)
        runner.auto_resolve()

        # Memory should not decrease (played for free)
        assert game.memory == memory_before, \
            f"Memory should not change (free play). Before={memory_before}, after={game.memory}"

    def test_end_of_turn_condition_requires_own_turn(self, debug_runner):
        """End of Turn condition should only activate on your turn."""
        deck = _make_deck_with("BT15-050")
        runner = debug_runner(deck1=deck, deck2=FILLER, initial_memory=5)
        game = runner.game
        p1 = game.player1

        perm = runner.place_on_field(1, ["BT15-050"])
        effects = perm.top_card.effect_list(None)
        eot = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        # On player 1's turn
        p1.is_my_turn = True
        assert eot.can_use_condition({'player': p1, 'permanent': perm}), \
            "Should be usable on own turn"

        # Not on player 1's turn
        p1.is_my_turn = False
        assert not eot.can_use_condition({'player': p1, 'permanent': perm}), \
            "Should NOT be usable on opponent's turn"
