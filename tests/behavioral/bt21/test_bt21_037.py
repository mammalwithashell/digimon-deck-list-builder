"""Behavioral tests for BT21-037 Lighdramon (Lv.4 Blue Digimon).

Card text:
<Piercing>, <Armor Purge>
[When Digivolving] Suspend 1 of your opponent's Digimon. Then, this Digimon
gets +2000 DP until your opponent's turn ends.
Alt-digi: From [Veemon] for cost 2
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT21_037Lighdramon:
    """Tests for BT21-037 Lighdramon."""

    # ---------------------------------------------------------------
    # Clause 1: Keywords (Piercing, Armor Purge)
    # ---------------------------------------------------------------

    def test_piercing_keyword(self, debug_runner):
        """Lighdramon should have the Piercing keyword."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT2-021", "BT21-037"])

        top = perm.top_card
        effects = top.effect_list(None)
        has_piercing = any(getattr(e, '_is_piercing', False) for e in effects)
        assert has_piercing, "Lighdramon should have Piercing"

    def test_armor_purge_keyword(self, debug_runner):
        """Lighdramon should have the Armor Purge keyword."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT2-021", "BT21-037"])

        top = perm.top_card
        effects = top.effect_list(None)
        has_armor_purge = any(getattr(e, '_is_armor_purge', False) for e in effects)
        assert has_armor_purge, "Lighdramon should have Armor Purge"

    # ---------------------------------------------------------------
    # Clause 2: Alt-digi from [Veemon] for cost 2
    # ---------------------------------------------------------------

    def test_alt_digi_attributes(self, debug_runner):
        """Lighdramon should have alt-digi from Veemon cost 2."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT2-021", "BT21-037"])

        top = perm.top_card
        effects = top.effect_list(None)
        alt_digi = [e for e in effects if getattr(e, '_alt_digi_name', None)]
        assert len(alt_digi) == 1, "Should have exactly 1 alt-digi effect"
        assert alt_digi[0]._alt_digi_name == "Veemon"
        assert alt_digi[0]._alt_digi_cost == 2

    # ---------------------------------------------------------------
    # Clause 3: [When Digivolving] Suspend opponent Digimon + DP buff
    # ---------------------------------------------------------------

    def test_when_digivolving_effect_exists(self, debug_runner):
        """Lighdramon should have a When Digivolving effect."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT2-021", "BT21-037"])

        top = perm.top_card
        effects = top.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        assert len(wd) == 1, "Should have exactly 1 When Digivolving effect"

    def test_when_digivolving_suspend_opponent_digimon(self, debug_runner):
        """When Digivolving should suspend an opponent's Digimon."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT2-021", "BT21-037"])

        # Place unsuspended opponent Digimon
        opp = runner.place_on_field(2, ["ST1-03"])
        assert not opp.is_suspended, "Opponent Digimon should start unsuspended"

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        # Execute the WD effect
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'card': top,
        })

        # Should be in selection phase
        snap = runner.snapshot()
        assert snap.phase == "SelectTarget", "Should enter target selection for suspend"

        # Select the opponent Digimon
        legal = runner.action_mask()
        target_actions = [a for a in legal if a != 62]  # exclude pass
        assert target_actions, "Should have selectable targets"
        runner.execute(target_actions[0])
        runner.auto_resolve()

        # Opponent Digimon should now be suspended
        assert opp.is_suspended, "Opponent's Digimon should be suspended"

    def test_when_digivolving_dp_buff(self, debug_runner):
        """When Digivolving should grant +2000 DP until opponent's turn ends."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT2-021", "BT21-037"])

        # Place opponent Digimon for suspend target
        runner.place_on_field(2, ["ST1-03"])

        game = runner.game
        base_dp = perm.dp  # Should be 6000
        assert base_dp == 6000, f"Base DP should be 6000, got {base_dp}"

        top = perm.top_card
        effects = top.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        # Execute the WD effect
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'card': top,
        })

        # Resolve the selection
        runner.auto_resolve()

        # DP should be +2000
        assert perm.dp == 8000, f"DP should be 8000 after buff, got {perm.dp}"

    def test_dp_buff_still_applies_when_no_suspend_targets(self, debug_runner):
        """DP +2000 should apply even if all opponent Digimon are already suspended."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT2-021", "BT21-037"])

        # Place an already-suspended opponent Digimon
        opp = runner.place_on_field(2, ["ST1-03"], is_suspended=True)

        game = runner.game
        base_dp = perm.dp

        top = perm.top_card
        effects = top.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        # Execute the WD effect -- no valid suspend targets (all suspended)
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'card': top,
        })
        runner.auto_resolve()

        # DP buff should still apply
        assert perm.dp == base_dp + 2000, (
            f"DP should be {base_dp + 2000} even with no suspend targets, got {perm.dp}"
        )

    def test_dp_buff_applies_when_no_opponent_digimon(self, debug_runner):
        """DP +2000 should apply even if opponent has no Digimon at all."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT2-021", "BT21-037"])

        # No opponent Digimon
        game = runner.game
        base_dp = perm.dp

        top = perm.top_card
        effects = top.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'card': top,
        })
        runner.auto_resolve()

        assert perm.dp == base_dp + 2000, (
            f"DP should be {base_dp + 2000} even with no opponent Digimon, got {perm.dp}"
        )

    def test_dp_buff_expiry_end_of_opponent_turn(self, debug_runner):
        """The +2000 DP modifier should have 'end_of_opponent_turn' expiry."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT2-021", "BT21-037"])

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'card': top,
        })
        runner.auto_resolve()

        # Check modifier registry for the DP modifier
        from digimon_gym.engine.interfaces.modifiers import ModifierType
        dp_mods = game.modifiers._modifiers.get(ModifierType.CHANGE_DP, [])
        lighdramon_mods = [m for m in dp_mods if m.source_permanent is perm]
        assert lighdramon_mods, "Should have a CHANGE_DP modifier registered"
        assert lighdramon_mods[0].expiry == 'end_of_opponent_turn', (
            f"DP modifier expiry should be 'end_of_opponent_turn', got '{lighdramon_mods[0].expiry}'"
        )

    def test_when_digivolving_requires_field_presence(self, debug_runner):
        """WD condition should require the card to be on field."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT2-021", "BT21-037"])

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        # On field: should pass
        result = wd.can_use_condition({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        assert result is True, "WD condition should pass when on field"

        # Off field: should fail
        game.player1.battle_area.remove(perm)
        result = wd.can_use_condition({
            'player': game.player1,
            'game': game,
        })
        assert result is False, "WD condition should fail when not on field"
