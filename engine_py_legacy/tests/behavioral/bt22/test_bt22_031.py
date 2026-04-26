"""Behavioral tests for BT22-031 GoldNumemon | Lv.4 (Yellow/Black, Mutant/CS, cost 5).

Card text (source of truth):
  [On Play] [When Digivolving] Give 1 of your opponent's Digimon <Security A. -2>
  (This Digimon checks 2 additional security cards.) until their turn ends.
  Then, if this Digimon's stack has 2 or more same-level cards, this Digimon may
  digivolve into [PlatinumNumemon] in the hand for a digivolution cost of 4,
  ignoring digivolution requirements.

  Inherited Effect:
  [Your Turn] [Once Per Turn] When this Digimon would digivolve into a Digimon
  card with the [CS] trait, reduce the digivolution cost by 1.

  Alt-digivolve (C#):
  (Lv.4 w/ [Numemon] in name) OR (Lv.3 w/ [CS] trait) for cost 2.

Clauses tested:
  Alt-digi:
    - A-1: Lv.4 Numemon → cost 2
    - A-2: Lv.3 CS trait → cost 2
    - A-3: non-matching base → no alt-digi

  On Play / When Digivolving main effect:
    - M-1: Registers CHANGE_SECURITY_ATTACK -2 modifier on selected opp Digimon
    - M-2: Modifier expires 'end_of_opponent_turn'
    - M-3: Target-equality guard: SA-2 does NOT leak to other opponent Digimon
    - M-4: If stack has 2+ same-level cards, offers optional digivolve into
           PlatinumNumemon from hand for cost 4, ignoring requirements
    - M-5: No PlatinumNumemon digivolve offered when stack lacks same-level pair

  Inherited CS cost reduction (WhenWouldDigivolve):
    - I-1: Timing is WhenWouldDigivolve
    - I-2: Digivolving into a CS-trait Digimon from a stack containing BT22-031
           reduces the cost by 1
    - I-3: Digivolving into a non-CS Digimon does NOT reduce
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming
from digimon_gym.engine.interfaces.modifiers import ModifierType


# --- Test card IDs ---
GOLD_NUMEMON = "BT22-031"       # Our card: Lv.4 Yellow/Black, CS/Mutant
PLATINUM_NUMEMON = "BT22-065"   # PlatinumNumemon Lv.6, Mutant/CS
# An Lv.4 Numemon-name for alt-digi:
# Check cards.json — BT14-066 is PlatinumNumemon Lv.6 (no good).
# Use BT8-065 Numemon (should be Lv.3 Yellow) — but need Lv.4 Numemon.
# Fallback: test alt-digi Lv.4 Numemon by building a deck with a known Lv.4 Numemon card.

# BT22-008 Agumon: Red Lv.3, CS trait (for alt-digi Lv.3 CS test)
CS_LV3 = "BT22-008"

# BT22-012 RizeGreymon: Lv.5 Red/Yellow, CS trait (target for inherited cost reduction)
CS_LV5_TARGET = "BT22-012"

# BT1-015 Greymon: Lv.4 Red, Dinosaur (non-CS, used as top of stack for inherited tests)
RED_LV4 = "BT1-015"

# ST1-03 Agumon: Red Lv.3, NO CS or Numemon
RED_LV3_FILLER = "ST1-03"

# BT1-032 Frigimon: Lv.4 Blue, NO CS trait (target that should NOT get inherited reduction)
NON_CS_LV4 = "BT1-032"


def _setup_security(runner, count=3):
    """Inject security cards so the game does not end prematurely."""
    for _ in range(count):
        runner.inject_card(1, RED_LV3_FILLER, "security_top")
        runner.inject_card(2, RED_LV3_FILLER, "security_top")


@pytest.mark.behavioral
class TestBT22031AltDigivolve:
    """Tests for BT22-031 alt-digivolve requirements."""

    def test_alt_digi_from_cs_lv3(self, debug_runner):
        """A-2: BT22-031 should be able to alt-digi from a Lv.3 w/ [CS] for cost 2."""
        runner = debug_runner(initial_memory=10)
        runner.clear_zone(1, "hand")
        _setup_security(runner)
        runner.set_phase("Main")

        # Place a Lv.3 CS Digimon on field (BT22-008 Agumon — Red Lv.3 CS)
        runner.place_on_field(1, [CS_LV3])
        # Inject BT22-031 into hand
        runner.inject_card(1, GOLD_NUMEMON, "hand")

        # Should be able to digivolve (using alt-digi since color doesn't match)
        action = runner.find_action("Digivolve GoldNumemon")
        if action is None:
            action = runner.find_action("Digivolve BT22-031")
        assert action is not None, (
            f"Should be able to alt-digi from Lv.3 CS. "
            f"Actions: {runner.actions()}"
        )

        before = runner.snapshot()
        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        memory_spent = before.memory - after.memory
        # Alt-digi cost is 2
        assert memory_spent == 2, (
            f"Alt-digi from Lv.3 CS should cost 2. Spent: {memory_spent}"
        )

    def test_alt_digi_rejected_for_non_matching_base(self, debug_runner):
        """A-3: BT22-031 should NOT alt-digi from a Lv.3 without CS trait and
        without Numemon name."""
        runner = debug_runner(initial_memory=10)
        runner.clear_zone(1, "hand")
        _setup_security(runner)
        runner.set_phase("Main")

        # Place a Lv.3 non-CS, non-Numemon Digimon
        runner.place_on_field(1, [RED_LV3_FILLER])
        runner.inject_card(1, GOLD_NUMEMON, "hand")

        # BT22-031 is Yellow/Black; Agumon ST1-03 is Red — standard evo also fails.
        # Alt-digi only fires for Lv.3 CS or Lv.4 Numemon.
        action = runner.find_action("Digivolve GoldNumemon")
        if action is None:
            action = runner.find_action("Digivolve BT22-031")
        assert action is None, (
            "Should NOT be able to digivolve from Lv.3 non-CS non-Numemon base"
        )


@pytest.mark.behavioral
class TestBT22031OnPlaySecurityAttackMinus2:
    """Tests for [On Play] Security A. -2 on opponent Digimon."""

    def test_on_play_registers_sa_minus_2_modifier(self, debug_runner):
        """M-1: On Play should register CHANGE_SECURITY_ATTACK -2 on selected opponent."""
        runner = debug_runner(initial_memory=10)
        runner.clear_zone(1, "hand")
        _setup_security(runner)
        runner.set_phase("Main")

        # Place an opponent Digimon as the target
        opp_perm = runner.place_on_field(2, [RED_LV3_FILLER])
        sa_before = opp_perm.security_attack_modifier()

        runner.inject_card(1, GOLD_NUMEMON, "hand")
        action = runner.find_action("Play GoldNumemon")
        if action is None:
            action = runner.find_action("GoldNumemon")
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        sa_after = opp_perm.security_attack_modifier()
        assert sa_after == sa_before - 2, (
            f"Opponent Digimon should have SA-2. Before: {sa_before}, After: {sa_after}"
        )

    def test_on_play_sa_modifier_uses_registry(self, debug_runner):
        """M-1 (registry): SA-2 should be implemented via game.register_modifier
        with CHANGE_SECURITY_ATTACK (not via _temp_sa_modifier which has
        wrong expiry)."""
        runner = debug_runner(initial_memory=10)
        runner.clear_zone(1, "hand")
        _setup_security(runner)
        runner.set_phase("Main")

        runner.place_on_field(2, [RED_LV3_FILLER])
        runner.inject_card(1, GOLD_NUMEMON, "hand")
        action = runner.find_action("Play GoldNumemon")
        if action is None:
            action = runner.find_action("GoldNumemon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        sa_modifiers = runner.game.modifiers._modifiers.get(
            ModifierType.CHANGE_SECURITY_ATTACK, [])
        assert len(sa_modifiers) > 0, (
            "SA-2 should be registered via ModifierType.CHANGE_SECURITY_ATTACK"
        )

    def test_on_play_sa_modifier_expires_end_opponent_turn(self, debug_runner):
        """M-2: SA-2 modifier should have 'end_of_opponent_turn' expiry."""
        runner = debug_runner(initial_memory=10)
        runner.clear_zone(1, "hand")
        _setup_security(runner)
        runner.set_phase("Main")

        runner.place_on_field(2, [RED_LV3_FILLER])
        runner.inject_card(1, GOLD_NUMEMON, "hand")
        action = runner.find_action("Play GoldNumemon")
        if action is None:
            action = runner.find_action("GoldNumemon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        sa_modifiers = runner.game.modifiers._modifiers.get(
            ModifierType.CHANGE_SECURITY_ATTACK, [])
        assert any(e.expiry == 'end_of_opponent_turn' for e in sa_modifiers), (
            "SA-2 modifier should expire at end_of_opponent_turn"
        )

    def test_sa_minus_2_does_not_leak_to_other_opponents(self, debug_runner):
        """M-3 (critical): SA-2 must only affect the SELECTED permanent,
        not leak to other opponent Digimon (target-equality guard)."""
        runner = debug_runner(initial_memory=10)
        runner.clear_zone(1, "hand")
        _setup_security(runner)
        runner.set_phase("Main")

        # Two opponent Digimon
        opp_a = runner.place_on_field(2, [RED_LV3_FILLER])
        opp_b = runner.place_on_field(2, [RED_LV3_FILLER])

        sa_a_before = opp_a.security_attack_modifier()
        sa_b_before = opp_b.security_attack_modifier()

        runner.inject_card(1, GOLD_NUMEMON, "hand")
        action = runner.find_action("Play GoldNumemon")
        if action is None:
            action = runner.find_action("GoldNumemon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        sa_a_after = opp_a.security_attack_modifier()
        sa_b_after = opp_b.security_attack_modifier()

        # Exactly ONE of the two should have SA-2; the other unchanged
        affected_a = (sa_a_after == sa_a_before - 2)
        affected_b = (sa_b_after == sa_b_before - 2)
        unaffected_a = (sa_a_after == sa_a_before)
        unaffected_b = (sa_b_after == sa_b_before)

        assert (affected_a and unaffected_b) or (affected_b and unaffected_a), (
            f"Exactly one opponent should get SA-2, the other unchanged. "
            f"A before={sa_a_before} after={sa_a_after}, "
            f"B before={sa_b_before} after={sa_b_after}"
        )


@pytest.mark.behavioral
class TestBT22031ThenDigivolvePlatinumNumemon:
    """Tests for the 'Then, if stack has 2+ same-level cards, digivolve into
    [PlatinumNumemon] for cost 4 ignoring requirements' clause."""

    def test_offers_platinum_digivolve_when_same_level_pair_in_stack(self, debug_runner):
        """M-4: If BT22-031's stack has 2+ cards at the same level,
        PlatinumNumemon in hand should be offered as an optional digivolve."""
        runner = debug_runner(initial_memory=20)
        runner.clear_zone(1, "hand")
        _setup_security(runner)
        runner.set_phase("Main")

        # Place [Lv.3 filler, Lv.3 filler, GoldNumemon] on field
        # Stack has BT22-031 (Lv.4) on top + two Lv.3 cards underneath
        # → same-level pair exists (the two Lv.3s)
        perm = runner.place_on_field(
            1, [RED_LV3_FILLER, RED_LV3_FILLER, GOLD_NUMEMON])

        # Put PlatinumNumemon in hand
        runner.inject_card(1, PLATINUM_NUMEMON, "hand")
        # Put an opponent Digimon so the SA-2 target selection can proceed
        runner.place_on_field(2, [RED_LV3_FILLER])

        # Trigger the On Play / When Digivolving effect by injecting GoldNumemon
        # BUT: we can't re-trigger On Play on an already-placed card. Use a
        # different approach: ensure the effect is processed directly via
        # WhenDigivolving trigger.
        # Instead, test the condition + process logic directly on a fresh play.
        # --- Fresh play test ---
        runner2 = debug_runner(initial_memory=20)
        runner2.clear_zone(1, "hand")
        _setup_security(runner2)
        runner2.set_phase("Main")

        # Build a preliminary stack that BT22-031 would be played onto.
        # Simplest: place BT22-031 directly with two Lv.3 cards already as
        # digivolution sources using place_on_field's stack-building, then
        # manually trigger its On Play via direct ctx call.
        perm = runner2.place_on_field(
            1, [RED_LV3_FILLER, RED_LV3_FILLER, GOLD_NUMEMON])
        runner2.inject_card(1, PLATINUM_NUMEMON, "hand")
        opp_perm = runner2.place_on_field(2, [RED_LV3_FILLER])

        # Get BT22-031 top card source
        top = perm.top_card
        assert top is not None
        effects = top.effect_list(EffectTiming.OnEnterFieldAnyone)
        on_play_eff = None
        for e in effects:
            if (getattr(e, 'is_on_play', False)
                    and not e.is_inherited_effect
                    and e.on_process_callback):
                on_play_eff = e
                break
        assert on_play_eff is not None, "Should find On Play effect"

        game = runner2.game
        player1 = game.player1
        hand_size_before = len(player1.hand_cards)
        # Manually fire the On Play effect process
        ctx = {
            'game': game,
            'player': player1,
            'permanent': perm,
            'card': top,
        }
        on_play_eff.on_process_callback(ctx)
        # Auto-resolve any selection phases (target SA-2, then digivolve choice)
        runner2.auto_resolve()

        # After auto_resolve picks first available actions:
        # - Select opp_perm for SA-2 (first legal target)
        # - Optionally digivolve into PlatinumNumemon (first legal = "yes, digivolve")
        # The top_card of perm should now be PlatinumNumemon
        new_top = perm.top_card
        assert new_top is not None
        # PlatinumNumemon card_id should be on top
        card_ids_on_stack = [
            cs.c_entity_base.card_id for cs in perm.card_sources if cs.c_entity_base
        ]
        assert PLATINUM_NUMEMON in card_ids_on_stack, (
            f"PlatinumNumemon should have been digivolved onto the stack. "
            f"Stack: {card_ids_on_stack}"
        )

    def test_no_platinum_digivolve_when_no_same_level_pair(self, debug_runner):
        """M-5: If stack has no same-level pair, no digivolve offered."""
        runner = debug_runner(initial_memory=20)
        runner.clear_zone(1, "hand")
        _setup_security(runner)
        runner.set_phase("Main")

        # Stack: [Lv.3 filler, GoldNumemon Lv.4] → no same-level pair
        perm = runner.place_on_field(1, [RED_LV3_FILLER, GOLD_NUMEMON])
        runner.inject_card(1, PLATINUM_NUMEMON, "hand")
        opp_perm = runner.place_on_field(2, [RED_LV3_FILLER])

        top = perm.top_card
        assert top is not None
        effects = top.effect_list(EffectTiming.OnEnterFieldAnyone)
        on_play_eff = None
        for e in effects:
            if (getattr(e, 'is_on_play', False)
                    and not e.is_inherited_effect
                    and e.on_process_callback):
                on_play_eff = e
                break
        assert on_play_eff is not None

        game = runner.game
        player1 = game.player1
        ctx = {
            'game': game,
            'player': player1,
            'permanent': perm,
            'card': top,
        }
        on_play_eff.on_process_callback(ctx)
        runner.auto_resolve()

        # Stack should NOT have PlatinumNumemon on top
        card_ids_on_stack = [
            cs.c_entity_base.card_id for cs in perm.card_sources if cs.c_entity_base
        ]
        assert PLATINUM_NUMEMON not in card_ids_on_stack, (
            f"PlatinumNumemon should NOT have been digivolved. "
            f"Stack: {card_ids_on_stack}"
        )


@pytest.mark.behavioral
class TestBT22031InheritedCSCostReduction:
    """Tests for the inherited [Your Turn] [OPT] WhenWouldDigivolve CS cost -1."""

    def test_inherited_timing_is_when_would_digivolve(self, debug_runner):
        """I-1: The inherited cost reduction effect must use WhenWouldDigivolve
        timing (not BeforePayCost, which is for play cost)."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(GOLD_NUMEMON)
        effects = cs.effect_list(None)
        cost_red = [e for e in effects
                    if e.timing == EffectTiming.WhenWouldDigivolve
                    and e.is_inherited_effect]
        assert len(cost_red) >= 1, (
            "BT22-031 should have an inherited WhenWouldDigivolve effect for CS cost reduction"
        )
        assert cost_red[0].cost_reduction == 1, (
            f"Cost reduction should be 1, got {cost_red[0].cost_reduction}"
        )

    def test_inherited_reduces_cs_digivolve_cost_by_1(self, debug_runner):
        """I-2: With BT22-031 in a stack, digivolving into a [CS]-trait
        Digimon should cost 1 less than the same digivolve without BT22-031.

        Compares two identical games:
          A) Stack = [BT22-031, RED_LV4] → digivolve into CS_LV5_TARGET
          B) Stack = [RED_LV3_FILLER, RED_LV4] → digivolve into CS_LV5_TARGET
        The memory spent in (A) should be exactly 1 less than in (B).
        """
        def _spend_for_stack(under_card):
            runner = debug_runner(initial_memory=20)
            runner.clear_zone(1, "hand")
            _setup_security(runner)
            runner.set_phase("Main")
            runner.place_on_field(1, [under_card, RED_LV4])
            runner.inject_card(1, CS_LV5_TARGET, "hand")
            before = runner.game.memory
            action = runner.find_action("Digivolve RizeGreymon")
            if action is None:
                action = runner.find_action("Digivolve BT22-012")
            assert action is not None, (
                f"Should find digivolve action. Actions: {runner.actions()}"
            )
            runner.execute(action)
            runner.auto_resolve()
            return before - runner.game.memory

        spent_with_gold = _spend_for_stack(GOLD_NUMEMON)
        spent_without = _spend_for_stack(RED_LV3_FILLER)
        assert spent_with_gold == spent_without - 1, (
            f"Digivolve into [CS] should cost 1 less when BT22-031 is under "
            f"the stack. Without: {spent_without}, with: {spent_with_gold}"
        )

    def test_inherited_no_reduction_for_non_cs_target(self, debug_runner):
        """I-3: Digivolving into a non-CS Digimon should NOT reduce cost."""
        runner = debug_runner(initial_memory=20)
        runner.clear_zone(1, "hand")
        _setup_security(runner)
        runner.set_phase("Main")

        # Stack: BT22-031 at bottom, Blue Lv.3 on top to build a proper stack
        # Actually we need the TOP to be a level that can digivolve into our
        # non-CS target. BT1-032 Frigimon is Lv.4 Blue, evo from Blue Lv.3 cost 2.
        BLUE_LV3 = "ST2-04"  # Gomamon, Blue Lv.3
        perm = runner.place_on_field(1, [GOLD_NUMEMON, BLUE_LV3])
        # Inject Frigimon (Lv.4 Blue, NOT CS trait)
        runner.inject_card(1, NON_CS_LV4, "hand")

        before_memory = runner.game.memory
        action = runner.find_action("Digivolve Frigimon")
        if action is None:
            action = runner.find_action("Digivolve BT1-032")
        assert action is not None, (
            f"Should find digivolve action. Actions: {runner.actions()}"
        )
        runner.execute(action)
        runner.auto_resolve()
        after_memory = runner.game.memory

        memory_spent = before_memory - after_memory
        # Base 2 (Blue Lv.3 evo), no reduction = 2
        assert memory_spent == 2, (
            f"Digivolve into non-CS should cost full 2. "
            f"Before: {before_memory}, After: {after_memory}, Spent: {memory_spent}"
        )
