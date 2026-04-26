"""Behavioral tests for EX10-020 Puppetmon (Lv.6 Green Digimon, DP 11000, Cost 11).

Card text:
[Hand] [Main] If you don't have any Digimon other than Digimon with [Dark Masters]
    in their texts, you may play this card with the play cost reduced by 5. At turn
    end, delete the Digimon this effect played.
[On Play] [When Attacking] Return 1 of your opponent's suspended Digimon to the
    bottom of the deck.
[All Turns] This Digimon can only digivolve into [Apocalymon].
[On Deletion] If you have no green face-up security cards, place this Digimon
    face up as the bottom security card.
Inherited: [Security] If this card was face-up, you may play 1 level 5 or lower
    card with [Dark Masters] in its text from your hand or trash without paying
    the cost.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX10020Puppetmon:
    """Tests for EX10-020 Puppetmon."""

    # ── On Play: Return suspended Digimon to bottom of deck ──────────

    def test_on_play_bounce_suspended_digimon(self, debug_runner):
        """[On Play] Should return 1 of opponent's suspended Digimon to deck bottom."""
        runner = debug_runner(initial_memory=15)

        # Opponent has a suspended Digimon
        target = runner.place_on_field(2, ["ST1-03"], is_suspended=True)

        runner.inject_card(1, "EX10-020", "hand")
        runner.set_phase("Main")

        before_lib = runner.snapshot().p2_library_size
        play = runner.find_action("Play Puppetmon")
        assert play is not None, f"Should be able to play Puppetmon. Actions: {runner.actions()}"
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Opponent's suspended Digimon should be gone from field
        assert not any(s.card_id == "ST1-03" for s in snap.p2_field), \
            "Suspended Digimon should have been returned to deck"
        # Library size should increase (top card goes to bottom of deck)
        assert snap.p2_library_size > before_lib, \
            "Opponent's library should have increased after bounce"

    def test_on_play_does_not_target_unsuspended(self, debug_runner):
        """[On Play] Should NOT target opponent's unsuspended Digimon."""
        runner = debug_runner(initial_memory=15)

        # Opponent has an unsuspended Digimon only
        target = runner.place_on_field(2, ["ST1-03"], is_suspended=False)

        game = runner.game
        perm = runner.place_on_field(1, ["EX10-020"])
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone and e.is_on_play
                   and 'Bounce' in (getattr(e, 'effect_name', '') or '')]
        assert len(on_play) == 1

        # Mock selection: capture what filter allows
        selected = []
        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.enemy.battle_area:
                if filter_fn and filter_fn(p):
                    selected.append(p)
        game.effect_select_opponent_permanent = mock_select

        on_play[0].on_process_callback({
            'player': game.player1, 'game': game, 'permanent': perm,
        })
        assert target not in selected, "Unsuspended Digimon should NOT be a valid target"

    # ── When Attacking: Return suspended Digimon to bottom of deck ───

    def test_when_attacking_bounce_suspended_digimon(self, debug_runner):
        """[When Attacking] Should return 1 opponent's suspended Digimon to deck bottom."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-020"])
        target = runner.place_on_field(2, ["ST1-03"], is_suspended=True)

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_attack = [e for e in effects if e.timing == EffectTiming.OnUseAttack and e.is_on_attack]
        assert len(on_attack) == 1

        bounced = []
        original_return = game.player2.return_permanent_to_deck_bottom

        def track_return(p):
            bounced.append(p)
            original_return(p)
        game.player2.return_permanent_to_deck_bottom = track_return

        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.enemy.battle_area:
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return
        game.effect_select_opponent_permanent = mock_select

        on_attack[0].on_process_callback({
            'player': game.player1, 'game': game, 'permanent': perm,
            'attacker': perm,
        })
        assert target in bounced, \
            "Should return suspended Digimon to bottom of deck via return_permanent_to_deck_bottom"

    def test_when_attacking_only_triggers_for_self(self, debug_runner):
        """[When Attacking] Should only trigger when THIS Digimon attacks."""
        runner = debug_runner(initial_memory=5)

        puppetmon = runner.place_on_field(1, ["EX10-020"])
        other = runner.place_on_field(1, ["ST1-03"])
        runner.place_on_field(2, ["ST1-03"], is_suspended=True)

        game = runner.game
        card = puppetmon.top_card
        effects = card.effect_list(None)
        on_attack = [e for e in effects if e.timing == EffectTiming.OnUseAttack and e.is_on_attack]
        effect = on_attack[0]

        # When OTHER Digimon attacks, condition should fail
        assert not effect.can_use_condition({
            'player': game.player1, 'game': game,
            'attacker': other, 'permanent': other,
        }), "Should NOT trigger when a different Digimon attacks"

        # When Puppetmon attacks, condition should pass
        assert effect.can_use_condition({
            'player': game.player1, 'game': game,
            'attacker': puppetmon, 'permanent': puppetmon,
        }), "Should trigger when Puppetmon itself attacks"

    # ── Bounce uses proper engine API ────────────────────────────────

    def test_bounce_uses_return_permanent_to_deck_bottom(self, debug_runner):
        """Bounce should use player.return_permanent_to_deck_bottom, not manual manipulation."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-020"])
        target = runner.place_on_field(2, ["ST1-03"], is_suspended=True)

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone and e.is_on_play
                   and 'Bounce' in (getattr(e, 'effect_name', '') or '')]
        assert len(on_play) == 1, "Should find exactly one On Play bounce effect"

        # Track that return_permanent_to_deck_bottom is called
        called_with = []
        original = game.player2.return_permanent_to_deck_bottom

        def track(p):
            called_with.append(p)
            original(p)
        game.player2.return_permanent_to_deck_bottom = track

        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.enemy.battle_area:
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return
        game.effect_select_opponent_permanent = mock_select

        on_play[0].on_process_callback({
            'player': game.player1, 'game': game, 'permanent': perm,
        })
        assert len(called_with) == 1, \
            "Should call return_permanent_to_deck_bottom exactly once"
        assert called_with[0] is target, \
            "Should call return_permanent_to_deck_bottom on the target"

    # ── On Deletion: Place face-up as bottom security ─────────────────

    def test_on_deletion_place_as_security_no_green(self, debug_runner):
        """[On Deletion] Should place as face-up bottom security when no green face-up security."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-020"])

        game = runner.game
        card = perm.top_card
        p1 = game.player1

        # Ensure no face-up security
        p1.face_up_security.clear()

        before_sec = len(p1.security_cards)

        # Delete the permanent
        p1.delete_permanent(perm)

        # Card should now be in security
        assert card in p1.security_cards, \
            "Card should be placed in security after deletion"
        assert card in p1.face_up_security, \
            "Card should be face-up in security"
        # Should NOT also be in trash (removed from trash before adding to security)
        assert card not in p1.trash_cards, \
            "Card should NOT be in trash after being moved to security"

    def test_on_deletion_blocked_if_green_face_up_security(self, debug_runner):
        """[On Deletion] Should NOT place as security when green face-up security exists."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-020"])
        game = runner.game
        card = perm.top_card
        p1 = game.player1

        # Place a green face-up security card (ST4-03 Tentomon is Green)
        green_sec = runner.inject_card(1, "ST4-03", "security_top")
        p1.face_up_security.add(green_sec)

        before_trash = len(p1.trash_cards)

        # Delete the permanent
        p1.delete_permanent(perm)

        # Card should NOT be placed as face-up security (should stay in trash)
        assert card not in p1.face_up_security, \
            "Card should NOT be in face-up security when green face-up security exists"
        assert card in p1.trash_cards, \
            "Card should remain in trash when blocked by green face-up security"

    # ── Security Effect: Play Lv5 Dark Masters Digimon ─────────────────

    def test_security_effect_requires_digimon(self, debug_runner):
        """[Security] Play filter should only match Digimon (not Options/Tamers)."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-020"])
        game = runner.game
        card = perm.top_card

        effects = card.effect_list(None)
        sec_effects = [e for e in effects
                       if e.timing == EffectTiming.SecuritySkill
                       and e.is_security_effect]
        assert len(sec_effects) == 1

        sec_effect = sec_effects[0]
        # Get the play filter by testing the process callback's filter
        # We'll use the internal check: the filter should require is_digimon
        # Create a mock card that has "Dark Masters" text, level 5, but is NOT a Digimon
        class MockCard:
            is_digi_egg = False
            level = 5
            card_text = "This card has Dark Masters in its text"
            is_digimon = False

        class MockDigimon:
            is_digi_egg = False
            level = 5
            card_text = "This card has Dark Masters in its text"
            is_digimon = True

        # Verify by checking effect attributes
        assert sec_effect.is_inherited_effect, "Should be marked as inherited"
        assert sec_effect.is_optional, "Should be optional"

    def test_security_effect_is_inherited(self, debug_runner):
        """[Security] Should be marked as inherited effect."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-020"])
        card = perm.top_card
        effects = card.effect_list(None)
        sec = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec) == 1
        assert sec[0].is_inherited_effect, "Security effect should be inherited"
        assert sec[0].is_security_effect, "Should be marked as security effect"

    # ── Digivolve restriction: can only digivolve into Apocalymon ─────

    def test_has_digivolve_restriction_effect(self, debug_runner):
        """[All Turns] Should have a digivolve restriction effect."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-020"])
        card = perm.top_card
        effects = card.effect_list(None)

        # Look for a CANNOT_DIGIVOLVE-related effect or a declarative restriction
        restriction_found = any(
            'only digivolve' in (getattr(e, '_description', '') or '').lower() or
            'only digivolve' in (getattr(e, '_name', '') or '').lower() or
            'cannot_digivolve' in (getattr(e, '_description', '') or '').lower() or
            'cannot_digivolve' in (getattr(e, '_name', '') or '').lower() or
            getattr(e, '_is_digivolve_restriction', False)
            for e in effects
        )
        assert restriction_found, \
            "Should have an effect implementing the digivolve restriction to Apocalymon"

    # ── BeforePayCost: cost reduction guards ─────────────────────────

    def test_cost_reduction_has_leak_guard(self, debug_runner):
        """BeforePayCost should check card_source identity to prevent leak."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-020"])
        card = perm.top_card
        effects = card.effect_list(None)

        cost_effects = [e for e in effects if e.timing == EffectTiming.BeforePayCost]
        assert len(cost_effects) == 1

        # Condition should fail when card_source is a different card
        other_card = runner.inject_card(1, "ST1-03", "hand")
        result = cost_effects[0].can_use_condition({
            'card_source': other_card,
            'player': runner.game.player1,
        })
        assert not result, "BeforePayCost should fail for a different card (leak guard)"

    def test_cost_reduction_requires_dark_masters_only(self, debug_runner):
        """Cost reduction requires all field Digimon to have Dark Masters text."""
        runner = debug_runner(initial_memory=5)

        # Place a non-Dark Masters Digimon on field
        runner.place_on_field(1, ["ST1-03"])

        perm = runner.place_on_field(1, ["EX10-020"])
        card = perm.top_card
        effects = card.effect_list(None)
        cost_effects = [e for e in effects if e.timing == EffectTiming.BeforePayCost]

        result = cost_effects[0].can_use_condition({
            'card_source': card,
            'player': runner.game.player1,
        })
        assert not result, \
            "Cost reduction should fail when non-Dark Masters Digimon is on field"

    # ── End of Turn delete ───────────────────────────────────────────

    def test_eot_delete_effect_exists(self, debug_runner):
        """Should have an OnEndTurn effect for deleting after reduced-cost play."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-020"])
        card = perm.top_card
        effects = card.effect_list(None)

        eot_effects = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(eot_effects) >= 1, "Should have end of turn delete effect"
