"""Behavioral tests for BT24-018 Styracomon (Lv.7 Red Dragonkin).

Card text:
<Progress> <Piercing> <Blocker> <Armor Purge>
[When Digivolving] You may trash any 1 of your opponent's security cards.
    Then, this Digimon may unsuspend.
[All Turns] [Once Per Turn] When your opponent's security stack is removed from,
    you may delete 1 of their Digimon.
[All Turns] [Once Per Turn] When any of your [Reptile] or [Dragonkin] trait
    Digimon would leave the battle area, by deleting 1 of your opponent's
    lowest DP Digimon, they don't leave.

Alt-digi: While you have [Owen Dreadnought], from [Lamiamon] for cost 6.

Evo Costs: Red Lv.6 for 4

CRITICAL BUGS verified:
1. Security trash must use selection (any card), not pop(0)
2. OnLoseSecurity must only fire when OPPONENT's security is removed
3. WhenRemoveField delete target must be lowest DP only
4. WhenRemoveField must NOT check is_opponent_effect (card text has no restriction)
5. WhenRemoveField must protect self too (Styracomon has Dragonkin trait)
6. Must use WhenPermanentWouldBeDeleted + _will_not_be_removed
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase


def _get_bt24_018_effects(perm):
    """Get all effects from BT24-018 top card only (not inherited from sources)."""
    top = perm.top_card
    if top is None:
        return []
    return top.effect_list(None)


def _find_effect_by_hash(perm, hash_string):
    """Find effect from BT24-018 by hash string."""
    for e in _get_bt24_018_effects(perm):
        if getattr(e, 'hash_string', None) == hash_string:
            return e
    return None


def _find_effect_by_name_substr(perm, substr):
    """Find effect from BT24-018 by substring in effect_name."""
    for e in _get_bt24_018_effects(perm):
        if e.effect_name and substr.lower() in e.effect_name.lower():
            return e
    return None


@pytest.mark.behavioral
class TestBT24018Keywords:
    """Test that all 4 keywords are registered."""

    def test_has_progress(self, debug_runner):
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT24-016", "BT24-018"])
        effects = _get_bt24_018_effects(perm)
        assert any(getattr(e, '_is_progress', False) for e in effects), \
            "Styracomon should have Progress keyword"

    def test_has_piercing(self, debug_runner):
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT24-016", "BT24-018"])
        effects = _get_bt24_018_effects(perm)
        assert any(getattr(e, '_is_piercing', False) for e in effects), \
            "Styracomon should have Piercing keyword"

    def test_has_blocker(self, debug_runner):
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT24-016", "BT24-018"])
        effects = _get_bt24_018_effects(perm)
        assert any(getattr(e, '_is_blocker', False) for e in effects), \
            "Styracomon should have Blocker keyword"

    def test_has_armor_purge(self, debug_runner):
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT24-016", "BT24-018"])
        effects = _get_bt24_018_effects(perm)
        assert any(getattr(e, '_is_armor_purge', False) for e in effects), \
            "Styracomon should have Armor Purge keyword"


@pytest.mark.behavioral
class TestBT24018WhenDigivolving:
    """[When Digivolving] You may trash any 1 of your opponent's security cards.
    Then, this Digimon may unsuspend."""

    def _get_digivolve_effect(self, perm):
        """Find the BT24-018 When Digivolving effect."""
        effects = _get_bt24_018_effects(perm)
        digivolve_effects = [
            e for e in effects
            if getattr(e, 'is_when_digivolving', False)
            and 'BT24-018' in (e.effect_name or '')
        ]
        assert len(digivolve_effects) == 1, \
            f"Should have exactly 1 When Digivolving effect from BT24-018, found: {[(e.effect_name) for e in digivolve_effects]}"
        return digivolve_effects[0]

    def test_digivolve_trash_security_uses_selection(self, debug_runner):
        """BUG #1: The security trash must use opponent security selection,
        not just pop index 0. The C# uses SelectCardEffect with SecurityCards."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT24-016", "BT24-018"])
        game = runner.game

        # Give opponent 3 security cards
        runner.inject_card(2, "ST1-02", "security_top")
        runner.inject_card(2, "ST1-03", "security_top")
        runner.inject_card(2, "ST1-07", "security_top")

        effect = self._get_digivolve_effect(perm)
        sec_before = len(game.player2.security_cards)

        # Execute the effect
        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # The first step is a "Yes/No" branch choice for trashing.
        # If Yes is chosen, it should then enter SelectSecurity to let
        # the player choose WHICH card to trash.
        phase = game.current_phase
        assert phase in (
            GamePhase.SelectEffectChoice,
            GamePhase.SelectSecurity,
        ), f"Expected branch choice or security selection phase, got {phase}"

        # Choose "Yes" if we're at branch choice
        if phase == GamePhase.SelectEffectChoice:
            ps = game.pending_selection
            if ps:
                # First option = Yes
                game.decode_action(ps.valid_indices[0], game.current_player_id)

        # Now should be in SelectSecurity phase (picking WHICH card to trash)
        assert game.current_phase == GamePhase.SelectSecurity, \
            f"After accepting, should be in SelectSecurity to pick which card. Got {game.current_phase}"

    def test_digivolve_decline_trash_still_offers_unsuspend(self, debug_runner):
        """Even if you decline the security trash, you should still be offered
        the unsuspend choice."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT24-016", "BT24-018"], is_suspended=True)
        game = runner.game

        runner.inject_card(2, "ST1-02", "security_top")
        sec_before = len(game.player2.security_cards)

        effect = self._get_digivolve_effect(perm)
        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Decline trash (branch choice - last option = No)
        if game.current_phase == GamePhase.SelectEffectChoice:
            ps = game.pending_selection
            if ps:
                no_action = ps.valid_indices[-1]
                game.decode_action(no_action, game.current_player_id)

        # Security should be unchanged
        assert len(game.player2.security_cards) == sec_before, \
            "Declining trash should not remove any security"

        # Should be asked about unsuspend since Digimon is suspended
        assert game.current_phase == GamePhase.SelectEffectChoice, \
            "Should offer unsuspend choice after declining trash"

    def test_digivolve_no_enemy_security_still_offers_unsuspend(self, debug_runner):
        """If opponent has 0 security, skip trash, still offer unsuspend."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT24-016", "BT24-018"], is_suspended=True)
        game = runner.game

        # Clear opponent security
        game.player2.security_cards.clear()

        effect = self._get_digivolve_effect(perm)
        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Should immediately offer unsuspend since no security to trash
        assert game.current_phase == GamePhase.SelectEffectChoice, \
            "Should offer unsuspend choice when no enemy security"


@pytest.mark.behavioral
class TestBT24018OnLoseSecurity:
    """[All Turns] [Once Per Turn] When your opponent's security stack is
    removed from, you may delete 1 of their Digimon."""

    def _get_on_lose_security_effect(self, perm):
        """Find the BT24-018 OnLoseSecurity effect by hash."""
        effect = _find_effect_by_hash(perm, "BT24_18_AT_Sec_Removed")
        assert effect is not None, \
            "Should find BT24-018 OnLoseSecurity effect by hash string"
        return effect

    def test_fires_when_opponent_loses_security(self, debug_runner):
        """BUG #2: The effect should fire when the OPPONENT's security is removed.
        In execute_effects, 'player' becomes the effect owner (player1),
        and 'event_player' becomes the player from extra_context (who lost security)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT24-016", "BT24-018"])
        game = runner.game

        effect = self._get_on_lose_security_effect(perm)

        # Context where OPPONENT (player2) lost security
        context = {
            'game': game,
            'player': game.player1,  # owner of the effect
            'permanent': perm,
            'event_player': game.player2,  # opponent lost security
        }
        assert effect.can_use_condition(context), \
            "Effect should trigger when opponent's security is removed"

    def test_does_not_fire_when_own_security_removed(self, debug_runner):
        """BUG #2: Must NOT fire when our OWN security is removed."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT24-016", "BT24-018"])
        game = runner.game

        effect = self._get_on_lose_security_effect(perm)

        # Context where OWN player (player1) lost security
        context = {
            'game': game,
            'player': game.player1,  # owner of the effect
            'permanent': perm,
            'event_player': game.player1,  # OWN security lost
        }
        assert not effect.can_use_condition(context), \
            "Effect must NOT trigger when own security is removed"

    def test_is_optional(self, debug_runner):
        """The effect is optional (you MAY delete)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT24-016", "BT24-018"])
        effect = self._get_on_lose_security_effect(perm)
        assert effect.is_optional, "OnLoseSecurity delete effect should be optional"

    def test_once_per_turn(self, debug_runner):
        """The effect has [Once Per Turn] restriction."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT24-016", "BT24-018"])
        effect = self._get_on_lose_security_effect(perm)
        assert effect.max_count_per_turn == 1, "Should be Once Per Turn"


@pytest.mark.behavioral
class TestBT24018PreventLeaving:
    """[All Turns] [Once Per Turn] When any of your [Reptile] or [Dragonkin]
    trait Digimon would leave the battle area, by deleting 1 of your opponent's
    lowest DP Digimon, they don't leave."""

    def _get_prevent_effect(self, perm):
        """Find the prevent-leaving effect by hash string."""
        effect = _find_effect_by_hash(perm, "BT24_018_AT_Prevent_Deletion")
        assert effect is not None, \
            f"Should find prevent-leaving effect by hash. All effects: {[(e.effect_name, getattr(e, '_hash_string', None)) for e in _get_bt24_018_effects(perm)]}"
        return effect

    def test_correct_timing(self, debug_runner):
        """BUG #6: Should use WhenPermanentWouldBeDeleted timing (fires BEFORE
        deletion), not WhenRemoveField (fires AFTER, too late to prevent)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT24-016", "BT24-018"])
        effect = self._get_prevent_effect(perm)
        assert effect.timing == EffectTiming.WhenPermanentWouldBeDeleted, \
            f"Expected WhenPermanentWouldBeDeleted timing, got {effect.timing}"

    def test_triggers_for_dragonkin(self, debug_runner):
        """Should trigger when a Dragonkin Digimon would be deleted."""
        runner = debug_runner(initial_memory=10)
        styracomon = runner.place_on_field(1, ["BT24-016", "BT24-018"])

        # Place another Dragonkin on our field (Lamiamon is Dragonkin)
        dragonkin = runner.place_on_field(1, ["BT24-016"])
        game = runner.game

        # Place an opponent Digimon to be deleted as cost
        runner.place_on_field(2, ["ST1-07"])  # Greymon Lv4 4000 DP

        effect = self._get_prevent_effect(styracomon)
        context = {
            'game': game,
            'player': game.player1,
            'permanent': dragonkin,  # the Digimon being deleted
        }
        assert effect.can_use_condition(context), \
            "Should trigger for Dragonkin Digimon being deleted"

    def test_triggers_for_reptile(self, debug_runner):
        """Should trigger when a Reptile Digimon would be deleted."""
        runner = debug_runner(initial_memory=10)
        styracomon = runner.place_on_field(1, ["BT24-016", "BT24-018"])

        # ST1-03 Agumon is Reptile trait
        reptile = runner.place_on_field(1, ["ST1-03"])
        game = runner.game

        runner.place_on_field(2, ["ST1-07"])

        effect = self._get_prevent_effect(styracomon)
        context = {
            'game': game,
            'player': game.player1,
            'permanent': reptile,
        }
        assert effect.can_use_condition(context), \
            "Should trigger for Reptile Digimon being deleted"

    def test_does_not_trigger_for_non_reptile_non_dragonkin(self, debug_runner):
        """Should NOT trigger for a Digimon without Reptile/Dragonkin trait."""
        runner = debug_runner(initial_memory=10)
        styracomon = runner.place_on_field(1, ["BT24-016", "BT24-018"])

        # ST1-02 Biyomon is Bird trait (not Reptile/Dragonkin)
        bird = runner.place_on_field(1, ["ST1-02"])
        game = runner.game

        runner.place_on_field(2, ["ST1-07"])

        effect = self._get_prevent_effect(styracomon)
        context = {
            'game': game,
            'player': game.player1,
            'permanent': bird,
        }
        assert not effect.can_use_condition(context), \
            "Should NOT trigger for Bird trait Digimon"

    def test_no_is_opponent_effect_gate(self, debug_runner):
        """BUG #4: Should trigger regardless of whether the deletion is from
        an opponent's effect or own effect. Card text says 'would leave the
        battle area' with no restriction on cause."""
        runner = debug_runner(initial_memory=10)
        styracomon = runner.place_on_field(1, ["BT24-016", "BT24-018"])
        reptile = runner.place_on_field(1, ["ST1-03"])  # Reptile
        game = runner.game
        runner.place_on_field(2, ["ST1-07"])

        effect = self._get_prevent_effect(styracomon)

        # No is_opponent_effect in context should still trigger
        context_no_flag = {
            'game': game,
            'player': game.player1,
            'permanent': reptile,
        }
        assert effect.can_use_condition(context_no_flag), \
            "Should trigger without is_opponent_effect flag"

        context_own_effect = {
            'game': game,
            'player': game.player1,
            'permanent': reptile,
            'is_opponent_effect': False,
        }
        assert effect.can_use_condition(context_own_effect), \
            "Should trigger even when is_opponent_effect is False"

    def test_protects_self(self, debug_runner):
        """BUG #5: Card says 'any of your [Reptile] or [Dragonkin] trait Digimon'
        which includes Styracomon itself (it has Dragonkin). The C# uses
        IsPermanentExistsOnOwnerBattleArea which includes the card itself."""
        runner = debug_runner(initial_memory=10)
        styracomon = runner.place_on_field(1, ["BT24-016", "BT24-018"])
        game = runner.game
        runner.place_on_field(2, ["ST1-07"])

        effect = self._get_prevent_effect(styracomon)

        # Styracomon being deleted itself should trigger the effect
        context = {
            'game': game,
            'player': game.player1,
            'permanent': styracomon,  # Styracomon itself is being deleted
        }
        assert effect.can_use_condition(context), \
            "Effect should protect Styracomon itself (has Dragonkin trait)"

    def test_delete_target_lowest_dp_only(self, debug_runner):
        """BUG #3: Card text says 'lowest DP Digimon'. The delete target
        must be restricted to opponent's Digimon with the lowest DP."""
        runner = debug_runner(initial_memory=10)
        styracomon = runner.place_on_field(1, ["BT24-016", "BT24-018"])
        reptile = runner.place_on_field(1, ["ST1-03"])  # Reptile (to trigger)
        game = runner.game

        # Place 2 opponent Digimon with different DP
        low_dp = runner.place_on_field(2, ["ST1-03"])   # Agumon 2000 DP
        high_dp = runner.place_on_field(2, ["ST1-07"])  # Greymon 4000 DP

        effect = self._get_prevent_effect(styracomon)

        # Execute the process callback
        effect.on_process_callback({
            'game': game,
            'player': game.player1,
            'permanent': reptile,
        })

        # Should enter a selection phase for opponent's permanent
        if game.current_phase == GamePhase.SelectTarget:
            ps = game.pending_selection
            assert ps is not None, "Should have pending selection"
            # Only the lowest DP Digimon (Agumon 2000) should be selectable
            assert len(ps.valid_indices) == 1, \
                f"Only lowest DP Digimon should be selectable, got {len(ps.valid_indices)} options"

    def test_once_per_turn(self, debug_runner):
        """Effect has [Once Per Turn] restriction."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT24-016", "BT24-018"])
        effect = self._get_prevent_effect(perm)
        assert effect.max_count_per_turn == 1, "Should be Once Per Turn"

    def test_is_optional(self, debug_runner):
        """Effect is optional (player can choose not to activate)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT24-016", "BT24-018"])
        effect = self._get_prevent_effect(perm)
        assert effect.is_optional, "Prevent-leaving effect should be optional"

    def test_prevention_sets_will_not_be_removed(self, debug_runner):
        """When the cost is paid (deleting opponent lowest DP Digimon),
        the leaving permanent should have _will_not_be_removed set to True."""
        runner = debug_runner(initial_memory=10)
        styracomon = runner.place_on_field(1, ["BT24-016", "BT24-018"])
        reptile = runner.place_on_field(1, ["ST1-03"])  # Reptile
        game = runner.game

        # Single opponent Digimon (will be auto-selected if only 1 option)
        opp = runner.place_on_field(2, ["ST1-03"])  # 2000 DP

        effect = self._get_prevent_effect(styracomon)
        effect.on_process_callback({
            'game': game,
            'player': game.player1,
            'permanent': reptile,
        })

        # Auto-resolve the selection
        runner.auto_resolve(max_steps=5)

        # The reptile permanent should be protected
        assert getattr(reptile, '_will_not_be_removed', False), \
            "After paying cost, the leaving permanent should have _will_not_be_removed = True"
