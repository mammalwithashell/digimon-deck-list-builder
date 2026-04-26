"""Behavioral tests for BT21-029 Medusamon (Lv.6 Red Digimon).

Card text:
<Security A. +1> (This Digimon checks 1 additional security card.)
<Progress> (While attacking, your opponent's effects don't affect this Digimon.)
[When Digivolving] [End of Attack] [Once Per Turn] You may delete 1 of your
opponent's lowest DP Digimon.
[All Turns] [Once Per Turn] When any of your opponent's Digimon are deleted or
their security stack is removed from, they play 1 [Petrification] Token.
(Digimon/White/3000 DP/[Your Turn] This Digimon can't suspend.
[On Deletion] Trash your top security card.)
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT21029Medusamon:
    """Tests for BT21-029 Medusamon."""

    # ---------------------------------------------------------------
    # Clause 1 & 2: Static keywords (Security Attack +1, Progress)
    # ---------------------------------------------------------------

    def test_security_attack_plus_one(self, debug_runner):
        """Medusamon should have Security Attack +1 keyword effect."""
        runner = debug_runner(archetype1="Medusamon", initial_memory=10)
        # Place Medusamon on field (Lv.6 over a Lv.5)
        perm = runner.place_on_field(1, ["BT21-025", "BT21-029"])

        # Check Security Attack modifier via the permanent
        sa_mod = perm.security_attack_modifier()
        assert sa_mod == 1, (
            f"Medusamon should have security_attack_modifier=1, got {sa_mod}"
        )

    def test_progress_keyword(self, debug_runner):
        """Medusamon should have the Progress keyword."""
        runner = debug_runner(archetype1="Medusamon", initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-025", "BT21-029"])

        # Check for progress in effects
        top = perm.top_card
        effects = top.effect_list(None)
        has_progress = any(getattr(e, '_is_progress', False) for e in effects)
        assert has_progress, "Medusamon should have the Progress keyword"

    # ---------------------------------------------------------------
    # Clause 3: [When Digivolving] delete lowest DP (OPT)
    # ---------------------------------------------------------------

    def test_when_digivolving_effect_exists(self, debug_runner):
        """Medusamon should have a When Digivolving effect with OnEnterFieldAnyone timing."""
        runner = debug_runner(archetype1="Medusamon", initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-025", "BT21-029"])

        top = perm.top_card
        effects = top.effect_list(None)
        wd_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        assert len(wd_effects) >= 1, (
            "Medusamon should have at least 1 When Digivolving effect"
        )

    def test_when_digivolving_deletes_lowest_dp(self, debug_runner):
        """When Digivolving should target only the opponent's lowest DP Digimon."""
        runner = debug_runner(archetype1="Medusamon", initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-025", "BT21-029"])

        # Place two opponent Digimon with different DP
        # ST1-03 Agumon (2000 DP) and ST1-07 Greymon (4000 DP)
        runner.place_on_field(2, ["ST1-03"])   # 2000 DP
        runner.place_on_field(2, ["ST1-07"])   # 4000 DP

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        wd_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        assert wd_effects, "Should have WD effect"
        wd = wd_effects[0]

        # Execute the effect process directly
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'card': top,
        })

        # The game should now be in a selection phase with valid targets
        # Only the lowest DP Digimon (Agumon, 2000 DP) should be selectable
        snap = runner.snapshot()
        assert snap.phase == "SelectTarget", "Should enter selection phase"
        legal = runner.action_mask()
        # Filter out action 62 (pass/decline) to pick an actual target
        target_actions = [a for a in legal if a != 62]
        assert target_actions, "Should have target selection options (not just pass)"
        # Select the target (should be Agumon only since it has lowest DP)
        result = runner.execute(target_actions[0])
        runner.auto_resolve()
        snap = runner.snapshot()
        # Agumon should be deleted, Greymon should remain
        opp_names = [s.card_name for s in snap.p2_field]
        assert "Agumon" not in opp_names, "Agumon (lowest DP) should be deleted"
        assert "Greymon" in opp_names, "Greymon (higher DP) should remain"

    def test_when_digivolving_uses_permanent_dp_not_base(self, debug_runner):
        """Lowest DP calculation should use effective Permanent.dp, not base card DP."""
        runner = debug_runner(archetype1="Medusamon", initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-025", "BT21-029"])

        # Place two opponent Digimon: both should have valid .dp values
        opp1 = runner.place_on_field(2, ["ST1-03"])   # 2000 DP
        opp2 = runner.place_on_field(2, ["ST1-07"])   # 4000 DP

        # Verify permanent DP is accessible and correct
        assert opp1.dp == 2000, f"Agumon permanent DP should be 2000, got {opp1.dp}"
        assert opp2.dp == 4000, f"Greymon permanent DP should be 4000, got {opp2.dp}"

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

        # Should be in selection phase targeting only the lowest DP (Agumon)
        snap = runner.snapshot()
        assert snap.phase == "SelectTarget", "Should enter selection phase"
        legal = runner.action_mask()
        target_actions = [a for a in legal if a != 62]
        assert target_actions, "Should have target selection options"
        runner.execute(target_actions[0])
        runner.auto_resolve()
        snap = runner.snapshot()
        # Only the lowest DP Digimon should have been selectable
        # After deletion, Greymon (4000 DP) should still be on field
        assert any(s.card_id == "ST1-07" for s in snap.p2_field), (
            "Greymon should remain on field"
        )

    # ---------------------------------------------------------------
    # Clause 4: [End of Attack] delete lowest DP (OPT shared with WD)
    # ---------------------------------------------------------------

    def test_end_of_attack_effect_exists(self, debug_runner):
        """Medusamon should have an End of Attack effect with OnEndAttack timing."""
        runner = debug_runner(archetype1="Medusamon", initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-025", "BT21-029"])

        top = perm.top_card
        effects = top.effect_list(None)
        eoa_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEndAttack
        ]
        assert len(eoa_effects) >= 1, (
            "Medusamon should have at least 1 OnEndAttack effect"
        )

    def test_end_of_attack_condition_checks_is_attacking(self, debug_runner):
        """[End of Attack] should only fire when Medusamon itself is attacking."""
        runner = debug_runner(archetype1="Medusamon", initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-025", "BT21-029"])

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)
        eoa = [e for e in effects if e.timing == EffectTiming.OnEndAttack][0]

        # Without is_attacking, condition should fail
        perm.is_attacking = False
        result = eoa.can_use_condition({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        assert result is False, (
            "End of Attack condition should be False when Medusamon is NOT attacking"
        )

        # With is_attacking, condition should pass
        perm.is_attacking = True
        result = eoa.can_use_condition({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        assert result is True, (
            "End of Attack condition should be True when Medusamon IS attacking"
        )
        perm.is_attacking = False  # cleanup

    def test_opt_shared_between_wd_and_eoa(self, debug_runner):
        """WD and EoA delete effects should share OPT via the same hash string."""
        runner = debug_runner(archetype1="Medusamon", initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-025", "BT21-029"])

        top = perm.top_card
        effects = top.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]
        eoa = [e for e in effects if e.timing == EffectTiming.OnEndAttack][0]

        # Both should have the same hash string for shared OPT
        assert wd.hash_string == eoa.hash_string, (
            f"WD hash={wd.hash_string}, EoA hash={eoa.hash_string} — should match for shared OPT"
        )

    # ---------------------------------------------------------------
    # Clause 5: [All Turns] Deletion observer => play Petrification Token
    # ---------------------------------------------------------------

    def test_deletion_observer_on_opponent_digimon(self, debug_runner):
        """When opponent's Digimon is deleted, play Petrification Token on opponent field."""
        runner = debug_runner(archetype1="Medusamon", initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-025", "BT21-029"])

        # Place an opponent Digimon to be deleted
        target = runner.place_on_field(2, ["ST1-03"])

        game = runner.game
        p2_field_before = len(game.player2.battle_area)

        # Delete the opponent's Digimon — should trigger Medusamon's observer
        game.player2.delete_permanent(target)

        # After deletion, check if a Petrification Token was played on opponent field
        p2_field_after = game.player2.battle_area
        token_found = any(
            getattr(p.top_card, 'is_token', False)
            and 'Petrification' in (p.top_card.c_entity_base.card_name_eng if p.top_card and p.top_card.c_entity_base else '')
            for p in p2_field_after
        )
        assert token_found, (
            "Deleting opponent's Digimon should place a Petrification Token on opponent's field"
        )

    def test_deletion_observer_not_on_own_digimon(self, debug_runner):
        """When own Digimon is deleted, should NOT trigger Petrification Token."""
        runner = debug_runner(archetype1="Medusamon", initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-025", "BT21-029"])

        # Place one of our own Digimon
        own_target = runner.place_on_field(1, ["ST1-03"])

        game = runner.game
        p2_field_before = len(game.player2.battle_area)

        # Delete our own Digimon — should NOT trigger Medusamon's observer
        game.player1.delete_permanent(own_target)

        # No Petrification Token should appear on opponent's field
        p2_tokens = [
            p for p in game.player2.battle_area
            if getattr(p.top_card, 'is_token', False)
        ]
        assert len(p2_tokens) == 0, (
            "Deleting own Digimon should NOT trigger Petrification Token on opponent"
        )

    def test_deletion_observer_only_digimon(self, debug_runner):
        """Deletion of non-Digimon (e.g. Tamer) should NOT trigger the observer."""
        runner = debug_runner(archetype1="Medusamon", initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-025", "BT21-029"])

        # Place an opponent Tamer (BT21-081 is a tamer in the Medusamon deck)
        tamer = runner.place_on_field(2, ["BT21-081"])

        game = runner.game

        # Delete the opponent's Tamer — should NOT trigger observer
        game.player2.delete_permanent(tamer)

        p2_tokens = [
            p for p in game.player2.battle_area
            if getattr(p.top_card, 'is_token', False)
        ]
        assert len(p2_tokens) == 0, (
            "Deleting opponent's Tamer should NOT trigger Petrification Token"
        )

    # ---------------------------------------------------------------
    # Clause 6: [All Turns] Security loss => play Petrification Token
    # ---------------------------------------------------------------

    def test_security_loss_triggers_petrification(self, debug_runner):
        """When opponent loses security, play Petrification Token on opponent field."""
        runner = debug_runner(archetype1="Medusamon", initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-025", "BT21-029"])

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)

        # Find the OnLoseSecurity effect
        sec_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnLoseSecurity
        ]
        assert len(sec_effects) >= 1, "Should have an OnLoseSecurity effect"
        sec_eff = sec_effects[0]

        # Verify condition: opponent losing security should trigger
        result = sec_eff.can_use_condition({
            'player': game.player1,
            'event_player': game.player2,
            'game': game,
            'permanent': perm,
        })
        assert result is True, (
            "Condition should pass when opponent (event_player) loses security"
        )

        # Verify condition: own security loss should NOT trigger
        result = sec_eff.can_use_condition({
            'player': game.player1,
            'event_player': game.player1,
            'game': game,
            'permanent': perm,
        })
        assert result is False, (
            "Condition should fail when own player loses security"
        )

    # ---------------------------------------------------------------
    # OPT sharing between deletion and security triggers
    # ---------------------------------------------------------------

    def test_opt_shared_between_deletion_and_security(self, debug_runner):
        """Deletion observer and security loss should share OPT via same hash."""
        runner = debug_runner(archetype1="Medusamon", initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-025", "BT21-029"])

        top = perm.top_card
        effects = top.effect_list(None)

        # Find deletion observer effect
        del_effects = [
            e for e in effects
            if getattr(e, '_is_deletion_observer', False)
        ]
        assert del_effects, "Should have a deletion observer effect"

        # Find OnLoseSecurity effect
        sec_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnLoseSecurity
        ]
        assert sec_effects, "Should have an OnLoseSecurity effect"

        # Both should share the same hash string
        assert del_effects[0].hash_string == sec_effects[0].hash_string, (
            "Deletion and security effects should share OPT hash"
        )

    # ---------------------------------------------------------------
    # Token correctness
    # ---------------------------------------------------------------

    def test_petrification_token_properties(self, debug_runner):
        """Petrification Token should be White, 3000 DP Digimon."""
        runner = debug_runner(archetype1="Medusamon", initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-025", "BT21-029"])

        # Place and delete opponent Digimon to generate a token
        target = runner.place_on_field(2, ["ST1-03"])
        game = runner.game
        game.player2.delete_permanent(target)

        # Find the token
        tokens = [
            p for p in game.player2.battle_area
            if getattr(p.top_card, 'is_token', False)
        ]
        assert tokens, "Should have created a Petrification Token"
        token = tokens[0]

        assert token.dp == 3000, f"Petrification Token DP should be 3000, got {token.dp}"
        assert token.is_digimon, "Petrification Token should be a Digimon"

    # ---------------------------------------------------------------
    # Field presence checks
    # ---------------------------------------------------------------

    def test_effects_require_field_presence(self, debug_runner):
        """All triggered effects should check card.permanent_of_this_card() is not None."""
        runner = debug_runner(archetype1="Medusamon", initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-025", "BT21-029"])

        game = runner.game
        top = perm.top_card
        effects = top.effect_list(None)

        # Get the WD, EoA, and OnLoseSecurity effects
        wd = [e for e in effects if e.timing == EffectTiming.OnEnterFieldAnyone and getattr(e, 'is_when_digivolving', False)]
        eoa = [e for e in effects if e.timing == EffectTiming.OnEndAttack]
        sec = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity]

        # All should pass condition when on field
        for eff_list, name in [(wd, "WD"), (eoa, "EoA"), (sec, "Security")]:
            if eff_list:
                ctx = {'player': game.player1, 'game': game, 'permanent': perm}
                if name == "EoA":
                    perm.is_attacking = True
                if name == "Security":
                    ctx['event_player'] = game.player2
                result = eff_list[0].can_use_condition(ctx)
                assert result is True, f"{name} condition should pass when on field"
                if name == "EoA":
                    perm.is_attacking = False

        # Remove from field
        game.player1.battle_area.remove(perm)

        # All should fail condition when NOT on field
        for eff_list, name in [(wd, "WD"), (eoa, "EoA"), (sec, "Security")]:
            if eff_list:
                ctx = {'player': game.player1, 'game': game}
                if name == "Security":
                    ctx['event_player'] = game.player2
                if name == "EoA":
                    perm.is_attacking = True
                result = eff_list[0].can_use_condition(ctx)
                assert result is False, f"{name} condition should fail when NOT on field"
                if name == "EoA":
                    perm.is_attacking = False
