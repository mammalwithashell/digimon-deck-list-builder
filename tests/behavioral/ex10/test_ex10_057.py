"""Behavioral tests for EX10-057 Piedmon (Lv.6 Purple Wizard/Dark Masters, DP 11000, Cost 11).

Card text:
[Hand] [Main] If you don't have any Digimon other than Digimon with [Dark Masters]
    in their texts, you may play this card with the play cost reduced by 5.
    At turn end, delete the Digimon this effect played.
[On Play] [When Attacking] Delete 1 of your opponent's unsuspended Digimon.
[All Turns] This Digimon can only digivolve into [Apocalymon].
[On Deletion] If you have no purple face-up security cards, place this Digimon
    face up as the bottom security card.
Inherited: [Security] If this card was face-up, you may play 1 level 5 or lower
    Digimon card with [Dark Masters] in its text from your hand or trash without
    paying the cost.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX10057Piedmon:
    """Tests for EX10-057 Piedmon."""

    # ── Cost Reduction ───────────────────────────────────────────────

    def test_cost_reduction_with_only_dark_masters_on_field(self, debug_runner):
        """[Hand][Main] Cost should be reduced by 5 when only Dark Masters Digimon on field."""
        runner = debug_runner(initial_memory=10)
        # Place a Dark Masters Digimon on field for P1
        # EX10-020 Puppetmon is a Dark Masters card
        runner.place_on_field(1, ["EX10-020"])

        # Inject EX10-057 into P1 hand
        runner.inject_card(1, "EX10-057", "hand")
        runner.set_phase("Main")

        game = runner.game
        p1 = game.player1
        # Find the Piedmon card in hand
        piedmon = None
        for c in p1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "EX10-057":
                piedmon = c
                break
        assert piedmon is not None, "EX10-057 must be in hand"

        # Calculate play cost — should be 11 - 5 = 6
        cost = game.calculate_play_cost(p1, piedmon)
        assert cost == 6, f"Expected play cost 6 (11 - 5), got {cost}"

    def test_cost_reduction_blocked_by_non_dark_masters(self, debug_runner):
        """[Hand][Main] Cost reduction should NOT apply when a non-Dark Masters Digimon is on field."""
        runner = debug_runner(initial_memory=10)
        # Place a NON-Dark Masters Digimon on field for P1
        runner.place_on_field(1, ["ST1-03"])  # Agumon — no Dark Masters text

        runner.inject_card(1, "EX10-057", "hand")
        runner.set_phase("Main")

        game = runner.game
        p1 = game.player1
        piedmon = None
        for c in p1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "EX10-057":
                piedmon = c
                break
        assert piedmon is not None

        # Cost should remain at 11 (no reduction)
        cost = game.calculate_play_cost(p1, piedmon)
        assert cost == 11, f"Expected full play cost 11, got {cost}"

    def test_cost_reduction_with_empty_field(self, debug_runner):
        """[Hand][Main] Cost reduction should apply when field is empty (no non-Dark Masters)."""
        runner = debug_runner(initial_memory=10)

        runner.inject_card(1, "EX10-057", "hand")
        runner.set_phase("Main")

        game = runner.game
        p1 = game.player1
        piedmon = None
        for c in p1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "EX10-057":
                piedmon = c
                break
        assert piedmon is not None

        # Empty field = vacuous truth: no Digimon other than Dark Masters
        cost = game.calculate_play_cost(p1, piedmon)
        assert cost == 6, f"Expected play cost 6 (11 - 5), got {cost}"

    def test_eot_delete_flag_set_on_reduced_play(self, debug_runner):
        """The _ex10_eot_delete flag should be set when played via cost reduction."""
        runner = debug_runner(initial_memory=10)

        runner.inject_card(1, "EX10-057", "hand")
        runner.set_phase("Main")

        game = runner.game
        p1 = game.player1
        piedmon = None
        for c in p1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "EX10-057":
                piedmon = c
                break
        assert piedmon is not None

        # Calculate with commit=True to fire the process callback
        game.calculate_play_cost(p1, piedmon, commit=True)
        assert getattr(piedmon, '_ex10_eot_delete', False), \
            "_ex10_eot_delete should be True after committing cost reduction"

    # ── On Play: Delete Unsuspended Digimon ──────────────────────────

    def test_on_play_delete_unsuspended(self, debug_runner):
        """[On Play] Should delete 1 of opponent's unsuspended Digimon."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-057"])
        target = runner.place_on_field(2, ["ST1-03"])  # opponent unsuspended Digimon

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone and e.is_on_play
        ]
        assert len(on_play_effects) == 1, "Should have 1 On Play effect"

        effect = on_play_effects[0]
        deleted_perms = []

        def mock_select_opp(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.enemy.battle_area:
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return

        game.effect_select_opponent_permanent = mock_select_opp

        original_delete = game.player2.delete_permanent
        def track_delete(p):
            deleted_perms.append(p)
            original_delete(p)
        game.player2.delete_permanent = track_delete

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert target in deleted_perms, "Should delete opponent's unsuspended Digimon"

    def test_on_play_filter_excludes_suspended(self, debug_runner):
        """[On Play] Should NOT target opponent's suspended Digimon."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-057"])
        target = runner.place_on_field(2, ["ST1-03"], is_suspended=True)

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone and e.is_on_play
        ]
        effect = on_play_effects[0]

        selected = []
        def mock_select_opp(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.enemy.battle_area:
                if filter_fn and filter_fn(p):
                    selected.append(p)
        game.effect_select_opponent_permanent = mock_select_opp

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert target not in selected, "Suspended Digimon should NOT be a valid target"

    # ── When Attacking: Delete Unsuspended Digimon ──────────────────

    def test_when_attacking_delete_unsuspended(self, debug_runner):
        """[When Attacking] Should delete 1 of opponent's unsuspended Digimon."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-057"])
        target = runner.place_on_field(2, ["ST1-03"])  # unsuspended

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_attack_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack and e.is_on_attack
        ]
        assert len(on_attack_effects) == 1, "Should have 1 When Attacking effect"

        effect = on_attack_effects[0]
        # Check condition passes for own permanent
        assert effect.can_use_condition({
            'attacker': perm,
            'permanent': perm,
        }), "Condition should pass when this is the attacker"

    def test_when_attacking_condition_wrong_permanent(self, debug_runner):
        """[When Attacking] Condition should fail for a different attacker."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-057"])
        other_perm = runner.place_on_field(1, ["ST1-03"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_attack_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack and e.is_on_attack
        ]
        effect = on_attack_effects[0]

        assert not effect.can_use_condition({
            'attacker': other_perm,
            'permanent': other_perm,
        }), "Condition should fail when a different permanent attacks"

    # ── On Deletion: Place as Bottom Security ────────────────────────

    def test_on_deletion_condition_no_purple_face_up(self, debug_runner):
        """[On Deletion] Condition should pass when no purple face-up security."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-057"])

        game = runner.game
        p1 = game.player1
        card = perm.top_card

        # Clear face-up security (default security cards are face-down)
        p1.face_up_security.clear()

        effects = card.effect_list(None)
        on_deletion_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnDestroyedAnyone and e.is_on_deletion
        ]
        assert len(on_deletion_effects) == 1, "Should have 1 On Deletion effect"

        effect = on_deletion_effects[0]
        assert effect.can_use_condition({}), \
            "Condition should pass with no purple face-up security"

    def test_on_deletion_condition_with_purple_face_up(self, debug_runner):
        """[On Deletion] Condition should fail when a purple face-up security exists."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-057"])

        game = runner.game
        p1 = game.player1
        card = perm.top_card

        # Add a purple card to security as face-up
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        purple_card = db.create_card_source("EX10-057", p1)
        p1.security_cards.append(purple_card)
        p1.face_up_security.add(purple_card)

        effects = card.effect_list(None)
        on_deletion_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnDestroyedAnyone and e.is_on_deletion
        ]
        effect = on_deletion_effects[0]
        assert not effect.can_use_condition({}), \
            "Condition should fail with purple face-up security"

    def test_on_deletion_places_as_bottom_security(self, debug_runner):
        """[On Deletion] Should place this Digimon face up as bottom security card."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-057"])

        game = runner.game
        p1 = game.player1
        card = perm.top_card
        p1.face_up_security.clear()
        initial_security_count = len(p1.security_cards)

        effects = card.effect_list(None)
        on_deletion_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnDestroyedAnyone and e.is_on_deletion
        ]
        effect = on_deletion_effects[0]

        effect.on_process_callback({
            'player': p1,
            'game': game,
        })

        assert len(p1.security_cards) == initial_security_count + 1, \
            "Security should have 1 more card"
        assert p1.security_cards[-1] is card, \
            "The card should be at the bottom (appended) of security"
        assert card in p1.face_up_security, \
            "The card should be face up"

    # ── Security Effect: Play Lv5 Dark Masters Digimon ──────────────

    def test_security_effect_filter_requires_digimon(self, debug_runner):
        """[Security] Play filter must require Digimon type (not Options/Tamers)."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-057"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        security_effects = [
            e for e in effects
            if e.timing == EffectTiming.SecuritySkill
            and e.is_security_effect
            and e.is_inherited_effect
        ]
        assert len(security_effects) == 1, "Should have 1 security effect"

        effect = security_effects[0]
        # Mock the play_from_zone to capture the filter
        captured_filters = []
        def mock_play_from_zone(player, zone, filter_fn, free=True, is_optional=True, prompt=None):
            captured_filters.append(filter_fn)
        game.effect_play_from_zone = mock_play_from_zone

        # Make condition pass (card must be face-up in security)
        p1 = game.player1
        p1.face_up_security.add(card)

        effect.on_process_callback({
            'player': p1,
            'game': game,
        })

        assert len(captured_filters) == 1, "Should have called effect_play_from_zone"
        play_filter = captured_filters[0]

        # Create a mock Option card with Dark Masters text and level 5
        class MockCard:
            pass

        option_card = MockCard()
        option_card.is_digi_egg = False
        option_card.is_digimon = False
        option_card.is_option = True
        option_card.level = 4
        option_card.card_text = "Dark Masters related option"

        assert not play_filter(option_card), \
            "Options should NOT pass the filter (C# requires Digimon)"

        digimon_card = MockCard()
        digimon_card.is_digi_egg = False
        digimon_card.is_digimon = True
        digimon_card.level = 5
        digimon_card.card_text = "Dark Masters Digimon"

        assert play_filter(digimon_card), \
            "Lv5 Digimon with Dark Masters text should pass"

    def test_security_effect_filter_level_5_or_lower(self, debug_runner):
        """[Security] Play filter must require level 5 or lower."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-057"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        security_effects = [
            e for e in effects
            if e.timing == EffectTiming.SecuritySkill
            and e.is_security_effect
        ]
        effect = security_effects[0]

        captured_filters = []
        def mock_play_from_zone(player, zone, filter_fn, free=True, is_optional=True, prompt=None):
            captured_filters.append(filter_fn)
        game.effect_play_from_zone = mock_play_from_zone

        p1 = game.player1
        p1.face_up_security.add(card)

        effect.on_process_callback({
            'player': p1,
            'game': game,
        })

        play_filter = captured_filters[0]

        class MockCard:
            pass

        lv6_card = MockCard()
        lv6_card.is_digi_egg = False
        lv6_card.is_digimon = True
        lv6_card.level = 6
        lv6_card.card_text = "Dark Masters"

        assert not play_filter(lv6_card), \
            "Lv6 Digimon should NOT pass (must be level 5 or lower)"

    # ── All Turns: Digivolve Restriction ─────────────────────────────

    def test_has_digivolve_restriction_effect(self, debug_runner):
        """[All Turns] Should have a declarative effect about digivolve restriction."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-057"])

        card = perm.top_card
        effects = card.effect_list(None)
        # Look for a declarative effect about Apocalymon restriction
        restriction_effects = [
            e for e in effects
            if 'apocalymon' in (getattr(e, 'effect_name', '') or '').lower()
            or 'can only digivolve' in (getattr(e, 'effect_description', '') or '').lower()
        ]
        assert len(restriction_effects) >= 1, \
            "Should have a digivolve restriction effect mentioning Apocalymon"

    # ── End of Turn Deletion ─────────────────────────────────────────

    def test_eot_delete_condition_only_on_own_turn(self, debug_runner):
        """End of turn delete should only fire on the turn it was played (own turn)."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-057"])

        game = runner.game
        card = perm.top_card
        card._ex10_eot_delete = True

        effects = card.effect_list(None)
        eot_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn
            and 'delete' in (getattr(e, 'effect_name', '') or '').lower()
        ]
        assert len(eot_effects) == 1, "Should have 1 end-of-turn delete effect"

        effect = eot_effects[0]
        # P1 is turn player, so condition should pass
        game.player1.is_my_turn = True
        assert effect.can_use_condition({}), \
            "EOT delete condition should pass on own turn with flag set"

    def test_eot_delete_condition_fails_without_flag(self, debug_runner):
        """End of turn delete should NOT fire if _ex10_eot_delete is not set."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-057"])

        game = runner.game
        card = perm.top_card
        # Don't set _ex10_eot_delete

        effects = card.effect_list(None)
        eot_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn
            and 'delete' in (getattr(e, 'effect_name', '') or '').lower()
        ]
        assert len(eot_effects) == 1, "Should have 1 end-of-turn delete effect"
        effect = eot_effects[0]

        assert not effect.can_use_condition({}), \
            "EOT delete condition should fail without _ex10_eot_delete flag"
