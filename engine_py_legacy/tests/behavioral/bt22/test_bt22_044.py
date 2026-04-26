"""Behavioral tests for BT22-044 Palmon (Lv.3 Green/Red, cost 3, DP 1000).

Traits: Vegetation, CS.

Card text:
  [Your Turn] [Once Per Turn] When effects place Digimon cards with the [CS]
    trait in this Digimon's digivolution cards, gain 1 memory.
  Inherited Effect:
    [Main] [Once Per Turn] By placing this [CS] trait Digimon's top stacked
    card as its bottom digivolution card, <Draw 1> (Draw 1 card from your deck.)

  Alt digivolve: Lv.2 for cost 0.

Clause decomposition:
  C1 (body):      [Your Turn][OPT] OnAddDigivolutionCards w/ [CS] Digimon add
                  -> gain 1 memory.
  C2 (inherited): [Main][OPT] Pay cost (place own top stacked card as bottom
                  digi-source) -> Draw 1. Source must have [CS] trait on top.
"""

from __future__ import annotations

import pytest

from engine_py_legacy.engine.data.enums import EffectTiming


# ── Card IDs used by tests ───────────────────────────────────────────────

# BT22-044 Palmon: Lv.3 Green/Red, [Vegetation, CS]
PALMON = "BT22-044"
# BT22-004 Wanyamon: Lv.2 Green digi-egg, [Lesser, CS] — matches alt-digi
WANYAMON = "BT22-004"
# BT22-008 Agumon: Lv.3 Red, [Reptile, CS]
CS_DIGIMON_LV3 = "BT22-008"
# BT22-013 WarGreymon: Lv.6 Red, [Dragonkin, CS]
CS_DIGIMON_LV6 = "BT22-013"
# ST1-03 Agumon: Lv.3 Red, [Reptile] — NO CS trait
NO_CS_DIGIMON_LV3 = "ST1-03"
# ST1-04 Dracomon: filler used to avoid name collisions
FILLER = "ST1-04"


def _get_all_source_effects(perm):
    """Return every ICardEffect from every card source on the permanent,
    bypassing inherited filtering (useful for shape checks)."""
    effects = []
    for source in perm.card_sources:
        effects.extend(source.effect_list(EffectTiming.NoTiming))
    return effects


def _find_bt22_044_memory_effect(perm):
    """Find the C1 OnAddDigivolutionCards +1 memory effect on the permanent.

    Looks across all sources because C1 is a body effect (non-inherited) and
    must come from Palmon when Palmon is the top of its own permanent.
    """
    for eff in _get_all_source_effects(perm):
        if 'BT22-044' not in (getattr(eff, 'effect_name', '') or ''):
            continue
        if eff.timing != EffectTiming.OnAddDigivolutionCards:
            continue
        return eff
    return None


def _find_bt22_044_inherited_field_main(perm):
    """Find the C2 [Main] inherited field_main effect currently active on the
    permanent. Uses perm.effect_list which correctly applies inherited-filter
    rules (top card's inherited effects are NOT active)."""
    for eff in perm.effect_list(EffectTiming.NoTiming):
        if 'BT22-044' not in (getattr(eff, 'effect_name', '') or ''):
            continue
        if not getattr(eff, '_is_field_main', False):
            continue
        if not getattr(eff, 'is_inherited_effect', False):
            continue
        return eff
    return None


# ── Shape / registration tests ───────────────────────────────────────────


@pytest.mark.behavioral
class TestBT22044Shape:
    """Static checks that the script declares the expected effect shape."""

    def test_alt_digi_lv2_cost_0(self, debug_runner):
        """BT22-044 should digivolve onto a Lv.2 Digimon for cost 0 (alt-digi)."""
        runner = debug_runner(
            deck1=[FILLER] * 50, deck2=[FILLER] * 50, initial_memory=5,
        )
        runner.place_on_field(1, [WANYAMON])  # Lv.2 Green
        runner.inject_card(1, PALMON, "hand")

        actions = runner.find_actions("Digivolve")
        palmon_digi = [a for a, d in actions.items() if "Palmon" in d]
        assert palmon_digi, (
            "BT22-044 should be able to digivolve onto BT22-004 Wanyamon "
            "(alt-digi Lv.2 for cost 0)."
        )

    def test_c1_memory_effect_is_opt_your_turn(self, debug_runner):
        """C1 must be OnAddDigivolutionCards, OPT, with a unique hash.

        C1 is marked is_inherited_effect=True so that it is found in
        perm.effect_list() once Palmon becomes a digivolution source
        (which happens the moment a card is appended to its stack via
        `add_card_source`). This mirrors the BT22-001 / BT22-004 / BT22-006
        pattern for OnAddDigivolutionCards body effects on low-level CS
        Digimon.
        """
        runner = debug_runner(
            deck1=[FILLER] * 50, deck2=[FILLER] * 50, initial_memory=10,
        )
        palmon_perm = runner.place_on_field(1, [PALMON])
        eff = _find_bt22_044_memory_effect(palmon_perm)
        assert eff is not None, "BT22-044 C1 memory effect must exist"
        assert eff.timing == EffectTiming.OnAddDigivolutionCards
        assert eff.max_count_per_turn == 1, "C1 must be Once Per Turn"
        assert eff.hash_string, "C1 must set a hash_string for OPT tracking"
        assert eff.is_inherited_effect, (
            "C1 must be inherited so it is scanned while Palmon is a "
            "digivolution source (mirrors BT22-001/004/006 pattern)"
        )

    def test_c2_inherited_field_main_opt(self, debug_runner):
        """C2 must be inherited, _is_field_main, OPT, with a unique hash."""
        runner = debug_runner(
            deck1=[FILLER] * 50, deck2=[FILLER] * 50, initial_memory=10,
        )
        # Palmon as digi-source beneath a CS Digimon on field
        perm = runner.place_on_field(1, [PALMON, CS_DIGIMON_LV6])
        eff = _find_bt22_044_inherited_field_main(perm)
        assert eff is not None, (
            "BT22-044 C2 inherited field_main effect must exist when Palmon "
            "is a digivolution source beneath another Digimon"
        )
        assert eff.is_inherited_effect, "C2 must be inherited"
        assert getattr(eff, '_is_field_main', False), (
            "C2 must be _is_field_main for [Main] activation from field"
        )
        assert eff.max_count_per_turn == 1, "C2 must be Once Per Turn"
        assert eff.hash_string, "C2 must set a hash_string for OPT tracking"

    def test_c1_and_c2_are_separate_instances(self, debug_runner):
        """C1 and C2 must be SEPARATE ICardEffect instances."""
        runner = debug_runner(
            deck1=[FILLER] * 50, deck2=[FILLER] * 50, initial_memory=10,
        )
        # Place Palmon as both top and source to pick up both effects lists
        perm_top = runner.place_on_field(1, [PALMON])
        c1 = _find_bt22_044_memory_effect(perm_top)
        assert c1 is not None

        # Under another Digimon to surface the inherited C2
        perm_inh = runner.place_on_field(1, [PALMON, CS_DIGIMON_LV6])
        c2 = _find_bt22_044_inherited_field_main(perm_inh)
        assert c2 is not None

        assert c1 is not c2, "C1 and C2 must be distinct effect instances"
        assert c1.hash_string != c2.hash_string, (
            "C1 and C2 must have distinct hash strings"
        )


# ── C1 — [Your Turn] OPT gain 1 memory on CS Digimon add ─────────────────


@pytest.mark.behavioral
class TestBT22044C1MemoryGain:
    """[Your Turn][OPT] When effects place [CS] trait Digimon cards in this
    Digimon's digivolution cards, gain 1 memory."""

    def test_gain_memory_when_cs_digimon_added_to_this_perm(self, debug_runner):
        """Adding a CS Digimon card to Palmon's digi-stack grants +1 memory."""
        runner = debug_runner(
            deck1=[FILLER] * 50, deck2=[FILLER] * 50, initial_memory=5,
        )
        game = runner.game
        palmon_perm = runner.place_on_field(1, [PALMON])

        # Build a fresh CS Digimon CardSource and add it to Palmon's stack
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs_digi = db.create_card_source(CS_DIGIMON_LV3, game.player1)

        mem_before = game.memory
        palmon_perm.add_card_source(cs_digi)

        assert game.memory == mem_before + 1, (
            f"Adding a [CS] Digimon to Palmon's digi-stack should grant +1 "
            f"memory. Before: {mem_before}, after: {game.memory}"
        )

    def test_no_gain_when_non_cs_digimon_added(self, debug_runner):
        """Adding a non-CS Digimon card should NOT grant memory."""
        runner = debug_runner(
            deck1=[FILLER] * 50, deck2=[FILLER] * 50, initial_memory=5,
        )
        game = runner.game
        palmon_perm = runner.place_on_field(1, [PALMON])

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        non_cs = db.create_card_source(NO_CS_DIGIMON_LV3, game.player1)

        mem_before = game.memory
        palmon_perm.add_card_source(non_cs)

        assert game.memory == mem_before, (
            f"Non-CS Digimon add should NOT trigger memory gain. "
            f"Before: {mem_before}, after: {game.memory}"
        )

    def test_no_gain_when_added_to_other_perm(self, debug_runner):
        """Adding a CS Digimon to a DIFFERENT permanent should NOT trigger."""
        runner = debug_runner(
            deck1=[FILLER] * 50, deck2=[FILLER] * 50, initial_memory=5,
        )
        game = runner.game
        # Palmon on its own permanent
        runner.place_on_field(1, [PALMON])
        # A separate, unrelated Digimon
        other_perm = runner.place_on_field(1, [NO_CS_DIGIMON_LV3])

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs_digi = db.create_card_source(CS_DIGIMON_LV3, game.player1)

        mem_before = game.memory
        # Add CS Digimon to the OTHER permanent, not Palmon's
        other_perm.add_card_source(cs_digi)

        assert game.memory == mem_before, (
            f"Adding a [CS] Digimon to a different permanent must not "
            f"trigger Palmon's +1 memory. Before: {mem_before}, "
            f"after: {game.memory}"
        )

    def test_once_per_turn(self, debug_runner):
        """C1 should fire only once per turn, even if two CS Digimon adds."""
        runner = debug_runner(
            deck1=[FILLER] * 50, deck2=[FILLER] * 50, initial_memory=5,
        )
        game = runner.game
        palmon_perm = runner.place_on_field(1, [PALMON])

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()

        mem_before = game.memory
        cs1 = db.create_card_source(CS_DIGIMON_LV3, game.player1)
        palmon_perm.add_card_source(cs1)
        cs2 = db.create_card_source(CS_DIGIMON_LV3, game.player1)
        palmon_perm.add_card_source(cs2)

        # Only one +1 memory gain from the first add; second is blocked by OPT
        assert game.memory == mem_before + 1, (
            f"C1 must be Once Per Turn. Expected +1 memory total, got "
            f"{game.memory - mem_before}"
        )

    def test_only_on_your_turn(self, debug_runner):
        """C1 should not fire on the opponent's turn."""
        runner = debug_runner(
            deck1=[FILLER] * 50, deck2=[FILLER] * 50, initial_memory=5,
        )
        game = runner.game
        palmon_perm = runner.place_on_field(1, [PALMON])

        # Flip control to player2 so it's NOT player1's turn
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs_digi = db.create_card_source(CS_DIGIMON_LV3, game.player1)

        mem_before = game.memory
        palmon_perm.add_card_source(cs_digi)

        assert game.memory == mem_before, (
            "C1 is [Your Turn] — must not fire on opponent's turn"
        )


# ── C2 — Inherited [Main][OPT] rearrange, Draw 1 ─────────────────────────


@pytest.mark.behavioral
class TestBT22044C2Inherited:
    """Inherited: [Main][OPT] By placing this [CS] trait Digimon's top stacked
    card as its bottom digivolution card, <Draw 1>."""

    def test_condition_requires_top_card_cs_trait(self, debug_runner):
        """Condition should fail if the top card has no CS trait."""
        runner = debug_runner(
            deck1=[FILLER] * 50, deck2=[FILLER] * 50, initial_memory=10,
        )
        game = runner.game
        # Palmon under a NON-CS Digimon
        perm = runner.place_on_field(1, [PALMON, NO_CS_DIGIMON_LV3])

        eff = _find_bt22_044_inherited_field_main(perm)
        assert eff is not None
        eff.set_effect_source_permanent(perm)
        ctx = {'game': game, 'player': game.player1, 'permanent': perm}
        assert eff.can_use_condition(ctx) is False, (
            "C2 should not activate when the top card lacks [CS] trait"
        )

    def test_condition_requires_at_least_one_source(self, debug_runner):
        """Condition should fail if no digivolution sources are present."""
        runner = debug_runner(
            deck1=[FILLER] * 50, deck2=[FILLER] * 50, initial_memory=10,
        )
        game = runner.game
        # Palmon all alone on field — no digi-sources beneath it
        perm = runner.place_on_field(1, [PALMON])

        # Palmon is the top here, so its inherited effects are NOT in its
        # own effect_list. There should be no usable inherited field_main.
        eff = _find_bt22_044_inherited_field_main(perm)
        assert eff is None, (
            "When Palmon is top (no sources beneath), its inherited C2 "
            "should not appear in perm.effect_list()"
        )

    def test_condition_passes_when_cs_top_with_source(self, debug_runner):
        """Condition should pass when top is [CS] and there's >= 1 source."""
        runner = debug_runner(
            deck1=[FILLER] * 50, deck2=[FILLER] * 50, initial_memory=10,
        )
        game = runner.game
        perm = runner.place_on_field(1, [PALMON, CS_DIGIMON_LV6])

        eff = _find_bt22_044_inherited_field_main(perm)
        assert eff is not None
        eff.set_effect_source_permanent(perm)
        ctx = {'game': game, 'player': game.player1, 'permanent': perm}
        assert eff.can_use_condition(ctx) is True, (
            "C2 condition should pass with [CS] top card and >= 1 source"
        )

    def test_process_moves_top_card_to_bottom_and_draws(self, debug_runner):
        """Process moves the top card to the bottom of the digi-stack and draws 1."""
        runner = debug_runner(
            deck1=[FILLER] * 50, deck2=[FILLER] * 50, initial_memory=10,
        )
        game = runner.game
        perm = runner.place_on_field(1, [PALMON, CS_DIGIMON_LV6])

        # Snapshot stack BEFORE: [PALMON, CS_DIGIMON_LV6] (bottom -> top)
        before_ids = [
            cs.c_entity_base.card_id if cs.c_entity_base else None
            for cs in perm.card_sources
        ]
        assert before_ids == [PALMON, CS_DIGIMON_LV6]
        assert perm.top_card.c_entity_base.card_id == CS_DIGIMON_LV6

        eff = _find_bt22_044_inherited_field_main(perm)
        assert eff is not None
        eff.set_effect_source_permanent(perm)

        hand_before = len(game.player1.hand_cards)
        lib_before = len(game.player1.library_cards)

        ctx = {'game': game, 'player': game.player1, 'permanent': perm}
        eff.on_process_callback(ctx)
        runner.auto_resolve(max_steps=5)

        # Stack AFTER: [CS_DIGIMON_LV6, PALMON] — old top moved to bottom
        after_ids = [
            cs.c_entity_base.card_id if cs.c_entity_base else None
            for cs in perm.card_sources
        ]
        assert after_ids == [CS_DIGIMON_LV6, PALMON], (
            f"Top card should move to the bottom. Before: {before_ids}, "
            f"after: {after_ids}"
        )
        assert perm.top_card.c_entity_base.card_id == PALMON, (
            "After rearranging, Palmon should become the new top card"
        )

        # Draw 1: hand +1, library -1
        assert len(game.player1.hand_cards) == hand_before + 1, (
            f"Expected +1 hand after Draw 1. Before: {hand_before}, "
            f"after: {len(game.player1.hand_cards)}"
        )
        assert len(game.player1.library_cards) == lib_before - 1, (
            f"Expected -1 library after Draw 1. Before: {lib_before}, "
            f"after: {len(game.player1.library_cards)}"
        )
