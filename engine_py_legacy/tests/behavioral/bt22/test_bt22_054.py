"""Behavioral tests for BT22-054 Hagurumon.

Card text:
    Digivolve: Lv.2 w/[CS] trait for cost 0
    [Your Turn] [Once Per Turn] When effects place Digimon cards with the [CS] trait
        in this Digimon's digivolution cards, 1 of your opponent's Digimon gets
        -3000 DP for the turn.
    Inherited Effect [Main] [Once Per Turn] By placing this [CS] trait Digimon's top
        stacked card as its bottom digivolution card, <Draw 1> (Draw 1 card from your
        deck.)
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming
from engine_py_legacy.engine.data.card_database import CardDatabase


# ── Helpers ──────────────────────────────────────────────────────────────


def _get_effects(perm):
    """Get all effects from all card sources on a permanent."""
    effects = []
    for source in perm.card_sources:
        effects.extend(source.effect_list(EffectTiming.NoTiming))
    return effects


def _find_effect(perm, needle):
    for eff in _get_effects(perm):
        if needle.lower() in (eff.effect_name or "").lower():
            return eff
    return None


def _find_on_add_digi_effect(perm):
    for source in perm.card_sources:
        for eff in source.effect_list(EffectTiming.OnAddDigivolutionCards):
            if eff.timing == EffectTiming.OnAddDigivolutionCards:
                return eff
    return None


def _find_field_main_effect(perm):
    for source in perm.card_sources:
        for eff in source.effect_list(EffectTiming.NoTiming):
            if getattr(eff, '_is_field_main', False):
                return eff
    return None


def _find_alt_digi_effect(card):
    """Find the alt-digivolve factory effect on a card source."""
    for eff in card.effect_list(EffectTiming.NoTiming):
        if getattr(eff, '_alt_digi_cost', None) is not None:
            return eff
    return None


# ── Tests ────────────────────────────────────────────────────────────────


@pytest.mark.behavioral
class TestBT22054Hagurumon:
    """Tests for BT22-054 Hagurumon."""

    # ── Clause 1: [Your Turn][OPT] OnAddDigivolutionCards CS -> -3000 DP ──

    def test_c1_effect_is_inherited(self, debug_runner):
        """C1 must be an inherited effect so it remains active once Hagurumon
        drops out of top_card after a card is stacked on top."""
        runner = debug_runner(
            deck1=['BT22-054']*4 + ['ST1-03']*46,
            deck2=['ST1-03']*50,
            initial_memory=10,
        )
        hag_perm = runner.place_on_field(1, ['BT22-054'])
        eff = _find_on_add_digi_effect(hag_perm)
        assert eff is not None, "C1 OnAddDigivolutionCards effect missing"
        assert getattr(eff, 'is_inherited_effect', False) is True, (
            "C1 MUST be is_inherited_effect=True (Hagurumon drops out of top_card "
            "after stacking; without inherited flag the effect is unreachable)"
        )

    def test_c1_opt_hash_and_max(self, debug_runner):
        """C1 must be OPT with max_count_per_turn=1 and a unique hash string."""
        runner = debug_runner(
            deck1=['BT22-054']*4 + ['ST1-03']*46,
            deck2=['ST1-03']*50,
            initial_memory=10,
        )
        hag_perm = runner.place_on_field(1, ['BT22-054'])
        eff = _find_on_add_digi_effect(hag_perm)
        assert eff is not None
        assert eff.max_count_per_turn == 1, "C1 must be OPT (max_count_per_turn=1)"
        assert eff.hash_string and 'BT22_054' in eff.hash_string, (
            "C1 hash_string must include BT22_054 for OPT tracking"
        )

    def test_c1_condition_only_own_turn(self, debug_runner):
        """C1 must be gated by [Your Turn] — only fires on owner's turn."""
        runner = debug_runner(
            deck1=['BT22-054']*4 + ['ST1-03']*46,
            deck2=['ST1-03']*50,
            initial_memory=10,
        )
        hag_perm = runner.place_on_field(1, ['BT22-054'])
        eff = _find_on_add_digi_effect(hag_perm)
        assert eff is not None

        db = CardDatabase()
        cs = db.create_card_source('BT22-008', runner.game.player1)  # Agumon CS Lv3

        # Not owner's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True
        ctx = {
            'game': runner.game, 'player': runner.game.player1,
            'permanent': hag_perm, 'event_permanent': hag_perm,
            'added_card': cs,
        }
        assert eff.can_use_condition(ctx) is False, (
            "C1 must not fire when it's not the owner's turn"
        )

    def test_c1_condition_scope_to_this_permanent(self, debug_runner):
        """C1 must only fire when the triggered event_permanent is Hagurumon's perm.

        Without this gate, any OnAddDigivolutionCards event on ANY permanent would
        fire Hagurumon's effect — a systemic leak.
        """
        runner = debug_runner(
            deck1=['BT22-054']*4 + ['BT22-008']*4 + ['ST1-03']*42,
            deck2=['ST1-03']*50,
            initial_memory=10,
        )
        hag_perm = runner.place_on_field(1, ['BT22-054'])
        other_perm = runner.place_on_field(1, ['BT22-008'])  # some other Digimon
        eff = _find_on_add_digi_effect(hag_perm)
        assert eff is not None

        db = CardDatabase()
        added = db.create_card_source('BT22-008', runner.game.player1)

        # Event fired on OTHER perm, not Hagurumon's
        ctx_other = {
            'game': runner.game, 'player': runner.game.player1,
            'permanent': other_perm, 'event_permanent': other_perm,
            'added_card': added,
        }
        assert eff.can_use_condition(ctx_other) is False, (
            "C1 must not fire when event_permanent is a different permanent"
        )

        # Event fired on Hagurumon's perm
        ctx_self = {
            'game': runner.game, 'player': runner.game.player1,
            'permanent': hag_perm, 'event_permanent': hag_perm,
            'added_card': added,
        }
        assert eff.can_use_condition(ctx_self) is True, (
            "C1 must fire when event_permanent is Hagurumon's permanent"
        )

    def test_c1_condition_requires_cs_digimon_added(self, debug_runner):
        """C1 must only fire when the added card is a Digimon with CS trait."""
        runner = debug_runner(
            deck1=['BT22-054']*4 + ['ST1-03']*46,
            deck2=['ST1-03']*50,
            initial_memory=10,
        )
        hag_perm = runner.place_on_field(1, ['BT22-054'])
        eff = _find_on_add_digi_effect(hag_perm)
        assert eff is not None

        db = CardDatabase()

        # Non-CS Digimon (ST1-03 Agumon, no CS trait)
        non_cs = db.create_card_source('ST1-03', runner.game.player1)
        ctx = {
            'game': runner.game, 'player': runner.game.player1,
            'permanent': hag_perm, 'event_permanent': hag_perm,
            'added_card': non_cs,
        }
        assert eff.can_use_condition(ctx) is False, (
            "C1 must not fire when added card lacks CS trait"
        )

        # CS Digimon
        cs_digi = db.create_card_source('BT22-008', runner.game.player1)
        ctx['added_card'] = cs_digi
        assert eff.can_use_condition(ctx) is True, (
            "C1 must fire when added card is a Digimon with CS trait"
        )

    def test_c1_process_reduces_opponent_dp_by_3000(self, debug_runner):
        """C1 process: selecting an opponent Digimon applies -3000 DP for the turn."""
        runner = debug_runner(
            deck1=['BT22-054']*4 + ['ST1-03']*46,
            deck2=['ST1-06']*50,  # Coredramon 6000 DP
            initial_memory=10,
        )
        hag_perm = runner.place_on_field(1, ['BT22-054'])
        opp = runner.place_on_field(2, ['ST1-06'])
        assert opp.dp == 6000

        eff = _find_on_add_digi_effect(hag_perm)
        assert eff is not None

        ctx = {
            'game': runner.game, 'player': runner.game.player1,
            'permanent': hag_perm, 'event_permanent': hag_perm,
            'added_card': None,
        }
        eff.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)

        assert opp.dp == 3000, f"Expected 6000 - 3000 = 3000, got {opp.dp}"

    def test_c1_process_selects_one_target(self, debug_runner):
        """C1 must only affect ONE opponent Digimon (not all)."""
        runner = debug_runner(
            deck1=['BT22-054']*4 + ['ST1-03']*46,
            deck2=['ST1-06']*50,
            initial_memory=10,
        )
        hag_perm = runner.place_on_field(1, ['BT22-054'])
        opp1 = runner.place_on_field(2, ['ST1-06'])  # 6000 DP
        opp2 = runner.place_on_field(2, ['ST1-06'])  # 6000 DP
        assert opp1.dp == 6000 and opp2.dp == 6000

        eff = _find_on_add_digi_effect(hag_perm)
        assert eff is not None

        ctx = {
            'game': runner.game, 'player': runner.game.player1,
            'permanent': hag_perm, 'event_permanent': hag_perm,
            'added_card': None,
        }
        eff.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)

        # Exactly one should be reduced
        dps = sorted([opp1.dp, opp2.dp])
        assert dps == [3000, 6000], (
            f"Only one opponent Digimon should receive -3000; got dps={dps}"
        )

    def test_c1_process_filters_opponent_digimon(self, debug_runner):
        """C1 selection should only target opponent Digimon (not tamers/options)."""
        runner = debug_runner(
            deck1=['BT22-054']*4 + ['ST1-03']*46,
            deck2=['ST1-06']*50,
            initial_memory=10,
        )
        hag_perm = runner.place_on_field(1, ['BT22-054'])
        opp = runner.place_on_field(2, ['ST1-06'])

        eff = _find_on_add_digi_effect(hag_perm)
        assert eff is not None

        # No crash when called; opp Digimon DP reduces
        ctx = {
            'game': runner.game, 'player': runner.game.player1,
            'permanent': hag_perm, 'event_permanent': hag_perm,
            'added_card': None,
        }
        eff.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)
        assert opp.dp == 3000

    def test_c1_end_to_end_trigger_on_digivolve(self, debug_runner):
        """End-to-end: stacking a CS Digimon onto Hagurumon triggers C1."""
        runner = debug_runner(
            deck1=['BT22-054']*4 + ['BT22-008']*4 + ['ST1-03']*42,
            deck2=['ST1-06']*50,
            initial_memory=10,
        )
        # Hagurumon base on field
        hag_perm = runner.place_on_field(1, ['BT22-054'])
        assert hag_perm.top_card.card_names == ['Hagurumon']
        # Opponent Digimon
        opp = runner.place_on_field(2, ['ST1-06'])
        assert opp.dp == 6000

        # Directly place Agumon (CS, Lv3) onto Hagurumon via add_card_source
        # This simulates a digivolution and fires OnAddDigivolutionCards
        db = CardDatabase()
        agu = db.create_card_source('BT22-008', runner.game.player1)
        hag_perm.add_card_source(agu)
        # Resolve selection
        runner.auto_resolve(max_steps=10)

        # Trigger should have fired and applied -3000
        assert opp.dp == 3000, (
            f"End-to-end: expected -3000 DP (6000 - 3000 = 3000), got {opp.dp}. "
            "C1 likely not firing — check is_inherited_effect and event_permanent scope."
        )

    def test_c1_opt_second_activation_blocked(self, debug_runner):
        """C1 OPT: second activation in same turn is blocked."""
        runner = debug_runner(
            deck1=['BT22-054']*4 + ['ST1-03']*46,
            deck2=['ST1-06']*50,
            initial_memory=10,
        )
        hag_perm = runner.place_on_field(1, ['BT22-054'])
        eff = _find_on_add_digi_effect(hag_perm)
        assert eff is not None

        assert eff.can_activate_this_turn()
        eff.record_activation()
        assert not eff.can_activate_this_turn(), (
            "C1 OPT must block second activation in the same turn"
        )

    # ── Clause 2: Inherited [Main][OPT] Place top card bottom, Draw 1 ──

    def test_c2_is_inherited_and_field_main(self, debug_runner):
        """C2 must be inherited effect AND field_main."""
        runner = debug_runner(
            deck1=['BT22-054']*4 + ['ST1-03']*46,
            deck2=['ST1-03']*50,
            initial_memory=10,
        )
        hag_perm = runner.place_on_field(1, ['BT22-054'])
        # Find the field_main effect (C2) by name
        eff = None
        for source in hag_perm.card_sources:
            for e in source.effect_list(EffectTiming.NoTiming):
                if getattr(e, '_is_field_main', False):
                    eff = e
                    break
        assert eff is not None, "C2 field_main effect not found"
        assert getattr(eff, 'is_inherited_effect', False) is True, (
            "C2 must be inherited"
        )
        assert getattr(eff, '_is_field_main', False) is True, (
            "C2 must be _is_field_main=True"
        )
        assert eff.max_count_per_turn == 1, "C2 must be OPT"

    def test_c2_condition_requires_top_is_cs_digimon(self, debug_runner):
        """C2 condition: top card must be a Digimon with the CS trait."""
        runner = debug_runner(
            deck1=['BT22-054']*4 + ['BT22-008']*4 + ['ST1-03']*42,
            deck2=['ST1-03']*50,
            initial_memory=10,
        )
        # Stack: Hagurumon (bottom) + Agumon ST1-03 (top, non-CS)
        # ST1-03 is non-CS — C2 should not fire
        perm_non_cs = runner.place_on_field(1, ['BT22-054', 'ST1-03'])
        eff = _find_field_main_effect(perm_non_cs)
        assert eff is not None

        ctx = {'game': runner.game, 'player': runner.game.player1, 'permanent': perm_non_cs}
        assert eff.can_use_condition(ctx) is False, (
            "C2 must not fire when top card lacks CS trait"
        )

        # Stack: Hagurumon (bottom) + BT22-008 (top, CS)
        perm_cs = runner.place_on_field(1, ['BT22-054', 'BT22-008'])
        eff2 = _find_field_main_effect(perm_cs)
        assert eff2 is not None
        ctx2 = {'game': runner.game, 'player': runner.game.player1, 'permanent': perm_cs}
        assert eff2.can_use_condition(ctx2) is True, (
            "C2 must fire when top card has CS trait and stack >= 2"
        )

    def test_c2_condition_requires_stack_size_at_least_2(self, debug_runner):
        """C2 condition: must have at least 2 cards in the stack
        (top + at least 1 digi-card)."""
        runner = debug_runner(
            deck1=['BT22-054']*4 + ['ST1-03']*46,
            deck2=['ST1-03']*50,
            initial_memory=10,
        )
        # Single-card permanent — Hagurumon alone, but C2 is inherited so it
        # won't be active when Hagurumon IS the top. Use a stack of 1 to verify
        # the length gate anyway.
        hag_perm = runner.place_on_field(1, ['BT22-054'])
        # Find the C2 effect (exists on the source even if inactive)
        eff = _find_field_main_effect(hag_perm)
        assert eff is not None
        assert len(hag_perm.card_sources) == 1
        ctx = {'game': runner.game, 'player': runner.game.player1, 'permanent': hag_perm}
        assert eff.can_use_condition(ctx) is False, (
            "C2 must not fire when stack has only 1 card"
        )

    def test_c2_condition_requires_is_my_turn(self, debug_runner):
        """C2 is a [Main] effect — only active on owner's turn."""
        runner = debug_runner(
            deck1=['BT22-054']*4 + ['BT22-008']*4 + ['ST1-03']*42,
            deck2=['ST1-03']*50,
            initial_memory=10,
        )
        perm = runner.place_on_field(1, ['BT22-054', 'BT22-008'])
        eff = _find_field_main_effect(perm)
        assert eff is not None

        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True
        ctx = {'game': runner.game, 'player': runner.game.player1, 'permanent': perm}
        assert eff.can_use_condition(ctx) is False, (
            "C2 must not fire when it's not the owner's turn"
        )

    def test_c2_process_moves_top_to_bottom_and_draws(self, debug_runner):
        """C2 process: top card moves to bottom of stack, draws 1."""
        runner = debug_runner(
            deck1=['BT22-054']*4 + ['BT22-008']*4 + ['ST1-03']*42,
            deck2=['ST1-03']*50,
            initial_memory=10,
        )
        perm = runner.place_on_field(1, ['BT22-054', 'BT22-008'])
        # Before: [Hagurumon, Agumon] — Agumon on top
        before = [s.card_names[0] for s in perm.card_sources]
        assert before == ['Hagurumon', 'Agumon'], f"Setup: {before}"
        hand_size_before = len(runner.game.player1.hand_cards)

        eff = _find_field_main_effect(perm)
        assert eff is not None
        ctx = {'game': runner.game, 'player': runner.game.player1, 'permanent': perm}
        eff.on_process_callback(ctx)
        runner.auto_resolve(max_steps=5)

        # After: [Agumon, Hagurumon] — Agumon (former top) now bottom, Hagurumon new top
        after = [s.card_names[0] for s in perm.card_sources]
        assert after == ['Agumon', 'Hagurumon'], (
            f"C2 should move top to bottom: expected ['Agumon', 'Hagurumon'], got {after}"
        )
        assert perm.top_card.card_names == ['Hagurumon'], (
            "New top should be Hagurumon"
        )
        # Draw 1
        assert len(runner.game.player1.hand_cards) == hand_size_before + 1, (
            f"Should have drawn 1 card (hand: {hand_size_before} -> {len(runner.game.player1.hand_cards)})"
        )

    def test_c2_opt_second_activation_blocked(self, debug_runner):
        """C2 OPT: second activation in same turn is blocked."""
        runner = debug_runner(
            deck1=['BT22-054']*4 + ['BT22-008']*4 + ['ST1-03']*42,
            deck2=['ST1-03']*50,
            initial_memory=10,
        )
        perm = runner.place_on_field(1, ['BT22-054', 'BT22-008'])
        eff = _find_field_main_effect(perm)
        assert eff is not None
        assert eff.can_activate_this_turn()
        eff.record_activation()
        assert not eff.can_activate_this_turn()

    def test_c2_condition_does_not_rely_on_effect_source_permanent(self, debug_runner):
        """C2 condition must use context['permanent'], NOT
        effect.effect_source_permanent (which is not set for field_main)."""
        runner = debug_runner(
            deck1=['BT22-054']*4 + ['BT22-008']*4 + ['ST1-03']*42,
            deck2=['ST1-03']*50,
            initial_memory=10,
        )
        perm = runner.place_on_field(1, ['BT22-054', 'BT22-008'])
        eff = _find_field_main_effect(perm)
        assert eff is not None

        # Clear effect_source_permanent to simulate fresh field_main call
        if hasattr(eff, 'effect_source_permanent'):
            eff.effect_source_permanent = None

        ctx = {'game': runner.game, 'player': runner.game.player1, 'permanent': perm}
        assert eff.can_use_condition(ctx) is True, (
            "C2 condition must use context['permanent'], not effect.effect_source_permanent"
        )

    # ── Alt-digivolve requirement (Lv.2 CS, no color) ──

    def test_alt_digi_requires_lv2_cs_no_color(self, debug_runner):
        """Alt digivolve: Lv.2 + CS trait, no color restriction (per C#)."""
        runner = debug_runner(
            deck1=['BT22-054']*4 + ['ST1-03']*46,
            deck2=['ST1-03']*50,
            initial_memory=10,
        )
        db = CardDatabase()
        card = db.create_card_source('BT22-054', runner.game.player1)
        alt = _find_alt_digi_effect(card)
        assert alt is not None, "Alt-digi effect missing"

        assert alt._alt_digi_cost == 0, "Alt-digi cost must be 0"
        assert alt._alt_digi_level == 2, "Alt-digi level must be 2"
        assert getattr(alt, '_alt_digi_trait', None) == 'CS', (
            "Alt-digi must require CS trait (not color)"
        )
        # The C# IsLevel2 && HasCSTraits check has NO color restriction
        assert getattr(alt, '_alt_digi_color', None) is None, (
            "Alt-digi must NOT have a color restriction "
            "(C# only checks IsLevel2 && HasCSTraits)"
        )
