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

        hand_before = len(player.hand_cards)
        lib_before = len(player.library_cards)

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

        # Library should have same card count (3 removed from top, 3 returned to bottom)
        # minus zero net change since no card was added to hand
        # (Yuugo was played from hand and is now on field, not in library)
        assert len(player.library_cards) == lib_before, (
            f"All 3 revealed cards should have been returned to bottom. "
            f"Before: {lib_before}, After: {len(player.library_cards)}"
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
            # Find any play action for the only card in hand
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

    def test_cost_reduction_is_optional(self, debug_runner):
        """The cost reduction is optional (player can decline)."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place Yuugo on field
        runner.place_on_field(1, [YUUGO])

        # Put a CS Digimon in hand
        runner.inject_card(1, CS_DIGIMON, "hand")
        runner.set_phase("Main")

        # The effect should be marked as optional in the script
        # Check that the effect has is_optional = True
        yuugo_perm = None
        for perm in game.player1.battle_area:
            if perm.top_card and perm.top_card.c_entity_base and perm.top_card.c_entity_base.card_id == YUUGO:
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

    def test_cost_reduction_only_on_your_turn(self, debug_runner):
        """Cost reduction should only work on your turn."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place Yuugo on field for player 1
        runner.place_on_field(1, [YUUGO])

        # Verify the BeforePayCost condition checks for is_my_turn
        yuugo_perm = None
        for perm in game.player1.battle_area:
            if perm.top_card and perm.top_card.c_entity_base and perm.top_card.c_entity_base.card_id == YUUGO:
                yuugo_perm = perm
                break
        assert yuugo_perm is not None

        from digimon_gym.engine.data.enums import EffectTiming
        before_pay_effects = [
            e for e in yuugo_perm.effect_list(EffectTiming.NoTiming)
            if getattr(e, 'timing', None) == EffectTiming.BeforePayCost
        ]
        assert len(before_pay_effects) > 0

        # Simulate not being the owner's turn
        game.player1.is_my_turn = False
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': yuugo_perm,
            'card_source': runner.inject_card(1, CS_DIGIMON, "hand"),
            'played_card': runner.inject_card(1, CS_DIGIMON, "hand"),
        }
        result = before_pay_effects[0].can_use_condition(ctx)
        assert not result, "BeforePayCost should NOT activate when it's not your turn"

        # Restore
        game.player1.is_my_turn = True
