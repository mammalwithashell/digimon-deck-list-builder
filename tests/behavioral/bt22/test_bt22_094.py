"""Behavioral tests for BT22-094 Yuugo Kamishiro (Tamer White, Cost 3, Traits: CS).

Card text:
  [On Play] Reveal the top 3 cards of your deck. Add 1 card with the [CS] trait
  among them to the hand. Return the rest to the bottom of the deck.
  [Your Turn] When any of your Digimon or Tamers with the [CS] trait would be
  played, by returning this Tamer to the bottom of the deck, reduce the play
  cost by 2.
  [Security] Play this card without paying the cost.
"""

import pytest


# BT22-008 Agumon: Red Lv.3 Digimon, cost 3, [Reptile, CS]
CS_DIGIMON = "BT22-008"
# BT22-094 Yuugo Kamishiro: White Tamer, cost 3, [CS]
YUUGO = "BT22-094"
# ST1-03 Agumon: Red Lv.3 Digimon, cost 3, [Reptile] — NO CS trait
NON_CS_DIGIMON = "ST1-03"
# BT22-013 WarGreymon: Red Lv.6, cost 12, [Dragonkin, CS]
CS_DIGIMON_EXPENSIVE = "BT22-013"
# BT22-090 Rie Kishibe: Tamer, cost 3, [CS] (has Start-of-main effect, fine)
CS_TAMER = "BT22-090"


@pytest.mark.behavioral
class TestBT22094OnPlay:
    """Tests for [On Play] Reveal top 3, add 1 [CS] card to hand, rest to bottom."""

    def test_on_play_reveals_and_adds_cs_card(self, debug_runner):
        """Playing Yuugo reveals top 3; selecting a CS card adds it to hand."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        # Stack deck: CS card, non-CS, non-CS (library_top inserts at 0)
        runner.inject_card(1, CS_DIGIMON, "library_top")
        runner.inject_card(1, NON_CS_DIGIMON, "library_top")
        runner.inject_card(1, NON_CS_DIGIMON, "library_top")
        # Deck top: NON_CS, NON_CS, CS, ...

        runner.inject_card(1, YUUGO, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Yuugo")
        if action is None:
            action = runner.find_action("BT22-094")
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        # The CS card should be in hand
        hand_ids = [
            c.c_entity_base.card_id for c in player.hand_cards if c.c_entity_base
        ]
        assert CS_DIGIMON in hand_ids, (
            f"Should have added CS card to hand. Hand: {hand_ids}"
        )

    def test_on_play_no_cs_in_revealed_returns_all_to_bottom(self, debug_runner):
        """When no CS cards are in the top 3, all revealed cards go to bottom."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        # Stack deck: 3 non-CS cards on top
        runner.inject_card(1, NON_CS_DIGIMON, "library_top")
        runner.inject_card(1, NON_CS_DIGIMON, "library_top")
        runner.inject_card(1, NON_CS_DIGIMON, "library_top")

        lib_before = len(player.library_cards)

        runner.inject_card(1, YUUGO, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Yuugo")
        if action is None:
            action = runner.find_action("BT22-094")
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        # Library should have same card count — 3 revealed, 3 returned to bottom
        assert len(player.library_cards) == lib_before, (
            f"All 3 revealed cards should have been returned to bottom. "
            f"Before: {lib_before}, After: {len(player.library_cards)}"
        )

    def test_on_play_non_cs_revealed_cards_go_to_bottom_of_deck(self, debug_runner):
        """Non-CS revealed cards are placed at the bottom of the deck (not top)."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        # Clear library to start clean
        player.library_cards.clear()
        # Push a distinct filler card to the bottom FIRST, so the library
        # becomes [filler]. Then push the top cards.
        runner.inject_card(1, "BT22-017", "library_top")  # Gabumon (CS) - as filler at future bottom
        # Now library is [Gabumon]. We'll consider this as "below" our 3 reveals.
        # Push 3 cards to top (in reverse order so first-inserted is at bottom of stack).
        runner.inject_card(1, NON_CS_DIGIMON, "library_top")  # bottom of reveal
        runner.inject_card(1, NON_CS_DIGIMON, "library_top")  # middle of reveal
        runner.inject_card(1, CS_DIGIMON, "library_top")     # top of reveal
        # Library now: [CS_DIGIMON, NON_CS, NON_CS, BT22-017]

        runner.inject_card(1, YUUGO, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Yuugo")
        if action is None:
            action = runner.find_action("BT22-094")
        runner.execute(action)
        runner.auto_resolve()

        # After effect resolves: CS_DIGIMON in hand; the 2 NON_CS cards at bottom
        # of deck, after the filler. Expected library: [BT22-017, NON_CS, NON_CS]
        lib_ids = [c.c_entity_base.card_id for c in player.library_cards if c.c_entity_base]
        # The filler Gabumon should still be at position 0 (top side now), and
        # both NON_CS cards should be somewhere after it.
        assert "BT22-017" in lib_ids
        # Non-CS revealed cards should be at bottom (positions >= filler position)
        filler_idx = lib_ids.index("BT22-017")
        non_cs_indices = [i for i, cid in enumerate(lib_ids) if cid == NON_CS_DIGIMON]
        assert len(non_cs_indices) == 2, (
            f"Expected 2 non-CS cards in library, got {non_cs_indices}. Lib: {lib_ids}"
        )
        # Non-CS cards should be after the filler (i.e., deeper in deck)
        assert all(i > filler_idx for i in non_cs_indices), (
            f"Non-CS cards should be at bottom. Filler idx={filler_idx}, "
            f"non-CS indices={non_cs_indices}, lib={lib_ids}"
        )


@pytest.mark.behavioral
class TestBT22094CostReduction:
    """Tests for [Your Turn] BeforePayCost: reduce CS Digimon/Tamer play cost by 2."""

    def test_cost_reduction_for_cs_digimon(self, debug_runner):
        """Playing a CS Digimon with Yuugo on field costs 2 less."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place Yuugo on field (already played)
        runner.place_on_field(1, [YUUGO])

        # Put a CS Digimon (cost 3) in hand
        runner.inject_card(1, CS_DIGIMON, "hand")
        runner.set_phase("Main")

        mem_before = game.memory
        play = runner.find_action("Agumon")
        if play is None:
            play = runner.find_action(CS_DIGIMON)
        assert play is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # CS Digimon cost 3, reduced by 2 = 1. Memory: 5 - 1 = 4
        assert any(s.card_id == CS_DIGIMON for s in snap.p1_field), (
            "CS Digimon should be on the field"
        )
        # Yuugo should have been returned to deck bottom
        assert not any(s.card_id == YUUGO for s in snap.p1_field), (
            "Yuugo should have been returned to deck bottom"
        )
        # Memory reduction: cost 3 - 2 = 1 memory spent
        assert snap.memory == mem_before - 1, (
            f"Cost should be reduced by 2 (3-2=1). Before={mem_before}, After={snap.memory}"
        )

    def test_cost_reduction_for_cs_tamer(self, debug_runner):
        """Playing a CS Tamer (not just Digimon) also triggers the reduction."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        runner.place_on_field(1, [YUUGO])
        runner.inject_card(1, CS_TAMER, "hand")
        runner.set_phase("Main")

        mem_before = game.memory
        play = runner.find_action("Rie Kishibe")
        if play is None:
            play = runner.find_action(CS_TAMER)
        assert play is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert any(s.card_id == CS_TAMER for s in snap.p1_field), (
            "CS Tamer should be on field"
        )
        assert not any(s.card_id == YUUGO for s in snap.p1_field), (
            "Yuugo should have been returned to deck bottom"
        )
        # Cost 3 - 2 = 1 memory spent
        assert snap.memory == mem_before - 1, (
            f"CS Tamer cost 3 - 2 = 1. Before={mem_before}, After={snap.memory}"
        )

    def test_yuugo_returns_to_deck_bottom(self, debug_runner):
        """Yuugo should end up at the bottom of the deck (not in trash)."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [YUUGO])
        runner.inject_card(1, CS_DIGIMON, "hand")
        runner.set_phase("Main")

        lib_before = len(player.library_cards)
        trash_before = len(player.trash_cards)

        play = runner.find_action("Agumon")
        if play is None:
            play = runner.find_action(CS_DIGIMON)
        runner.execute(play)
        runner.auto_resolve()

        # Library should grow by 1 (Yuugo added to bottom)
        assert len(player.library_cards) == lib_before + 1, (
            f"Library should have 1 more card. Before={lib_before}, After={len(player.library_cards)}"
        )
        # Trash should NOT grow — Yuugo is not trashed
        assert len(player.trash_cards) == trash_before, (
            f"Trash should not change. Before={trash_before}, After={len(player.trash_cards)}"
        )
        # Yuugo should be at the bottom of the library (last position)
        bottom_card = player.library_cards[-1]
        assert bottom_card.c_entity_base and bottom_card.c_entity_base.card_id == YUUGO, (
            f"Yuugo should be at the bottom. Bottom card: "
            f"{bottom_card.c_entity_base.card_id if bottom_card.c_entity_base else 'unknown'}"
        )

    def test_no_cost_reduction_for_non_cs_digimon(self, debug_runner):
        """Playing a non-CS Digimon should NOT get cost reduction from Yuugo."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place Yuugo on field
        runner.place_on_field(1, [YUUGO])

        # Clear hand so no other playable cards interfere
        runner.clear_zone(1, "hand")

        # Put a non-CS Digimon (cost 3) in hand
        runner.inject_card(1, NON_CS_DIGIMON, "hand")
        runner.set_phase("Main")

        mem_before = game.memory
        # Use specific card name from ST1-03 (Agumon from Starter Deck)
        play = runner.find_action("ST1-03")
        if play is None:
            play = runner.find_action("Play Agumon")
        if play is None:
            plays = runner.find_actions("Play")
            play = next(iter(plays)) if plays else None
        assert play is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Full cost 3 — no reduction
        assert snap.memory == mem_before - 3, (
            f"Non-CS Digimon should pay full cost 3. Before={mem_before}, After={snap.memory}"
        )
        # Yuugo should still be on field (not returned to deck)
        assert any(s.card_id == YUUGO for s in snap.p1_field), (
            "Yuugo should still be on field when non-CS card is played"
        )

    def test_leak_guard_self_in_hand_cannot_reduce_itself(self, debug_runner):
        """A hand-only Yuugo (no field copy) must pay full cost 3.

        Verifies the field-presence check acts as an implicit self-leak guard:
        when Yuugo is in hand and not on field, its own BeforePayCost effect
        cannot reduce its own play cost.
        """
        runner = debug_runner(initial_memory=5)
        game = runner.game

        runner.inject_card(1, YUUGO, "hand")
        # Stack deck for the reveal the card will trigger on enter
        runner.inject_card(1, NON_CS_DIGIMON, "library_top")
        runner.inject_card(1, NON_CS_DIGIMON, "library_top")
        runner.inject_card(1, NON_CS_DIGIMON, "library_top")

        runner.set_phase("Main")
        mem_before = game.memory

        play = runner.find_action("Yuugo")
        if play is None:
            play = runner.find_action(YUUGO)
        assert play is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Full cost 3 spent (no field Yuugo to offer a reduction)
        assert snap.memory == mem_before - 3, (
            f"Hand-only Yuugo should pay full cost 3. "
            f"Before={mem_before}, After={snap.memory}"
        )
        # The new Yuugo should be on field
        yuugo_on_field = any(s.card_id == YUUGO for s in snap.p1_field)
        assert yuugo_on_field, "Yuugo should be on field after play"

    def test_field_yuugo_can_reduce_another_yuugo_play(self, debug_runner):
        """Per card text, a field Yuugo DOES offer its reduction for another CS
        Tamer (including another Yuugo) being played. The leak guard is only
        object-identity based: the SAME CardSource cannot reduce its own cost,
        but a separate copy can."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        runner.place_on_field(1, [YUUGO])
        runner.inject_card(1, YUUGO, "hand")

        # Deck stack for the new Yuugo's on-play reveal
        runner.inject_card(1, NON_CS_DIGIMON, "library_top")
        runner.inject_card(1, NON_CS_DIGIMON, "library_top")
        runner.inject_card(1, NON_CS_DIGIMON, "library_top")

        runner.set_phase("Main")
        mem_before = game.memory

        play = runner.find_action("Yuugo")
        if play is None:
            play = runner.find_action(YUUGO)
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # The field Yuugo offered its reduction (returned to deck), so the new
        # Yuugo pays 3 - 2 = 1. Memory: 5 - 1 = 4.
        assert snap.memory == mem_before - 1, (
            f"New Yuugo should pay 1 (cost 3 - 2 reduction from field Yuugo). "
            f"Before={mem_before}, After={snap.memory}"
        )
        # Only one Yuugo on field (the newly played one)
        yuugo_count = sum(1 for s in snap.p1_field if s.card_id == YUUGO)
        assert yuugo_count == 1, (
            f"Expected 1 Yuugo on field (field copy returned to deck). Field: "
            f"{[(s.card_id, s.card_name) for s in snap.p1_field]}"
        )

    def test_no_reduction_for_opponent_plays(self, debug_runner):
        """Yuugo's cost reduction must not apply to opponent's CS plays."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place Yuugo on P1's field
        runner.place_on_field(1, [YUUGO])

        # Inject CS Digimon into P1's hand (our side) to establish baseline —
        # but we actually want to test that when P2 plays a CS card on their
        # turn, Yuugo's reduction doesn't apply. We test this at the condition
        # level since cross-turn play-action orchestration is heavy.
        runner.inject_card(2, CS_DIGIMON, "hand")

        # Retrieve Yuugo's BeforePayCost effect and invoke its condition with
        # a played CS card owned by opponent (player2). The condition must
        # return False.
        yuugo_perm = None
        for perm in game.player1.battle_area:
            if (perm.top_card and perm.top_card.c_entity_base
                    and perm.top_card.c_entity_base.card_id == YUUGO):
                yuugo_perm = perm
                break
        assert yuugo_perm is not None

        from digimon_gym.engine.data.enums import EffectTiming
        before_pay_effects = [
            e for e in yuugo_perm.effect_list(EffectTiming.NoTiming)
            if getattr(e, 'timing', None) == EffectTiming.BeforePayCost
        ]
        assert len(before_pay_effects) > 0

        # Get the opponent's CS card from their hand
        opp_cs_card = None
        for c in game.player2.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == CS_DIGIMON:
                opp_cs_card = c
                break
        assert opp_cs_card is not None

        # Build context as if calculating cost for P2's play
        ctx = {
            'game': game,
            'player': game.player2,
            'permanent': yuugo_perm,
            'card_source': opp_cs_card,
            'played_card': opp_cs_card,
            'source_zone': 'hand',
            'free': False,
        }
        result = before_pay_effects[0].can_use_condition(ctx)
        assert not result, (
            "Yuugo's cost reduction must not apply to opponent's CS plays"
        )

    def test_cost_reduction_is_optional_flag(self, debug_runner):
        """The BeforePayCost effect must be marked as optional (intent)."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        runner.place_on_field(1, [YUUGO])

        yuugo_perm = None
        for perm in game.player1.battle_area:
            if (perm.top_card and perm.top_card.c_entity_base
                    and perm.top_card.c_entity_base.card_id == YUUGO):
                yuugo_perm = perm
                break
        assert yuugo_perm is not None

        from digimon_gym.engine.data.enums import EffectTiming
        before_pay_effects = [
            e for e in yuugo_perm.effect_list(EffectTiming.NoTiming)
            if getattr(e, 'timing', None) == EffectTiming.BeforePayCost
        ]
        assert len(before_pay_effects) > 0, "Should have a BeforePayCost effect"
        assert before_pay_effects[0].is_optional, (
            "BeforePayCost effect should be optional"
        )
        # Also assert cost_reduction value
        assert before_pay_effects[0].cost_reduction == 2, (
            f"cost_reduction should be 2, got {before_pay_effects[0].cost_reduction}"
        )

    def test_cost_reduction_only_on_your_turn(self, debug_runner):
        """Cost reduction should only work on your turn."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        runner.place_on_field(1, [YUUGO])

        yuugo_perm = None
        for perm in game.player1.battle_area:
            if (perm.top_card and perm.top_card.c_entity_base
                    and perm.top_card.c_entity_base.card_id == YUUGO):
                yuugo_perm = perm
                break
        assert yuugo_perm is not None

        from digimon_gym.engine.data.enums import EffectTiming
        before_pay_effects = [
            e for e in yuugo_perm.effect_list(EffectTiming.NoTiming)
            if getattr(e, 'timing', None) == EffectTiming.BeforePayCost
        ]
        assert len(before_pay_effects) > 0

        # Get a CS card to use as card_source
        cs_card = runner.inject_card(1, CS_DIGIMON, "hand")

        # Simulate not being the owner's turn
        game.player1.is_my_turn = False
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': yuugo_perm,
            'card_source': cs_card,
            'played_card': cs_card,
        }
        result = before_pay_effects[0].can_use_condition(ctx)
        assert not result, "BeforePayCost should NOT activate when it's not your turn"

        # Restore and verify the condition passes on your turn
        game.player1.is_my_turn = True
        result = before_pay_effects[0].can_use_condition(ctx)
        assert result, "BeforePayCost should activate when it IS your turn (same ctx)"


@pytest.mark.behavioral
class TestBT22094Security:
    """Tests for [Security] Play this card without paying the cost."""

    def test_security_effect_plays_tamer(self, debug_runner):
        """When revealed from security, Yuugo should be played onto the field for free."""
        # P2 is first player so they can attack P1's security immediately
        runner = debug_runner(initial_memory=5, first_player=2)
        game = runner.game

        # Clear P1's security and place Yuugo as the ONLY security card
        game.player1.security_cards.clear()
        runner.inject_card(1, YUUGO, "security_top")

        # Give Yuugo a deck top so its on-play reveal has cards to work with
        runner.inject_card(1, NON_CS_DIGIMON, "library_top")
        runner.inject_card(1, NON_CS_DIGIMON, "library_top")
        runner.inject_card(1, NON_CS_DIGIMON, "library_top")

        # Place a strong attacker on P2's field (no summoning sickness)
        # BT1-021 MetalGreymon: Lv.5, non-ADVENTURE
        runner.place_on_field(2, ["BT1-021"], turn_played=-1)

        runner.set_phase("Main")

        attack = runner.find_action("Attack player with MetalGreymon")
        assert attack is not None, (
            f"P2 should be able to attack. Actions: {runner.actions()}"
        )
        runner.execute(attack)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Yuugo should be on P1's field after security trigger
        tamer_on_field = any(s.card_id == YUUGO for s in snap_after.p1_field)
        assert tamer_on_field, (
            f"BT22-094 should be played from security. "
            f"P1 field: {[(s.card_id, s.card_name) for s in snap_after.p1_field]}"
        )
        # Yuugo should NOT be in P1's trash (not trashed by security attack)
        p1_trash_ids = snap_after.p1_trash
        assert YUUGO not in p1_trash_ids, (
            f"Yuugo should not be in trash after being played from security. "
            f"Trash: {p1_trash_ids}"
        )
