"""Behavioral tests for BT17-077 Imperialdramon: Paladin Mode (Lv.7 White/Blue ACE).

Card text:
  [Hand] [Counter] <Blast Digivolve>
  [On Play] [When Digivolving] Trash all digivolution cards of all of your
    opponent's Digimon. Then, return all cards from your or your opponent's
    trash to the bottom of the deck. If this effect returned a white level 7
    card, gain 3 memory.
  [When Attacking] By returning 1 of your opponent's Digimon with no
    digivolution cards to the bottom of the deck, unsuspend this Digimon.
  Inherited: Ace Overflow <-5>

  Alt-Digi: From Lv.6 [Imperialdramon] for cost 5
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, CardColor


@pytest.mark.behavioral
class TestBT17077ImperialdramonPaladinMode:
    """Tests for BT17-077 Imperialdramon: Paladin Mode."""

    # ── Alt-Digi: Lv.6 [Imperialdramon] for cost 5 ──────────────────

    def test_alt_digi_attributes(self, debug_runner):
        """Alt-digi effect should specify cost 5 from Imperialdramon."""
        runner = debug_runner(initial_memory=10)
        # Get the card effects directly
        card = runner.inject_card(1, "BT17-077", "hand")
        effects = card.effect_list(None)
        alt_digi = [e for e in effects if hasattr(e, '_alt_digi_cost') and e._alt_digi_cost is not None]
        assert len(alt_digi) == 1, "Should have exactly 1 alt-digi effect"
        assert alt_digi[0]._alt_digi_cost == 5, "Alt-digi cost should be 5"
        assert alt_digi[0]._alt_digi_name == "Imperialdramon", "Alt-digi name should be Imperialdramon"

    # ── Blast Digivolve ──────────────────────────────────────────────

    def test_has_blast_digivolve(self, debug_runner):
        """Should have Blast Digivolve effect."""
        runner = debug_runner(initial_memory=10)
        card = runner.inject_card(1, "BT17-077", "hand")
        effects = card.effect_list(None)
        blast = [e for e in effects if getattr(e, '_is_blast_digivolve', False)]
        assert len(blast) >= 1, "Should have a Blast Digivolve effect"
        assert blast[0].is_counter_effect, "Blast Digivolve should be a counter effect"

    # ── [On Play] Trash digi-cards, return trash, memory check ───────

    def test_on_play_trashes_opponent_digivolution_cards(self, debug_runner):
        """[On Play] should trash ALL digivolution cards from ALL opponent Digimon."""
        runner = debug_runner(initial_memory=15)

        # Place opponent Digimon with digivolution cards (stacked)
        # BT1-025 (WarGreymon Lv.6) on top of BT1-010 (Agumon Lv.3) = 1 digi card
        opp_perm1 = runner.place_on_field(2, ["BT1-010", "BT1-025"])
        # Another stacked opponent Digimon
        opp_perm2 = runner.place_on_field(2, ["BT1-010", "BT1-025"])

        assert len(opp_perm1.card_sources) == 2, "opp_perm1 should have 2 cards in stack"
        assert len(opp_perm2.card_sources) == 2, "opp_perm2 should have 2 cards in stack"

        # Place Imperialdramon on field (simulating On Play)
        perm = runner.place_on_field(1, ["BT17-077"])

        # Manually trigger the On Play effect
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play]
        assert len(on_play) >= 1, "Should have On Play effect"

        ctx = {'player': runner.game.player1, 'game': runner.game}
        on_play[0].on_process_callback(ctx)

        # After trashing digi-cards, each opponent perm should have only 1 card (top card)
        assert len(opp_perm1.card_sources) == 1, (
            f"opp_perm1 should have 1 card after trash. Got {len(opp_perm1.card_sources)}")
        assert len(opp_perm2.card_sources) == 1, (
            f"opp_perm2 should have 1 card after trash. Got {len(opp_perm2.card_sources)}")

        # The trashed cards should be in opponent's trash
        p2_trash_ids = [c.c_entity_base.card_id for c in runner.game.player2.trash_cards]
        assert "BT1-010" in p2_trash_ids, "Trashed digi-card should be in P2 trash"

    def test_on_play_choose_own_trash_returns_to_deck(self, debug_runner):
        """[On Play] Choosing 'Your Trash' (branch 0) should return own trash to deck bottom."""
        runner = debug_runner(initial_memory=15)

        # Put some cards in P1's trash
        runner.inject_card(1, "BT1-010", "trash")
        runner.inject_card(1, "BT1-010", "trash")
        p1_trash_before = len(runner.game.player1.trash_cards)
        p1_library_before = len(runner.game.player1.library_cards)

        # Place Imperialdramon on field
        perm = runner.place_on_field(1, ["BT17-077"])
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play]
        assert len(on_play) >= 1

        # Trigger the effect - it will call effect_choose_branch
        # We need to intercept the branch selection
        branch_selected = []
        original_choose = runner.game.effect_choose_branch

        def mock_choose(player, num, callback, **kwargs):
            branch_selected.append(0)  # Choose "Your Trash"
            callback(0)

        runner.game.effect_choose_branch = mock_choose
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play[0].on_process_callback(ctx)
        finally:
            runner.game.effect_choose_branch = original_choose

        # P1 trash should be empty (returned to deck)
        assert len(runner.game.player1.trash_cards) == 0, (
            f"P1 trash should be empty after returning. Got {len(runner.game.player1.trash_cards)}")
        # P1 library should have grown
        assert len(runner.game.player1.library_cards) > p1_library_before, (
            "P1 library should have more cards after trash returned")

    def test_on_play_choose_opponent_trash_returns_to_deck(self, debug_runner):
        """[On Play] Choosing 'Opponent's Trash' (branch 1) should return opp trash to deck bottom."""
        runner = debug_runner(initial_memory=15)

        # Put some cards in P2's trash
        runner.inject_card(2, "BT1-010", "trash")
        runner.inject_card(2, "BT1-010", "trash")
        p2_library_before = len(runner.game.player2.library_cards)

        perm = runner.place_on_field(1, ["BT17-077"])
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play]

        original_choose = runner.game.effect_choose_branch

        def mock_choose(player, num, callback, **kwargs):
            callback(1)  # Choose "Opponent's Trash"

        runner.game.effect_choose_branch = mock_choose
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play[0].on_process_callback(ctx)
        finally:
            runner.game.effect_choose_branch = original_choose

        assert len(runner.game.player2.trash_cards) == 0, (
            f"P2 trash should be empty after returning. Got {len(runner.game.player2.trash_cards)}")
        assert len(runner.game.player2.library_cards) > p2_library_before, (
            "P2 library should have more cards after trash returned")

    def test_on_play_gain_3_memory_if_white_lv7_returned(self, debug_runner):
        """[On Play] If returned trash contained a white Lv.7 card, gain 3 memory."""
        runner = debug_runner(initial_memory=5)

        # Put a white Lv.7 card in P1's trash (BT17-077 itself is White Lv.7)
        runner.inject_card(1, "BT17-077", "trash")

        perm = runner.place_on_field(1, ["BT17-077"])
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play]

        memory_before = runner.game.memory
        original_choose = runner.game.effect_choose_branch

        def mock_choose(player, num, callback, **kwargs):
            callback(0)  # Choose "Your Trash" (which has the white Lv.7)

        runner.game.effect_choose_branch = mock_choose
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play[0].on_process_callback(ctx)
        finally:
            runner.game.effect_choose_branch = original_choose

        # Should gain 3 memory
        assert runner.game.memory == memory_before + 3, (
            f"Should gain 3 memory from white Lv.7 return. "
            f"Before: {memory_before}, After: {runner.game.memory}")

    def test_on_play_no_memory_if_no_white_lv7(self, debug_runner):
        """[On Play] If returned trash has no white Lv.7, do NOT gain memory."""
        runner = debug_runner(initial_memory=5)

        # Put non-white, non-Lv.7 cards in trash
        runner.inject_card(1, "BT1-010", "trash")  # Red Lv.3 Agumon

        perm = runner.place_on_field(1, ["BT17-077"])
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play]

        memory_before = runner.game.memory
        original_choose = runner.game.effect_choose_branch

        def mock_choose(player, num, callback, **kwargs):
            callback(0)

        runner.game.effect_choose_branch = mock_choose
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play[0].on_process_callback(ctx)
        finally:
            runner.game.effect_choose_branch = original_choose

        assert runner.game.memory == memory_before, (
            f"Should NOT gain memory without white Lv.7. "
            f"Before: {memory_before}, After: {runner.game.memory}")

    # ── [When Digivolving] Same as On Play ───────────────────────────

    def test_when_digivolving_effect_exists(self, debug_runner):
        """Should have a When Digivolving effect with same behavior as On Play."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT17-077"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving]
        assert len(when_digi) >= 1, "Should have When Digivolving effect"

    def test_when_digivolving_trashes_digi_cards(self, debug_runner):
        """[When Digivolving] should also trash all opponent digi-cards."""
        runner = debug_runner(initial_memory=15)

        opp_perm = runner.place_on_field(2, ["BT1-010", "BT1-025"])
        assert len(opp_perm.card_sources) == 2

        perm = runner.place_on_field(1, ["BT17-077"])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving]
        assert len(when_digi) >= 1

        original_choose = runner.game.effect_choose_branch

        def mock_choose(player, num, callback, **kwargs):
            callback(1)

        runner.game.effect_choose_branch = mock_choose
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            when_digi[0].on_process_callback(ctx)
        finally:
            runner.game.effect_choose_branch = original_choose

        assert len(opp_perm.card_sources) == 1, (
            f"When Digivolving should trash digi-cards too. Got {len(opp_perm.card_sources)}")

    # ── [When Attacking] Return 0-digi-card Digimon, unsuspend self ──

    def test_when_attacking_effect_exists(self, debug_runner):
        """Should have When Attacking effect with optional flag."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT17-077"])
        card = perm.top_card
        effects = card.effect_list(None)
        on_attack = [e for e in effects if e.is_on_attack]
        assert len(on_attack) >= 1, "Should have When Attacking effect"
        assert on_attack[0].is_optional, "When Attacking should be optional"

    def test_when_attacking_condition_requires_no_digi_card_target(self, debug_runner):
        """When Attacking condition should require an opponent Digimon with 0 digi-cards."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT17-077"])
        card = perm.top_card
        effects = card.effect_list(None)
        on_attack = [e for e in effects if e.is_on_attack][0]

        # No opponent Digimon at all - condition should fail
        ctx = {'attacker': perm, 'permanent': perm}
        assert not on_attack.can_use_condition(ctx), (
            "Should fail with no opponent Digimon")

        # Opponent has Digimon with digivolution cards (2 in stack) - should fail
        opp_stacked = runner.place_on_field(2, ["BT1-010", "BT1-025"])
        assert not on_attack.can_use_condition(ctx), (
            "Should fail when all opponent Digimon have digi-cards")

        # Add opponent Digimon with NO digivolution cards (just 1 card in stack)
        opp_bare = runner.place_on_field(2, ["BT1-010"])
        assert on_attack.can_use_condition(ctx), (
            "Should pass when opponent has Digimon with 0 digi-cards")

    def test_when_attacking_bottom_decks_and_unsuspends(self, debug_runner):
        """When Attacking should bottom-deck the selected Digimon and unsuspend self."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT17-077"], is_suspended=True)
        assert perm.is_suspended, "Should start suspended"

        # Place bare opponent Digimon (no digi-cards)
        opp_perm = runner.place_on_field(2, ["BT1-010"])
        assert len(opp_perm.card_sources) == 1, "Should have 1 card (no digi-cards)"

        card = perm.top_card
        effects = card.effect_list(None)
        on_attack = [e for e in effects if e.is_on_attack][0]

        # Mock the opponent selection to pick the first valid target
        original_select = runner.game.effect_select_opponent_permanent

        def mock_select(player, callback, filter_fn=None, is_optional=False, **kwargs):
            # Find first valid target
            enemy = player.enemy
            for p in enemy.battle_area:
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return

        runner.game.effect_select_opponent_permanent = mock_select
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_attack.on_process_callback(ctx)
        finally:
            runner.game.effect_select_opponent_permanent = original_select

        # opp_perm should be gone from field
        assert opp_perm not in runner.game.player2.battle_area, (
            "Opponent Digimon should be bottom-decked")
        # Self should be unsuspended
        assert not perm.is_suspended, "Should unsuspend after bottom-decking"

    def test_when_attacking_condition_only_this_digimon(self, debug_runner):
        """When Attacking should only fire for THIS Digimon attacking, not others."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT17-077"])
        other_perm = runner.place_on_field(1, ["BT1-025"])
        runner.place_on_field(2, ["BT1-010"])  # bare opp Digimon

        card = perm.top_card
        effects = card.effect_list(None)
        on_attack = [e for e in effects if e.is_on_attack][0]

        # With other_perm as attacker, should fail
        ctx = {'attacker': other_perm, 'permanent': other_perm}
        assert not on_attack.can_use_condition(ctx), (
            "Should NOT trigger for a different Digimon attacking")

        # With self as attacker, should pass
        ctx = {'attacker': perm, 'permanent': perm}
        assert on_attack.can_use_condition(ctx), (
            "Should trigger for this Digimon attacking")

    # ── Ace Overflow -5 ──────────────────────────────────────────────

    def test_ace_overflow_inherited(self, debug_runner):
        """Should have Ace Overflow -5 as inherited effect."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT17-077"])
        card = perm.top_card
        assert card.c_entity_base.is_ace, "BT17-077 should be an ACE card"
        assert card.c_entity_base.ace_overflow_cost == 5, (
            f"Ace overflow should be 5, got {card.c_entity_base.ace_overflow_cost}")

    # ── On Play with trashed digi-cards going to opp trash ───────────

    def test_trashed_digi_cards_go_to_opponent_trash(self, debug_runner):
        """Step 1 trashes digi-cards to OPPONENT's trash (they own the Digimon)."""
        runner = debug_runner(initial_memory=15)

        # Clear P2 trash first
        runner.game.player2.trash_cards.clear()

        # Place opponent Digimon with 2 digi-cards: 3 cards in stack
        opp_perm = runner.place_on_field(2, ["BT1-010", "BT1-010", "BT1-025"])
        assert len(opp_perm.card_sources) == 3

        perm = runner.place_on_field(1, ["BT17-077"])
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play]

        original_choose = runner.game.effect_choose_branch

        def mock_choose(player, num, callback, **kwargs):
            callback(1)  # Choose opponent trash

        runner.game.effect_choose_branch = mock_choose
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play[0].on_process_callback(ctx)
        finally:
            runner.game.effect_choose_branch = original_choose

        # After trash step, 2 digi-cards should be in P2 trash
        # Then the "return trash" step should have returned them to P2's deck
        # So P2 trash should be 0 (we chose to return opponent's trash)
        assert len(runner.game.player2.trash_cards) == 0, (
            f"P2 trash should be empty after returning. Got {len(runner.game.player2.trash_cards)}")

    def test_branch_choice_is_presented(self, debug_runner):
        """The effect should call effect_choose_branch with 2 options."""
        runner = debug_runner(initial_memory=15)

        perm = runner.place_on_field(1, ["BT17-077"])
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play]

        branch_calls = []
        original_choose = runner.game.effect_choose_branch

        def mock_choose(player, num, callback, branch_labels=None, **kwargs):
            branch_calls.append({
                'num_choices': num,
                'labels': branch_labels,
            })
            callback(0)

        runner.game.effect_choose_branch = mock_choose
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play[0].on_process_callback(ctx)
        finally:
            runner.game.effect_choose_branch = original_choose

        assert len(branch_calls) == 1, "Should call effect_choose_branch once"
        assert branch_calls[0]['num_choices'] == 2, "Should have 2 branch choices"
        assert branch_calls[0]['labels'] is not None, "Should have branch labels"
        assert len(branch_calls[0]['labels']) == 2, "Should have 2 labels"
