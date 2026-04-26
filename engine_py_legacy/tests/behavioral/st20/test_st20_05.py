"""Behavioral tests for ST20-05 Gatomon (Lv.4 Yellow Digimon, Holy Beast/ADVENTURE, DP 4000, Cost 4).

Card text:
[Security] At the end of the battle, play this card without paying the cost.
[On Play] [When Digivolving] Give 2 of your opponent's Digimon <Security A. -1>
    (This Digimon checks 1 fewer security card.) until their turn ends.
Inherited Effect:
[When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -2000 DP
    for the turn.

C# reference: DCGO/Assets/Scripts/CardEffect/ST20/Yellow/ST20_05.cs
- Alt-digi: Lv.3 w/[ADVENTURE] trait cost 2 (AddSelfDigivolutionRequirementStaticEffect)
- Security: PlaySelfDigimonAfterBattleSecurityEffect
- On Play / When Digivolving: SelectPermanent (max 2) +
    CardEffectCommons.ChangeDigimonSAttack(-1, UntilOpponentTurnEnd)
- Inherited When Attacking: OnAllyAttack in C# but maps to OnUseAttack in Python,
    SelectPermanent (max 1) + ChangeDigimonDP(-2000, UntilEachTurnEnd),
    OPT (maxCount=1), hash "DP-2000_ST20-005".
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase
from engine_py_legacy.engine.interfaces.modifiers import ModifierType


# Minimal deck for DebugRunner
DECK = ["ST20-05"] * 4 + ["ST1-03"] * 46


# ──────────────────────────────────────────────────────────────────────
# Static effect structure tests
# ──────────────────────────────────────────────────────────────────────


@pytest.mark.behavioral
class TestST2005StaticStructure:
    """Verify the ST20-05 card script exposes the required effects/flags."""

    def test_alt_digi_adventure_lv3_cost2(self, debug_runner):
        """Alt-digi: Lv.3 w/[ADVENTURE] trait for cost 2 (color-agnostic)."""
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, "ST20-05", "hand")
        hand_card = runner.game.player1.hand_cards[-1]
        effects = hand_card.effect_list(EffectTiming.NoTiming)

        alt_effects = [
            e for e in effects
            if getattr(e, "_alt_digi_cost", None) is not None
        ]
        assert len(alt_effects) >= 1, (
            "ST20-05 should have an alt-digi effect (Lv.3 w/ADVENTURE cost 2)"
        )
        adventure_alt = [
            e for e in alt_effects
            if getattr(e, "_alt_digi_trait", None) == "ADVENTURE"
        ]
        assert len(adventure_alt) >= 1, (
            "ST20-05 alt-digi must require ADVENTURE trait"
        )
        eff = adventure_alt[0]
        assert eff._alt_digi_level == 3, (
            f"Alt-digi level should be 3, got {eff._alt_digi_level}"
        )
        assert eff._alt_digi_cost == 2, (
            f"Alt-digi cost should be 2, got {eff._alt_digi_cost}"
        )
        # Color-agnostic (C# PermanentCondition has no color check)
        assert getattr(eff, "_alt_digi_color", None) is None, (
            "Alt-digi for ADVENTURE should be color-agnostic"
        )

    def test_security_effect_marked(self, debug_runner):
        """[Security] effect should exist with SecuritySkill timing + is_security_effect."""
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, "ST20-05", "hand")
        hand_card = runner.game.player1.hand_cards[-1]
        effects = hand_card.effect_list(EffectTiming.NoTiming)

        sec_effects = [e for e in effects
                       if e.timing == EffectTiming.SecuritySkill
                       and getattr(e, "is_security_effect", False)]
        assert len(sec_effects) >= 1, (
            "ST20-05 should have a SecuritySkill effect flagged as security"
        )

    def test_on_play_effect_exists(self, debug_runner):
        """[On Play] effect with is_on_play flag should exist (non-inherited)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST20-05"])
        top = perm.top_card
        effects = top.effect_list(EffectTiming.NoTiming)

        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, "is_on_play", False)
            and not e.is_inherited_effect
        ]
        assert len(on_play) >= 1, (
            "ST20-05 should have a main [On Play] effect with is_on_play=True"
        )

    def test_when_digivolving_effect_exists(self, debug_runner):
        """[When Digivolving] effect with is_when_digivolving flag should exist."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST20-05"])
        top = perm.top_card
        effects = top.effect_list(EffectTiming.NoTiming)

        wd_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, "is_when_digivolving", False)
            and not e.is_inherited_effect
        ]
        assert len(wd_effects) >= 1, (
            "ST20-05 should have a [When Digivolving] effect"
        )

    def test_on_play_and_when_digivolving_are_separate_effects(self, debug_runner):
        """[On Play] and [When Digivolving] must be two separate ICardEffect instances."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST20-05"])
        top = perm.top_card
        effects = top.effect_list(EffectTiming.NoTiming)

        on_play = [e for e in effects if getattr(e, "is_on_play", False)]
        wd = [e for e in effects if getattr(e, "is_when_digivolving", False)]
        assert len(on_play) >= 1 and len(wd) >= 1, (
            "Both [On Play] and [When Digivolving] effects must exist"
        )
        # They must be DIFFERENT ICardEffect instances
        assert all(op is not wde for op in on_play for wde in wd), (
            "[On Play] and [When Digivolving] must be separate effect instances"
        )

    def test_inherited_when_attacking_effect_exists(self, debug_runner):
        """[When Attacking] inherited effect with OnUseAttack timing, OPT, hash."""
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, "ST20-05", "hand")
        hand_card = runner.game.player1.hand_cards[-1]
        effects = hand_card.effect_list(EffectTiming.NoTiming)

        inherited_wa = [
            e for e in effects
            if e.is_inherited_effect
            and e.timing == EffectTiming.OnUseAttack
        ]
        assert len(inherited_wa) >= 1, (
            "ST20-05 should have an inherited [When Attacking] effect "
            "with OnUseAttack timing (not OnTappedAnyone or OnAllyAttack)"
        )
        eff = inherited_wa[0]
        assert eff.max_count_per_turn == 1, (
            "Inherited [When Attacking] should be OPT (max_count_per_turn=1)"
        )
        assert getattr(eff, "hash_string", "") != "", (
            "Inherited [When Attacking] OPT must have a non-empty hash string"
        )
        assert getattr(eff, "is_on_attack", False), (
            "Inherited [When Attacking] should set is_on_attack=True so "
            "it only fires when THIS Digimon is the attacker"
        )


# ──────────────────────────────────────────────────────────────────────
# [Security] Play this card from security
# ──────────────────────────────────────────────────────────────────────


@pytest.mark.behavioral
class TestST2005Security:
    """Tests for [Security] At the end of the battle, play this card without paying the cost."""

    def test_security_plays_self_to_field(self, debug_runner):
        """When revealed from security, ST20-05 should enter the field."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game
        p2 = game.player2

        runner.clear_zone(2, "security")
        runner.inject_card(2, "ST20-05", "security_top")

        # Weak attacker so ST20-05 (4000 DP) wins the battle and p2 doesn't lose the game
        from engine_py_legacy.tests.helpers.game_builder import make_permanent
        weak_attacker = make_permanent("WEAK-001", "WeakAttacker", dp=3000)
        game.player1.battle_area.append(weak_attacker)

        field_before = len(p2.battle_area)
        p2.security_attack(weak_attacker)
        runner.auto_resolve()

        # ST20-05 should now be on P2's field (played from security)
        field_after = len(p2.battle_area)
        assert field_after >= field_before + 1, (
            f"ST20-05 should be on the field after security, "
            f"went from {field_before} to {field_after}"
        )
        gatomon_on_field = [
            p for p in p2.battle_area
            if p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "ST20-05"
        ]
        assert len(gatomon_on_field) >= 1, (
            "ST20-05 should be a permanent on the field after security"
        )

    def test_security_not_trashed_when_played(self, debug_runner):
        """ST20-05 should NOT end up in the trash when played from security."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game
        p2 = game.player2

        runner.clear_zone(2, "security")
        runner.inject_card(2, "ST20-05", "security_top")

        from engine_py_legacy.tests.helpers.game_builder import make_permanent
        weak_attacker = make_permanent("WEAK-001", "WeakAttacker", dp=3000)
        game.player1.battle_area.append(weak_attacker)

        p2.security_attack(weak_attacker)
        runner.auto_resolve()

        in_trash = [c for c in p2.trash_cards
                    if c.c_entity_base and c.c_entity_base.card_id == "ST20-05"]
        assert len(in_trash) == 0, (
            "ST20-05 must not be in trash when played from security"
        )


# ──────────────────────────────────────────────────────────────────────
# [On Play] / [When Digivolving] — Security Attack -1 to 2 opponent Digimon
# ──────────────────────────────────────────────────────────────────────


@pytest.mark.behavioral
class TestST2005SecurityAttackMinus:
    """Tests for [On Play] / [When Digivolving] Give 2 opponent Digimon <Security A. -1>."""

    def _get_on_play_effect(self, perm):
        effects = perm.top_card.effect_list(EffectTiming.NoTiming)
        return next(
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, "is_on_play", False)
            and not e.is_inherited_effect
        )

    def test_on_play_applies_sa_minus_1_to_two_opponent_digimon(self, debug_runner):
        """[On Play] should reduce two opponent Digimon's security attack by 1."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        gatomon = runner.place_on_field(1, ["ST20-05"])

        opp1 = runner.place_on_field(2, ["ST1-03"])  # cost 3 Agumon
        opp2 = runner.place_on_field(2, ["ST1-03"])

        sa_before_1 = opp1.security_attack_modifier()
        sa_before_2 = opp2.security_attack_modifier()

        effect = self._get_on_play_effect(gatomon)
        effect.on_process_callback({
            "game": game,
            "player": game.player1,
            "permanent": gatomon,
        })

        # Two sequential selections — auto-resolve picks first legal option each time
        runner.auto_resolve()

        sa_after_1 = opp1.security_attack_modifier()
        sa_after_2 = opp2.security_attack_modifier()

        # Both opponent Digimon should have SA reduced by 1
        assert sa_after_1 == sa_before_1 - 1, (
            f"opp1 SA should drop by 1, went from {sa_before_1} to {sa_after_1}"
        )
        assert sa_after_2 == sa_before_2 - 1, (
            f"opp2 SA should drop by 1, went from {sa_before_2} to {sa_after_2}"
        )

    def test_on_play_applies_to_only_one_when_only_one_available(self, debug_runner):
        """With only 1 opponent Digimon, SA-1 should apply to that single target."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        gatomon = runner.place_on_field(1, ["ST20-05"])

        opp1 = runner.place_on_field(2, ["ST1-03"])

        sa_before = opp1.security_attack_modifier()

        effect = self._get_on_play_effect(gatomon)
        effect.on_process_callback({
            "game": game,
            "player": game.player1,
            "permanent": gatomon,
        })
        runner.auto_resolve()

        assert opp1.security_attack_modifier() == sa_before - 1, (
            "Sole opponent Digimon should receive SA-1 when only 1 target exists"
        )

    def test_on_play_no_op_when_no_opponent_digimon(self, debug_runner):
        """Running [On Play] with no opponent Digimon should not crash or pend."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        gatomon = runner.place_on_field(1, ["ST20-05"])
        # No opponent permanents

        effect = self._get_on_play_effect(gatomon)
        # Should not raise; should not leave a pending selection
        effect.on_process_callback({
            "game": game,
            "player": game.player1,
            "permanent": gatomon,
        })
        assert game.pending_selection is None, (
            "Should not request selection with no valid targets"
        )

    def test_sa_modifier_does_not_leak_to_own_digimon(self, debug_runner):
        """SA-1 modifier must not leak to the controller's own Digimon."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        gatomon = runner.place_on_field(1, ["ST20-05"])

        own_other = runner.place_on_field(1, ["ST1-03"])
        opp1 = runner.place_on_field(2, ["ST1-03"])
        opp2 = runner.place_on_field(2, ["ST1-03"])

        own_sa_before = own_other.security_attack_modifier()
        gatomon_sa_before = gatomon.security_attack_modifier()

        effect = self._get_on_play_effect(gatomon)
        effect.on_process_callback({
            "game": game,
            "player": game.player1,
            "permanent": gatomon,
        })
        runner.auto_resolve()

        assert own_other.security_attack_modifier() == own_sa_before, (
            "Own ally Digimon's SA should be unaffected by ST20-05 [On Play]"
        )
        assert gatomon.security_attack_modifier() == gatomon_sa_before, (
            "ST20-05 itself should not be affected by its own SA-1 effect"
        )

    def test_sa_modifier_registered_via_CHANGE_SECURITY_ATTACK(self, debug_runner):
        """The SA-1 effect should use ModifierType.CHANGE_SECURITY_ATTACK modifiers."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        gatomon = runner.place_on_field(1, ["ST20-05"])
        opp1 = runner.place_on_field(2, ["ST1-03"])
        opp2 = runner.place_on_field(2, ["ST1-03"])

        mods_before = game.modifiers._modifiers.get(
            ModifierType.CHANGE_SECURITY_ATTACK, [])
        count_before = len(mods_before)

        effect = self._get_on_play_effect(gatomon)
        effect.on_process_callback({
            "game": game,
            "player": game.player1,
            "permanent": gatomon,
        })
        runner.auto_resolve()

        mods_after = game.modifiers._modifiers.get(
            ModifierType.CHANGE_SECURITY_ATTACK, [])
        new_mods = len(mods_after) - count_before
        assert new_mods >= 2, (
            f"Should register 2 CHANGE_SECURITY_ATTACK modifiers, "
            f"got {new_mods} new entries"
        )
        # All new modifiers should use end_of_opponent_turn expiry
        for m in mods_after[count_before:]:
            assert m.expiry == "end_of_opponent_turn", (
                f"SA modifier expiry should be 'end_of_opponent_turn', "
                f"got {m.expiry}"
            )


# ──────────────────────────────────────────────────────────────────────
# Inherited [When Attacking] [OPT] -2000 DP
# ──────────────────────────────────────────────────────────────────────


@pytest.mark.behavioral
class TestST2005InheritedDPMinus:
    """Tests for inherited [When Attacking] [Once Per Turn] -2000 DP."""

    def _get_inherited_wa_effect(self, source_card):
        effects = source_card.effect_list(EffectTiming.NoTiming)
        return next(
            e for e in effects
            if e.is_inherited_effect
            and e.timing == EffectTiming.OnUseAttack
        )

    def test_inherited_wa_applies_dp_minus_2000(self, debug_runner):
        """Triggering the inherited WA effect should reduce a selected opponent Digimon's DP by 2000."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place ST20-05 as digivolution source under a Lv.5 Digimon
        # ST1-08 MetalGreymon is Lv.5 (Blue, 6000 DP)
        perm = runner.place_on_field(1, ["ST20-05", "ST1-08"])
        source_card = perm.card_sources[0]  # ST20-05

        opp = runner.place_on_field(2, ["ST1-07"])  # Lv.4 Greymon, 4000 DP
        dp_before = opp.dp

        effect = self._get_inherited_wa_effect(source_card)
        # Host permanent (attacker) must equal the scanned perm
        effect.on_process_callback({
            "game": game,
            "player": game.player1,
            "permanent": perm,
            "attacker": perm,
        })
        runner.auto_resolve()

        dp_after = opp.dp
        assert dp_after == dp_before - 2000, (
            f"Opponent DP should drop by 2000, went from {dp_before} to {dp_after}"
        )

    def test_inherited_wa_dp_modifier_registered_with_end_of_turn(self, debug_runner):
        """The -2000 DP modifier should use 'end_of_turn' expiry."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        perm = runner.place_on_field(1, ["ST20-05", "ST1-08"])
        source_card = perm.card_sources[0]
        opp = runner.place_on_field(2, ["ST1-07"])

        mods_before = game.modifiers._modifiers.get(ModifierType.CHANGE_DP, [])
        count_before = len(mods_before)

        effect = self._get_inherited_wa_effect(source_card)
        effect.on_process_callback({
            "game": game,
            "player": game.player1,
            "permanent": perm,
            "attacker": perm,
        })
        runner.auto_resolve()

        mods_after = game.modifiers._modifiers.get(ModifierType.CHANGE_DP, [])
        new_mods = mods_after[count_before:]
        assert len(new_mods) >= 1, (
            "Should register at least 1 CHANGE_DP modifier"
        )
        # Find our modifier — "for the turn" = end_of_turn
        for m in new_mods:
            assert m.expiry == "end_of_turn", (
                f"DP modifier expiry should be 'end_of_turn' (for the turn), "
                f"got {m.expiry}"
            )

    def test_inherited_wa_dp_modifier_has_target_equality_condition(self, debug_runner):
        """The -2000 DP modifier must not leak to other opponent Digimon."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        perm = runner.place_on_field(1, ["ST20-05", "ST1-08"])
        source_card = perm.card_sources[0]

        opp1 = runner.place_on_field(2, ["ST1-07"])  # will be selected
        opp2 = runner.place_on_field(2, ["ST1-07"])  # should be unaffected

        opp1_dp_before = opp1.dp
        opp2_dp_before = opp2.dp

        effect = self._get_inherited_wa_effect(source_card)
        effect.on_process_callback({
            "game": game,
            "player": game.player1,
            "permanent": perm,
            "attacker": perm,
        })
        runner.auto_resolve()

        # Exactly one of them should have lost 2000 DP
        drops = [
            opp1_dp_before - opp1.dp,
            opp2_dp_before - opp2.dp,
        ]
        assert drops.count(2000) == 1 and drops.count(0) == 1, (
            f"Exactly one opponent Digimon should lose 2000 DP, other unaffected. "
            f"Drops: {drops}"
        )

    def test_inherited_wa_opt_limits_to_once_per_turn(self, debug_runner):
        """The inherited [When Attacking] must be capped at once per turn."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        perm = runner.place_on_field(1, ["ST20-05", "ST1-08"])
        source_card = perm.card_sources[0]
        opp1 = runner.place_on_field(2, ["ST1-07"])

        effect = self._get_inherited_wa_effect(source_card)
        # Verify OPT structure
        assert effect.max_count_per_turn == 1, (
            "Inherited WA effect should be OPT (max_count_per_turn=1)"
        )
        # First activation should be allowed
        assert effect.can_activate_this_turn()
        effect.record_activation()
        # Second activation should be blocked
        assert not effect.can_activate_this_turn(), (
            "Inherited WA effect should be limited to once per turn"
        )

    def test_inherited_wa_condition_requires_this_is_attacker(self, debug_runner):
        """Condition should gate so effect only fires when THIS host is the attacker."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        perm = runner.place_on_field(1, ["ST20-05", "ST1-08"])
        source_card = perm.card_sources[0]
        other_perm = runner.place_on_field(1, ["ST1-03"])

        effect = self._get_inherited_wa_effect(source_card)

        # With this perm as attacker — condition should PASS (if opponent has targets)
        opp = runner.place_on_field(2, ["ST1-07"])
        ctx_ok = {
            "game": game,
            "player": game.player1,
            "permanent": perm,
            "attacker": perm,
        }
        assert effect.can_use_condition(ctx_ok), (
            "Condition should pass when THIS host is attacking"
        )

        # With DIFFERENT perm as attacker — condition should FAIL
        ctx_other = {
            "game": game,
            "player": game.player1,
            "permanent": perm,
            "attacker": other_perm,
        }
        assert not effect.can_use_condition(ctx_other), (
            "Condition should fail when a different permanent is attacking"
        )


# ──────────────────────────────────────────────────────────────────────
# Anti-leak: BeforePayCost / field-presence sanity
# ──────────────────────────────────────────────────────────────────────


@pytest.mark.behavioral
class TestST2005AntiLeak:
    """Guard tests for field-presence and no cost-reduction leak."""

    def test_on_play_condition_fails_when_not_on_field(self, debug_runner):
        """[On Play] condition should fail if the card isn't on the field yet."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        runner.inject_card(1, "ST20-05", "hand")
        hand_card = game.player1.hand_cards[-1]
        effects = hand_card.effect_list(EffectTiming.NoTiming)

        on_play = next(
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, "is_on_play", False)
            and not e.is_inherited_effect
        )
        # While still in hand, permanent_of_this_card() is None → condition should fail
        result = on_play.can_use_condition({
            "game": game,
            "player": game.player1,
        })
        assert not result, (
            "[On Play] condition should fail while ST20-05 is in hand"
        )

    def test_no_stray_beforepaycost_effect(self, debug_runner):
        """ST20-05 should not register a BeforePayCost cost-reduction effect."""
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, "ST20-05", "hand")
        hand_card = runner.game.player1.hand_cards[-1]
        effects = hand_card.effect_list(EffectTiming.NoTiming)

        bpc = [e for e in effects if e.timing == EffectTiming.BeforePayCost]
        assert not bpc, (
            "ST20-05 should not have any BeforePayCost effects"
        )
