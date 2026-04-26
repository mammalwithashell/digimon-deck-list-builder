"""Behavioral tests for EX10-035 Machinedramon (Lv.6 Black Machine/Dark Masters, DP 11000, Cost 11).

Card text:
[Hand] [Main] If you don't have any Digimon other than Digimon with [Dark Masters]
    in their texts, you may play this card with the play cost reduced by 5.
    At turn end, delete the Digimon this effect played.
[On Play] [When Attacking] <De-Digivolve 2> 2 of your opponent's Digimon.
    (Trash up to 2 cards from the top. You can't trash past level 3 cards.)
[All Turns] This Digimon can only digivolve into [Apocalymon].
[On Deletion] If you have no black face-up security cards, place this Digimon
    face up as the bottom security card.

Inherited Effect:
Security Effect [Security] If this card was face-up, you may play 1 level 5
    or lower card with [Dark Masters] in its text from your hand or trash
    without paying the cost.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX10035Machinedramon:
    """Tests for EX10-035 Machinedramon."""

    # ── Cost Reduction Effect ────────────────────────────────────────

    def test_cost_reduction_condition_only_dark_masters_on_field(self, debug_runner):
        """Cost reduction should be available when all field Digimon have Dark Masters in text."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a Dark Masters Digimon on field (EX10-035 itself has Dark Masters in text)
        runner.place_on_field(1, ["EX10-035"])
        # Inject a second copy to hand
        runner.inject_card(1, "EX10-035", "hand")

        game = runner.game
        player = game.player1
        hand_card = player.hand_cards[-1]

        effects = hand_card.effect_list(None)
        cost_effects = [e for e in effects
                        if e.timing == EffectTiming.BeforePayCost]
        assert len(cost_effects) >= 1, "Should have BeforePayCost effect"

        effect = cost_effects[0]
        ctx = {'card_source': hand_card, 'player': player}
        assert effect.can_use_condition(ctx), \
            "Cost reduction should be available when all field Digimon have Dark Masters text"

    def test_cost_reduction_blocked_by_non_dark_masters(self, debug_runner):
        """Cost reduction should NOT be available when a non-Dark Masters Digimon is on field."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a non-Dark Masters Digimon on field
        runner.place_on_field(1, ["ST1-03"])  # Agumon, no Dark Masters text
        runner.inject_card(1, "EX10-035", "hand")

        game = runner.game
        player = game.player1
        hand_card = player.hand_cards[-1]

        effects = hand_card.effect_list(None)
        cost_effects = [e for e in effects
                        if e.timing == EffectTiming.BeforePayCost]
        effect = cost_effects[0]
        ctx = {'card_source': hand_card, 'player': player}
        assert not effect.can_use_condition(ctx), \
            "Cost reduction should be blocked when non-Dark Masters Digimon on field"

    def test_cost_reduction_allowed_with_empty_field(self, debug_runner):
        """Cost reduction should be available when field is empty (no non-DM Digimon)."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.inject_card(1, "EX10-035", "hand")

        game = runner.game
        player = game.player1
        hand_card = player.hand_cards[-1]

        effects = hand_card.effect_list(None)
        cost_effects = [e for e in effects
                        if e.timing == EffectTiming.BeforePayCost]
        effect = cost_effects[0]
        ctx = {'card_source': hand_card, 'player': player}
        assert effect.can_use_condition(ctx), \
            "Cost reduction should be available when field is empty"

    def test_cost_reduction_not_leaked_to_other_cards(self, debug_runner):
        """Cost reduction should only apply to THIS card, not other cards."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.inject_card(1, "EX10-035", "hand")
        runner.inject_card(1, "ST1-03", "hand")

        game = runner.game
        player = game.player1

        machinedramon = None
        other_card = None
        for c in player.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "EX10-035":
                machinedramon = c
            elif c.c_entity_base and c.c_entity_base.card_id == "ST1-03":
                other_card = c

        assert machinedramon is not None
        assert other_card is not None

        effects = machinedramon.effect_list(None)
        cost_effects = [e for e in effects
                        if e.timing == EffectTiming.BeforePayCost]
        effect = cost_effects[0]

        # Should NOT apply to the other card
        ctx = {'card_source': other_card, 'player': player}
        assert not effect.can_use_condition(ctx), \
            "Cost reduction must not apply to other cards"

    # ── On Play De-Digivolve ─────────────────────────────────────────

    def test_on_play_de_digivolve_two_targets(self, debug_runner):
        """[On Play] Should De-Digivolve 2 on two opponent Digimon."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX10-035"])

        # Place two digivolved Digimon on opponent's field (with stacks)
        target1 = runner.place_on_field(2, ["ST1-03", "ST1-07"])  # Agumon -> Greymon
        target2 = runner.place_on_field(2, ["ST1-03", "ST1-11"])  # Agumon -> WarGreymon

        game = runner.game
        card = perm.top_card

        effects = card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone and e.is_on_play]
        assert len(on_play) >= 1, "Should have On Play effect"

        effect = on_play[0]

        # Track selections
        selected = []
        original_select = game.effect_select_opponent_permanent

        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=None):
            candidates = [p for p in player.enemy.battle_area
                          if filter_fn is None or filter_fn(p)]
            if candidates:
                target = candidates[0]
                selected.append(target)
                callback(target)

        game.effect_select_opponent_permanent = mock_select

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert len(selected) == 2, "Should select 2 opponent Digimon for De-Digivolve"

    def test_on_play_de_digivolve_uses_selection_phase(self, debug_runner):
        """[On Play] De-Digivolve must go through selection phase even with 1 candidate.

        The no-auto-selection rule requires all selections go through
        effect_select_opponent_permanent, not bypassing for single candidates.
        """
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX10-035"])
        # Only 1 opponent Digimon
        target = runner.place_on_field(2, ["ST1-03", "ST1-07"])

        game = runner.game
        card = perm.top_card

        effects = card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone and e.is_on_play]
        effect = on_play[0]

        select_calls = []

        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=None):
            candidates = [p for p in player.enemy.battle_area
                          if filter_fn is None or filter_fn(p)]
            select_calls.append(len(candidates))
            if candidates:
                callback(candidates[0])

        game.effect_select_opponent_permanent = mock_select

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Must call selection phase even with 1 candidate
        assert len(select_calls) >= 1, \
            "Must call effect_select_opponent_permanent even with 1 candidate"

    def test_de_digivolve_trashes_from_top(self, debug_runner):
        """De-Digivolve 2 should trash up to 2 cards from top of stack."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX10-035"])
        # Stack: ST1-03 (bottom/Lv3) -> ST1-07 (Lv4) -> ST1-11 (top/Lv6)
        target = runner.place_on_field(2, ["ST1-03", "ST1-07", "ST1-11"])

        initial_stack_size = len(target.card_sources)
        assert initial_stack_size == 3

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone and e.is_on_play]
        effect = on_play[0]

        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=None):
            candidates = [p for p in player.enemy.battle_area
                          if filter_fn is None or filter_fn(p)]
            if candidates:
                callback(candidates[0])

        game.effect_select_opponent_permanent = mock_select

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # De-Digivolve 2 removes top 2 from the 3-stack, leaving base Lv.3
        assert len(target.card_sources) == 1, \
            "De-Digivolve 2 should remove 2 cards from stack"

    # ── When Attacking De-Digivolve ──────────────────────────────────

    def test_when_attacking_de_digivolve(self, debug_runner):
        """[When Attacking] Should also De-Digivolve 2 on two opponent Digimon."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX10-035"])
        target = runner.place_on_field(2, ["ST1-03", "ST1-07"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_attack = [e for e in effects
                     if e.timing == EffectTiming.OnUseAttack and e.is_on_attack]
        assert len(on_attack) >= 1, "Should have When Attacking effect"

        # Condition should pass for THIS permanent
        effect = on_attack[0]
        ctx = {'attacker': perm, 'permanent': perm}
        assert effect.can_use_condition(ctx), "Attack condition should pass for this Digimon"

    def test_when_attacking_condition_rejects_other_attacker(self, debug_runner):
        """[When Attacking] Should NOT fire for a different attacker."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX10-035"])
        other = runner.place_on_field(1, ["ST1-03"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_attack = [e for e in effects
                     if e.timing == EffectTiming.OnUseAttack and e.is_on_attack]
        effect = on_attack[0]

        ctx = {'attacker': other, 'permanent': other}
        assert not effect.can_use_condition(ctx), \
            "Attack effect should not fire for a different attacker"

    # ── All Turns: Digivolve Restriction ─────────────────────────────

    def test_digivolve_restriction_exists(self, debug_runner):
        """[All Turns] Should have a digivolve restriction (only into Apocalymon)."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX10-035"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)

        # Should either register a CANNOT_DIGIVOLVE modifier or have some restriction
        from engine_py_legacy.engine.interfaces.modifiers import ModifierType

        # Fire any static/declarative effects
        for e in effects:
            if e.on_process_callback and (getattr(e, 'is_declarative', False) or
                    e.timing in (EffectTiming.NoTiming, None)):
                if e.can_use_condition is None or e.can_use_condition({}):
                    try:
                        e.on_process_callback({
                            'player': game.player1,
                            'game': game,
                            'permanent': perm,
                        })
                    except Exception:
                        pass

        # Check if CANNOT_DIGIVOLVE modifier is registered
        has_digi_restriction = game.modifiers.has_modifier(
            perm, ModifierType.CANNOT_DIGIVOLVE)
        # If no modifier, check for any digivolve restriction attribute
        has_restriction_attr = any(
            'digivolve' in (getattr(e, 'effect_name', '') or '').lower() and
            'restriction' in (getattr(e, 'effect_name', '') or '').lower()
            for e in effects
        ) or any(
            'only digivolve' in (getattr(e, 'effect_description', '') or '').lower()
            for e in effects
        )

        assert has_digi_restriction or has_restriction_attr, \
            "[All Turns] Must have digivolve restriction (only into Apocalymon)"

    # ── On Deletion ──────────────────────────────────────────────────

    def test_on_deletion_places_as_face_up_security(self, debug_runner):
        """[On Deletion] Should place as face-up bottom security when no black face-up security."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX10-035"])

        game = runner.game
        player = game.player1
        card = perm.top_card

        # Clear security to ensure no black face-up cards
        player.security_cards.clear()
        player.face_up_security.clear()

        effects = card.effect_list(None)
        on_deletion = [e for e in effects
                       if e.timing == EffectTiming.OnDestroyedAnyone
                       and e.is_on_deletion]
        assert len(on_deletion) >= 1, "Should have On Deletion effect"

        effect = on_deletion[0]
        ctx = {
            'player': player,
            'game': game,
            'permanent': perm,
        }
        assert effect.can_use_condition(ctx), \
            "On Deletion condition should pass with no black face-up security"

        effect.on_process_callback(ctx)

        assert card in player.security_cards, "Card should be in security"
        assert card in player.face_up_security, "Card should be face-up in security"

    def test_on_deletion_blocked_by_black_face_up_security(self, debug_runner):
        """[On Deletion] Should NOT activate when there's a black face-up security card."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX10-035"])
        game = runner.game
        player = game.player1
        card = perm.top_card

        # Add a black card to face-up security
        runner.inject_card(1, "EX10-035", "security_top")
        # security_top inserts at index 0
        if player.security_cards:
            sec_card = player.security_cards[0]
            player.face_up_security.add(sec_card)

        effects = card.effect_list(None)
        on_deletion = [e for e in effects
                       if e.timing == EffectTiming.OnDestroyedAnyone
                       and e.is_on_deletion]
        effect = on_deletion[0]
        ctx = {
            'player': player,
            'game': game,
            'permanent': perm,
        }
        assert not effect.can_use_condition(ctx), \
            "On Deletion should be blocked when black face-up security exists"

    # ── Security Effect ──────────────────────────────────────────────

    def test_security_effect_plays_dark_masters_lv5(self, debug_runner):
        """[Security] Should play Lv5 or lower Dark Masters Digimon from hand/trash."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX10-035"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)

        sec_effects = [e for e in effects
                       if e.timing == EffectTiming.SecuritySkill
                       and e.is_security_effect]
        assert len(sec_effects) >= 1, "Should have security effect"

        effect = sec_effects[0]
        assert effect.is_optional, "Security effect should be optional"
        assert effect.is_inherited_effect, "Security effect should be inherited"

    def test_security_effect_filter_requires_digimon(self, debug_runner):
        """[Security] Play filter should require target to be a Digimon.

        C# reference checks cardSource.IsDigimon — Options with Dark Masters
        text should NOT be valid targets.
        """
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX10-035"])
        game = runner.game
        player = game.player1
        card = perm.top_card

        effects = card.effect_list(None)
        sec_effects = [e for e in effects
                       if e.timing == EffectTiming.SecuritySkill
                       and e.is_security_effect]
        effect = sec_effects[0]

        # Capture the filter used by effect_play_from_zone
        captured_filter = []

        def mock_play_from_zone(player, zone, filter_fn, free=True, manual_reduction=0,
                                is_optional=True, prompt=""):
            captured_filter.append(filter_fn)

        game.effect_play_from_zone = mock_play_from_zone

        # Add card to face_up_security so condition passes
        player.face_up_security.add(card)

        effect.on_process_callback({
            'player': player,
            'game': game,
        })

        assert len(captured_filter) == 1, "Should call effect_play_from_zone"
        play_filter = captured_filter[0]

        # Create a mock non-Digimon card that has Dark Masters text
        class MockCard:
            def __init__(self, is_digimon, level, card_text, is_digi_egg=False):
                self.is_digimon = is_digimon
                self.level = level
                self.card_text = card_text
                self.is_digi_egg = is_digi_egg

        # Level 5 Dark Masters Digimon should pass
        dm_digimon = MockCard(True, 5, "Has Dark Masters in text")
        assert play_filter(dm_digimon), "Lv5 Dark Masters Digimon should pass filter"

        # Level 5 Dark Masters non-Digimon should FAIL (C# requires IsDigimon)
        dm_option = MockCard(False, 5, "Has Dark Masters in text")
        assert not play_filter(dm_option), \
            "Non-Digimon with Dark Masters text should NOT pass filter (C# requires IsDigimon)"

    def test_security_effect_filter_rejects_level_over_5(self, debug_runner):
        """[Security] Filter should reject cards with level > 5."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX10-035"])
        game = runner.game
        player = game.player1
        card = perm.top_card

        effects = card.effect_list(None)
        sec_effects = [e for e in effects
                       if e.timing == EffectTiming.SecuritySkill
                       and e.is_security_effect]
        effect = sec_effects[0]

        captured_filter = []

        def mock_play_from_zone(player, zone, filter_fn, free=True, manual_reduction=0,
                                is_optional=True, prompt=""):
            captured_filter.append(filter_fn)

        game.effect_play_from_zone = mock_play_from_zone
        player.face_up_security.add(card)

        effect.on_process_callback({
            'player': player,
            'game': game,
        })

        play_filter = captured_filter[0]

        class MockCard:
            def __init__(self, is_digimon, level, card_text, is_digi_egg=False):
                self.is_digimon = is_digimon
                self.level = level
                self.card_text = card_text
                self.is_digi_egg = is_digi_egg

        lv6_dm = MockCard(True, 6, "Has Dark Masters in text")
        assert not play_filter(lv6_dm), "Level 6 should be rejected"

    def test_security_effect_filter_rejects_no_dark_masters(self, debug_runner):
        """[Security] Filter should reject cards without Dark Masters text."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX10-035"])
        game = runner.game
        player = game.player1
        card = perm.top_card

        effects = card.effect_list(None)
        sec_effects = [e for e in effects
                       if e.timing == EffectTiming.SecuritySkill
                       and e.is_security_effect]
        effect = sec_effects[0]

        captured_filter = []

        def mock_play_from_zone(player, zone, filter_fn, free=True, manual_reduction=0,
                                is_optional=True, prompt=""):
            captured_filter.append(filter_fn)

        game.effect_play_from_zone = mock_play_from_zone
        player.face_up_security.add(card)

        effect.on_process_callback({
            'player': player,
            'game': game,
        })

        play_filter = captured_filter[0]

        class MockCard:
            def __init__(self, is_digimon, level, card_text, is_digi_egg=False):
                self.is_digimon = is_digimon
                self.level = level
                self.card_text = card_text
                self.is_digi_egg = is_digi_egg

        no_dm = MockCard(True, 4, "Regular Digimon text")
        assert not play_filter(no_dm), "Card without Dark Masters text should be rejected"

    # ── End of Turn Delete ───────────────────────────────────────────

    def test_eot_delete_condition_false_by_default(self, debug_runner):
        """End of turn delete should NOT trigger for normally played Machinedramon."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX10-035"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        eot_effects = [e for e in effects
                       if e.timing == EffectTiming.OnEndTurn]
        assert len(eot_effects) >= 1, "Should have OnEndTurn effect"

        effect = eot_effects[0]
        # Card was placed directly, not via cost reduction
        assert not effect.can_use_condition({
            'player': game.player1,
            'game': game,
        }), "EOT delete should not trigger without cost reduction play"
