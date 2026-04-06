"""Behavioral tests for BT3-093 Davis Motomiya (Tamer, Blue, Cost 4).

Card text:
  [Start of Your Turn] If you have 2 or less memory, set it to 3.
  [On Play] Reveal the top 3 cards of your deck. Add 1 blue and 1 green
    Digimon card among them to your hand. Place the remaining cards at
    the bottom of your deck in any order.
  Security: Play this card without paying the cost.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, CardColor


@pytest.mark.behavioral
class TestBT3093DavisMotomiya:
    """Tests for BT3-093 Davis Motomiya."""

    # ── [Start of Your Turn] Set memory to 3 ─────────────────────────

    def test_start_turn_effect_exists(self, debug_runner):
        """Should have OnStartTurn effect."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, ["BT3-093"])
        card = perm.top_card
        effects = card.effect_list(None)
        start_turn = [e for e in effects if e.timing == EffectTiming.OnStartTurn]
        assert len(start_turn) >= 1, "Should have OnStartTurn effect"

    def test_start_turn_sets_memory_to_3_when_at_0(self, debug_runner):
        """When memory is 0 (<=2), should set to 3."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, ["BT3-093"])
        card = perm.top_card
        effects = card.effect_list(None)
        start_turn = [e for e in effects if e.timing == EffectTiming.OnStartTurn][0]

        runner.game.memory = 0
        ctx = {'player': runner.game.player1, 'game': runner.game}
        start_turn.on_process_callback(ctx)
        assert runner.game.memory == 3, f"Memory should be 3, got {runner.game.memory}"

    def test_start_turn_sets_memory_to_3_when_at_2(self, debug_runner):
        """When memory is exactly 2 (<=2), should set to 3."""
        runner = debug_runner(initial_memory=2)
        perm = runner.place_on_field(1, ["BT3-093"])
        card = perm.top_card
        effects = card.effect_list(None)
        start_turn = [e for e in effects if e.timing == EffectTiming.OnStartTurn][0]

        runner.game.memory = 2
        ctx = {'player': runner.game.player1, 'game': runner.game}
        start_turn.on_process_callback(ctx)
        assert runner.game.memory == 3, f"Memory should be 3, got {runner.game.memory}"

    def test_start_turn_does_nothing_when_at_3(self, debug_runner):
        """When memory is 3 (>2), should NOT change."""
        runner = debug_runner(initial_memory=3)
        perm = runner.place_on_field(1, ["BT3-093"])
        card = perm.top_card
        effects = card.effect_list(None)
        start_turn = [e for e in effects if e.timing == EffectTiming.OnStartTurn][0]

        runner.game.memory = 3
        ctx = {'player': runner.game.player1, 'game': runner.game}
        start_turn.on_process_callback(ctx)
        assert runner.game.memory == 3, f"Memory should stay 3, got {runner.game.memory}"

    def test_start_turn_does_nothing_when_at_5(self, debug_runner):
        """When memory is 5 (>2), should NOT change."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT3-093"])
        card = perm.top_card
        effects = card.effect_list(None)
        start_turn = [e for e in effects if e.timing == EffectTiming.OnStartTurn][0]

        runner.game.memory = 5
        ctx = {'player': runner.game.player1, 'game': runner.game}
        start_turn.on_process_callback(ctx)
        assert runner.game.memory == 5, f"Memory should stay 5, got {runner.game.memory}"

    def test_start_turn_sets_memory_when_negative(self, debug_runner):
        """When memory is negative (<=2), should set to 3."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, ["BT3-093"])
        card = perm.top_card
        effects = card.effect_list(None)
        start_turn = [e for e in effects if e.timing == EffectTiming.OnStartTurn][0]

        runner.game.memory = -2
        ctx = {'player': runner.game.player1, 'game': runner.game}
        start_turn.on_process_callback(ctx)
        assert runner.game.memory == 3, (
            f"Memory should be set to 3 from -2, got {runner.game.memory}")

    def test_start_turn_condition_requires_own_turn(self, debug_runner):
        """Condition should only pass on owner's turn."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, ["BT3-093"])
        card = perm.top_card
        effects = card.effect_list(None)
        start_turn = [e for e in effects if e.timing == EffectTiming.OnStartTurn][0]

        # P1's turn
        assert start_turn.can_use_condition({}), "Should pass on P1's turn"

        # Switch to P2's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True
        assert not start_turn.can_use_condition({}), "Should fail on P2's turn"

    def test_start_turn_condition_requires_on_field(self, debug_runner):
        """Condition should fail if tamer is not on the field."""
        runner = debug_runner(initial_memory=0)
        card = runner.inject_card(1, "BT3-093", "hand")
        effects = card.effect_list(None)
        start_turn = [e for e in effects if e.timing == EffectTiming.OnStartTurn][0]

        # Card is in hand, not on field
        assert not start_turn.can_use_condition({}), (
            "Should fail when tamer is not on field")

    # ── [On Play] Reveal top 3, add blue + green Digimon ─────────────

    def test_on_play_effect_exists(self, debug_runner):
        """Should have On Play effect."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, ["BT3-093"])
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play]
        assert len(on_play) >= 1, "Should have On Play effect"

    def test_on_play_reveals_3_cards(self, debug_runner):
        """[On Play] Should call effect_reveal_and_select_multi with count=3."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT3-093"])
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        reveal_calls = []
        original_reveal = runner.game.effect_reveal_and_select_multi

        def mock_reveal(player, count, passes, remaining_placement='deck_bottom',
                        is_optional=False):
            reveal_calls.append({
                'count': count,
                'passes': passes,
                'remaining_placement': remaining_placement,
                'is_optional': is_optional,
            })

        runner.game.effect_reveal_and_select_multi = mock_reveal
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play.on_process_callback(ctx)
        finally:
            runner.game.effect_reveal_and_select_multi = original_reveal

        assert len(reveal_calls) == 1, "Should call effect_reveal_and_select_multi once"
        assert reveal_calls[0]['count'] == 3, "Should reveal 3 cards"
        assert reveal_calls[0]['remaining_placement'] == 'deck_bottom', (
            "Remaining cards should go to deck bottom")

    def test_on_play_two_selection_passes(self, debug_runner):
        """[On Play] Should have 2 passes: 1 blue Digimon, 1 green Digimon."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT3-093"])
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        reveal_calls = []
        original_reveal = runner.game.effect_reveal_and_select_multi

        def mock_reveal(player, count, passes, **kwargs):
            reveal_calls.append({'passes': passes})

        runner.game.effect_reveal_and_select_multi = mock_reveal
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play.on_process_callback(ctx)
        finally:
            runner.game.effect_reveal_and_select_multi = original_reveal

        assert len(reveal_calls) == 1
        passes = reveal_calls[0]['passes']
        assert len(passes) == 2, f"Should have 2 passes (blue + green). Got {len(passes)}"

        # Each pass should be (filter_fn, 'hand')
        for i, (filter_fn, placement) in enumerate(passes):
            assert placement == 'hand', f"Pass {i} placement should be 'hand', got '{placement}'"
            assert callable(filter_fn), f"Pass {i} should have a callable filter"

    def test_on_play_blue_filter_accepts_blue_digimon(self, debug_runner):
        """Blue filter pass should accept Blue Digimon and reject non-Blue."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT3-093"])
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        # Extract the filter functions
        reveal_calls = []
        original_reveal = runner.game.effect_reveal_and_select_multi

        def mock_reveal(player, count, passes, **kwargs):
            reveal_calls.append({'passes': passes})

        runner.game.effect_reveal_and_select_multi = mock_reveal
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play.on_process_callback(ctx)
        finally:
            runner.game.effect_reveal_and_select_multi = original_reveal

        blue_filter = reveal_calls[0]['passes'][0][0]

        # Create test card sources
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        blue_digimon = db.create_card_source("BT3-021", runner.game.player1)  # Veemon (Blue)
        red_digimon = db.create_card_source("BT1-010", runner.game.player1)  # Agumon (Red)
        blue_tamer = db.create_card_source("BT3-093", runner.game.player1)  # Davis (Blue Tamer)

        assert blue_filter(blue_digimon), "Blue filter should accept Blue Digimon"
        assert not blue_filter(red_digimon), "Blue filter should reject Red Digimon"
        assert not blue_filter(blue_tamer), "Blue filter should reject non-Digimon (Tamer)"

    def test_on_play_green_filter_accepts_green_digimon(self, debug_runner):
        """Green filter pass should accept Green Digimon and reject non-Green."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT3-093"])
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        reveal_calls = []
        original_reveal = runner.game.effect_reveal_and_select_multi

        def mock_reveal(player, count, passes, **kwargs):
            reveal_calls.append({'passes': passes})

        runner.game.effect_reveal_and_select_multi = mock_reveal
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play.on_process_callback(ctx)
        finally:
            runner.game.effect_reveal_and_select_multi = original_reveal

        green_filter = reveal_calls[0]['passes'][1][0]

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        green_digimon = db.create_card_source("BT3-047", runner.game.player1)  # Wormmon (Green)
        blue_digimon = db.create_card_source("BT3-021", runner.game.player1)  # Veemon (Blue)

        assert green_filter(green_digimon), "Green filter should accept Green Digimon"
        assert not green_filter(blue_digimon), "Green filter should reject Blue Digimon"

    def test_on_play_is_optional(self, debug_runner):
        """[On Play] reveal+select should be optional (player can decline picks)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT3-093"])
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        reveal_calls = []
        original_reveal = runner.game.effect_reveal_and_select_multi

        def mock_reveal(player, count, passes, is_optional=False, **kwargs):
            reveal_calls.append({'is_optional': is_optional})

        runner.game.effect_reveal_and_select_multi = mock_reveal
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play.on_process_callback(ctx)
        finally:
            runner.game.effect_reveal_and_select_multi = original_reveal

        assert reveal_calls[0]['is_optional'], (
            "On Play reveal+select should be optional")

    # ── Security Effect ──────────────────────────────────────────────

    def test_security_effect_exists(self, debug_runner):
        """Should have Security effect."""
        runner = debug_runner(initial_memory=0)
        card = runner.inject_card(1, "BT3-093", "hand")
        effects = card.effect_list(None)
        sec = [e for e in effects if e.is_security_effect]
        assert len(sec) >= 1, "Should have Security effect"

    def test_security_effect_timing(self, debug_runner):
        """Security effect should have SecuritySkill timing."""
        runner = debug_runner(initial_memory=0)
        card = runner.inject_card(1, "BT3-093", "hand")
        effects = card.effect_list(None)
        sec = [e for e in effects if e.is_security_effect]
        # At least one should have SecuritySkill timing
        sec_skill = [e for e in sec if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_skill) >= 1, "Should have SecuritySkill timing effect"
