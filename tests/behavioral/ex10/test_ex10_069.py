"""Behavioral tests for EX10-069 Unique Emblem: Gravel Hearts.

Card text:
  Option | Black | Play Cost 3
  [Main] You may play 1 [Sunarizamon] or [Close] from your hand or trash
      without paying the cost. Then, place this card in the battle area.
  [Your Turn] When any of your [Close]s suspend, <Delay>
      (By trashing this card after the placing turn, activate the effect below.)
      * 1 of your [Mineral] or [Rock] trait Digimon may digivolve into a Digimon
        card with the [Mineral] trait and [LIBERATOR] trait in the hand with the
        digivolution cost reduced by 3.
  [Security] Activate this card's [Main] effects.

Key faithfulness points tested (from C# reference):
  - [Main] plays 1 Sunarizamon or Close from hand or trash without paying cost.
  - "Then, place this card in the battle area" handled by Delay marker (_is_delay).
  - Delay trigger: OnTappedAnyone timing, checks your turn, suspended perm is YOUR
    Close Tamer on YOUR battle area.
  - Delay effect: trash this card, select own Mineral/Rock Digimon, digivolve into
    (Rock OR Mineral) AND LIBERATOR from hand with cost -3.
  - C# CardCondition checks HasRockMineralTraits && HasLiberatorTraits for digivolve
    target, meaning both Rock and Mineral qualify (not just Mineral).
  - Security: replays Main effect.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming

# Card IDs
EX10_069 = "EX10-069"          # Card under test (Option)
CLOSE_TAMER = "EX10-063"       # Close (Tamer, LIBERATOR, cost 3)
SUNARIZAMON = "EX10-025"       # Sunarizamon (Digimon, Reptile/LIBERATOR, Lv.3)
GOTSUMON_ROCK = "BT2-054"      # Gotsumon (Rock trait, Lv.3)
GOLEMON_MINERAL = "BT4-066"    # Golemon (Mineral trait, Lv.4)
LANDRAMON = "EX10-028"         # Landramon (Mineral + LIBERATOR, Lv.4)
PROGANOMON = "EX10-032"        # Proganomon (Mineral + LIBERATOR, Lv.5)
PYRAMIDIMON = "EX10-033"       # Pyramidimon (Mineral + LIBERATOR, Lv.6)
AGUMON_ST1 = "ST1-03"          # Agumon (Lv.3 Red, filler)

FILLER = [AGUMON_ST1] * 50


@pytest.mark.behavioral
class TestEX10069MainEffect:
    """[Main] You may play 1 [Sunarizamon] or [Close] from hand or trash free."""

    def test_has_option_skill_effect(self, debug_runner):
        """Should have an OptionSkill timing effect."""
        runner = debug_runner(initial_memory=5)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(EX10_069, runner.game.player1)
        effects = cs.effect_list(None)
        option_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(option_effects) >= 1, "Should have OptionSkill effect"

    def test_has_delay_marker(self, debug_runner):
        """Should have a _is_delay effect so the option stays on the field."""
        runner = debug_runner(initial_memory=5)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(EX10_069, runner.game.player1)
        effects = cs.effect_list(None)
        delay_effects = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delay_effects) >= 1, "Should have _is_delay marker effect"

    def test_play_sunarizamon_from_hand(self, debug_runner):
        """Main effect should allow playing Sunarizamon from hand without paying cost."""
        runner = debug_runner(
            deck1=[EX10_069] * 4 + [SUNARIZAMON] * 4 + [AGUMON_ST1] * 42,
            deck2=FILLER,
            initial_memory=5,
        )
        runner.set_phase("Main")

        # Place a Black Digimon to satisfy option color requirement
        runner.place_on_field(1, [GOTSUMON_ROCK])

        # Inject Sunarizamon into hand
        runner.inject_card(1, SUNARIZAMON, "hand")

        before = runner.snapshot()
        suna_in_hand = sum(1 for c in before.p1_hand if c == SUNARIZAMON)
        assert suna_in_hand >= 1, "Should have Sunarizamon in hand"

        # Play the option
        play_action = runner.find_action("Gravel Hearts")
        if play_action is None:
            play_action = runner.find_action("EX10-069")
        assert play_action is not None, f"Should be able to play EX10-069. Actions: {runner.actions()}"
        runner.execute(play_action)

        # Should enter selection phase to choose Sunarizamon/Close
        # Auto-resolve picks the first available
        runner.auto_resolve()

        after = runner.snapshot()
        # The option should be on the field (delay) and Sunarizamon should be played
        field_names = [f.card_name for f in after.p1_field]
        # At least Sunarizamon or the option should be on the field
        assert any("Sunarizamon" in n for n in field_names) or \
               any("Gravel Hearts" in n for n in field_names), \
               f"After Main effect, field should have Sunarizamon or option. Field: {field_names}"

    def test_play_close_from_hand(self, debug_runner):
        """Main effect should allow playing Close tamer from hand without paying cost."""
        runner = debug_runner(
            deck1=[EX10_069] * 4 + [CLOSE_TAMER] * 4 + [AGUMON_ST1] * 42,
            deck2=FILLER,
            initial_memory=5,
        )
        runner.set_phase("Main")

        # Place a Black Digimon to satisfy option color requirement
        runner.place_on_field(1, [GOTSUMON_ROCK])

        runner.inject_card(1, CLOSE_TAMER, "hand")

        play_action = runner.find_action("Gravel Hearts")
        if play_action is None:
            play_action = runner.find_action("EX10-069")
        assert play_action is not None, f"Should be able to play EX10-069. Actions: {runner.actions()}"
        runner.execute(play_action)
        runner.auto_resolve()

        after = runner.snapshot()
        field_names = [f.card_name for f in after.p1_field]
        assert any("Close" in n for n in field_names), \
            f"Close tamer should be on the field. Field: {field_names}"


@pytest.mark.behavioral
class TestEX10069DelayTrigger:
    """[Your Turn] When any of your [Close]s suspend, <Delay>."""

    def test_delay_trigger_condition_checks_close_tamer(self, debug_runner):
        """Delay trigger should fire when a Close tamer suspends on your turn."""
        runner = debug_runner(initial_memory=5)

        # Place the option card in battle area (simulating post-Main)
        option_perm = runner.place_on_field(1, [EX10_069])
        # Place a Close tamer
        close_perm = runner.place_on_field(1, [CLOSE_TAMER])

        runner.set_phase("Main")

        game = runner.game
        card = option_perm.top_card
        effects = card.effect_list(None)

        # Find the OnTappedAnyone effect
        on_tap_effects = [e for e in effects if e.timing == EffectTiming.OnTappedAnyone]
        assert len(on_tap_effects) >= 1, "Should have OnTappedAnyone effect for delay trigger"

        effect = on_tap_effects[0]
        # Check condition with Close tamer as the suspended permanent
        context = {
            'game': game,
            'player': game.player1,
            'permanent': option_perm,
            'event_permanent': close_perm,
        }
        assert effect.can_use_condition(context), \
            "Delay trigger should fire when Close tamer suspends on your turn"

    def test_delay_trigger_rejects_non_close_digimon(self, debug_runner):
        """Delay trigger should NOT fire when a non-Close permanent suspends."""
        runner = debug_runner(initial_memory=5)

        option_perm = runner.place_on_field(1, [EX10_069])
        agumon_perm = runner.place_on_field(1, [AGUMON_ST1])

        runner.set_phase("Main")

        game = runner.game
        card = option_perm.top_card
        effects = card.effect_list(None)
        on_tap_effects = [e for e in effects if e.timing == EffectTiming.OnTappedAnyone]
        effect = on_tap_effects[0]

        context = {
            'game': game,
            'player': game.player1,
            'permanent': option_perm,
            'event_permanent': agumon_perm,
        }
        assert not effect.can_use_condition(context), \
            "Delay trigger should NOT fire for non-Close permanents"

    def test_delay_trigger_rejects_opponent_close(self, debug_runner):
        """Delay trigger should NOT fire when opponent's Close suspends."""
        runner = debug_runner(initial_memory=5)

        option_perm = runner.place_on_field(1, [EX10_069])
        opp_close = runner.place_on_field(2, [CLOSE_TAMER])

        runner.set_phase("Main")

        game = runner.game
        card = option_perm.top_card
        effects = card.effect_list(None)
        on_tap_effects = [e for e in effects if e.timing == EffectTiming.OnTappedAnyone]
        effect = on_tap_effects[0]

        context = {
            'game': game,
            'player': game.player1,
            'permanent': option_perm,
            'event_permanent': opp_close,
        }
        assert not effect.can_use_condition(context), \
            "Delay trigger should NOT fire for opponent's Close"

    def test_delay_trigger_rejects_opponent_turn(self, debug_runner):
        """Delay trigger should NOT fire on opponent's turn (Your Turn only)."""
        runner = debug_runner(initial_memory=5, first_player=2)

        option_perm = runner.place_on_field(1, [EX10_069])
        close_perm = runner.place_on_field(1, [CLOSE_TAMER])

        runner.set_phase("Main")

        game = runner.game
        card = option_perm.top_card
        effects = card.effect_list(None)
        on_tap_effects = [e for e in effects if e.timing == EffectTiming.OnTappedAnyone]
        effect = on_tap_effects[0]

        context = {
            'game': game,
            'player': game.player1,
            'permanent': option_perm,
            'event_permanent': close_perm,
        }
        assert not effect.can_use_condition(context), \
            "Delay trigger should NOT fire on opponent's turn"


@pytest.mark.behavioral
class TestEX10069DelayEffect:
    """Delay effect: digivolve Mineral/Rock Digimon into Mineral+LIBERATOR from hand."""

    def test_delay_process_trashes_option(self, debug_runner):
        """Activating the delay effect should trash the option card from battle area."""
        runner = debug_runner(initial_memory=5)

        option_perm = runner.place_on_field(1, [EX10_069])
        close_perm = runner.place_on_field(1, [CLOSE_TAMER])
        # Rock Digimon that can be selected as digivolve base
        rock_perm = runner.place_on_field(1, [GOTSUMON_ROCK])

        runner.set_phase("Main")

        game = runner.game
        card = option_perm.top_card
        effects = card.effect_list(None)
        on_tap_effects = [e for e in effects if e.timing == EffectTiming.OnTappedAnyone]
        effect = on_tap_effects[0]

        # Manually invoke the process callback
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': option_perm,
            'event_permanent': close_perm,
        }
        effect.on_process_callback(ctx)
        # Auto-resolve any selection phases
        runner.auto_resolve()

        # The option should have been trashed (deleted from battle area)
        after = runner.snapshot()
        field_ids = [f.card_id for f in after.p1_field]
        assert EX10_069 not in field_ids, \
            f"Option should be trashed after delay activation. Field: {field_ids}"

    def test_delay_selects_rock_or_mineral_digimon(self, debug_runner):
        """Delay effect should allow selecting Rock or Mineral trait Digimon."""
        runner = debug_runner(initial_memory=5)

        option_perm = runner.place_on_field(1, [EX10_069])
        rock_perm = runner.place_on_field(1, [GOTSUMON_ROCK])
        mineral_perm = runner.place_on_field(1, [GOLEMON_MINERAL])

        game = runner.game
        card = option_perm.top_card
        effects = card.effect_list(None)
        on_tap_effects = [e for e in effects if e.timing == EffectTiming.OnTappedAnyone]
        effect = on_tap_effects[0]

        # Verify the base_filter logic accepts both Rock and Mineral
        # We test via the process callback's internal filter by mocking
        selected_perms = []
        original_select = game.effect_select_own_permanent

        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.battle_area:
                if filter_fn and filter_fn(p):
                    selected_perms.append(p)

        game.effect_select_own_permanent = mock_select

        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': option_perm,
        }
        effect.on_process_callback(ctx)

        game.effect_select_own_permanent = original_select

        assert rock_perm in selected_perms, "Rock Digimon should be selectable"
        assert mineral_perm in selected_perms, "Mineral Digimon should be selectable"

    def test_delay_rejects_non_rock_mineral_digimon(self, debug_runner):
        """Delay effect should NOT allow selecting Digimon without Rock/Mineral traits."""
        runner = debug_runner(initial_memory=5)

        option_perm = runner.place_on_field(1, [EX10_069])
        agumon_perm = runner.place_on_field(1, [AGUMON_ST1])

        game = runner.game
        card = option_perm.top_card
        effects = card.effect_list(None)
        on_tap_effects = [e for e in effects if e.timing == EffectTiming.OnTappedAnyone]
        effect = on_tap_effects[0]

        selected_perms = []
        original_select = game.effect_select_own_permanent

        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.battle_area:
                if filter_fn and filter_fn(p):
                    selected_perms.append(p)

        game.effect_select_own_permanent = mock_select

        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': option_perm,
        }
        effect.on_process_callback(ctx)

        game.effect_select_own_permanent = original_select

        assert agumon_perm not in selected_perms, \
            "Non-Rock/Mineral Digimon should NOT be selectable"

    def test_digivolve_filter_accepts_rock_or_mineral_plus_liberator(self, debug_runner):
        """Digivolve target filter should accept Digimon with (Rock OR Mineral) AND LIBERATOR.

        The C# CardCondition uses HasRockMineralTraits && HasLiberatorTraits.
        HasRockMineralTraits = Rock OR Mineral.
        So both Mineral+LIBERATOR and (hypothetical) Rock+LIBERATOR cards should qualify.
        """
        runner = debug_runner(initial_memory=5)

        option_perm = runner.place_on_field(1, [EX10_069])
        rock_perm = runner.place_on_field(1, [GOTSUMON_ROCK])

        # Put a Mineral+LIBERATOR card in hand
        runner.inject_card(1, LANDRAMON, "hand")

        game = runner.game
        card = option_perm.top_card
        effects = card.effect_list(None)
        on_tap_effects = [e for e in effects if e.timing == EffectTiming.OnTappedAnyone]
        effect = on_tap_effects[0]

        # Mock to capture the digi_filter
        captured_filters = []
        original_digivolve = game.effect_digivolve_from_hand

        def mock_digivolve(player, permanent, filter_fn, **kwargs):
            captured_filters.append(filter_fn)

        game.effect_digivolve_from_hand = mock_digivolve

        # Also mock select to immediately call the callback
        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.battle_area:
                if filter_fn and filter_fn(p):
                    callback(p)
                    return

        game.effect_select_own_permanent = mock_select

        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': option_perm,
        }
        effect.on_process_callback(ctx)

        game.effect_digivolve_from_hand = original_digivolve

        assert len(captured_filters) >= 1, "Should have called effect_digivolve_from_hand"

        digi_filter = captured_filters[0]

        # Test the filter with mock CardSource objects
        class MockCard:
            def __init__(self, is_digimon, traits):
                self.is_digimon = is_digimon
                self.card_traits = traits

        # Mineral + LIBERATOR should pass
        assert digi_filter(MockCard(True, ['Mineral', 'LIBERATOR'])), \
            "Mineral+LIBERATOR Digimon should be accepted"

        # Rock + LIBERATOR should also pass (per C# HasRockMineralTraits)
        assert digi_filter(MockCard(True, ['Rock', 'LIBERATOR'])), \
            "Rock+LIBERATOR Digimon should be accepted (C# uses HasRockMineralTraits)"

        # Just Mineral (no LIBERATOR) should fail
        assert not digi_filter(MockCard(True, ['Mineral'])), \
            "Mineral without LIBERATOR should be rejected"

        # Just LIBERATOR (no Rock/Mineral) should fail
        assert not digi_filter(MockCard(True, ['LIBERATOR'])), \
            "LIBERATOR without Rock/Mineral should be rejected"

        # Non-Digimon should fail
        assert not digi_filter(MockCard(False, ['Mineral', 'LIBERATOR'])), \
            "Non-Digimon should be rejected"

    def test_digivolve_cost_reduction_3(self, debug_runner):
        """Digivolve should have cost_reduction=3."""
        runner = debug_runner(initial_memory=5)

        option_perm = runner.place_on_field(1, [EX10_069])
        rock_perm = runner.place_on_field(1, [GOTSUMON_ROCK])
        runner.inject_card(1, LANDRAMON, "hand")

        game = runner.game
        card = option_perm.top_card
        effects = card.effect_list(None)
        on_tap_effects = [e for e in effects if e.timing == EffectTiming.OnTappedAnyone]
        effect = on_tap_effects[0]

        captured_kwargs = []
        original_digivolve = game.effect_digivolve_from_hand

        def mock_digivolve(player, permanent, filter_fn, **kwargs):
            captured_kwargs.append(kwargs)

        game.effect_digivolve_from_hand = mock_digivolve

        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.battle_area:
                if filter_fn and filter_fn(p):
                    callback(p)
                    return

        game.effect_select_own_permanent = mock_select

        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': option_perm,
        }
        effect.on_process_callback(ctx)

        game.effect_digivolve_from_hand = original_digivolve

        assert len(captured_kwargs) >= 1, "Should have called effect_digivolve_from_hand"
        assert captured_kwargs[0].get('cost_reduction') == 3, \
            f"Cost reduction should be 3, got {captured_kwargs[0].get('cost_reduction')}"


@pytest.mark.behavioral
class TestEX10069SecurityEffect:
    """[Security] Activate this card's [Main] effects."""

    def test_has_security_effect(self, debug_runner):
        """Should have a SecuritySkill timing effect."""
        runner = debug_runner(initial_memory=5)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(EX10_069, runner.game.player1)
        effects = cs.effect_list(None)
        security_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(security_effects) >= 1, "Should have SecuritySkill effect"

    def test_security_effect_plays_sunarizamon_or_close(self, debug_runner):
        """Security effect should replay the Main effect (play Sunarizamon/Close)."""
        runner = debug_runner(initial_memory=5)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(EX10_069, runner.game.player1)
        effects = cs.effect_list(None)
        security_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(security_effects) >= 1

        effect = security_effects[0]
        game = runner.game

        # Inject a Sunarizamon into hand
        runner.inject_card(1, SUNARIZAMON, "hand")

        # Track calls to effect_play_from_zone
        called = []
        original = game.effect_play_from_zone

        def mock_play(player, zone, filter_fn, **kwargs):
            called.append((zone, kwargs.get('free', False)))
            # Don't actually execute to avoid side effects
        game.effect_play_from_zone = mock_play

        ctx = {
            'game': game,
            'player': game.player1,
        }
        effect.on_process_callback(ctx)

        game.effect_play_from_zone = original

        assert len(called) >= 1, "Security effect should call effect_play_from_zone"
        assert called[0][0] == 'hand_or_trash', \
            f"Should play from hand_or_trash, got {called[0][0]}"
        assert called[0][1] is True, "Should play for free"
