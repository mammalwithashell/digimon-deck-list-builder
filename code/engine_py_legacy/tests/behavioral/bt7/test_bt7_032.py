"""Behavioral tests for BT7-032 Pulsemon.

Card:
  BT7-032 Pulsemon | Lv.3 Yellow Rookie Digimon | DP 2000 | Cost 3
  Type: Beastkin | Attribute: Vaccine

Card Text:
  Inherited Effect [When Attacking] [Once Per Turn]
    If you have 3 security cards, gain 2 memory.

Key faithfulness points tested:
  1. Effect is inherited-only (fires from digivolution stack under attacker).
  2. Triggers on [When Attacking] (OnUseAttack timing, is_on_attack flag).
  3. [Once Per Turn] — does not fire more than once per turn.
  4. Condition: owner must have EXACTLY 3 security cards (not more, not less).
  5. Action: owner gains 2 memory.
  6. Does NOT fire when BT7-032 is itself the top (only triggers as inherited).
  7. Opponent attacking with own Digimon does not gain memory from P1's
     BT7-032 (owner gate: condition checks the attacking stack's owner).

C# reference: BT7_032.cs
  - C# uses EffectTiming.OnAllyAttack + CanTriggerOnAttack helper (which
    gates to attacker). Python engine semantics map this to
    EffectTiming.OnUseAttack + is_on_attack = True (engine gates perm==attacker).
  - C# checks `card.Owner.SecurityCards.Count == 3` — strictly 3.
  - C# is_inherited_effect = True, set_hash_string("Memory+2_BT7_032"),
    max_count_per_turn = 1.
"""

import pytest

# Card IDs
BT7_032 = "BT7-032"        # Pulsemon (card under test)
REPPAMON = "BT1-051"       # Lv.4 Yellow, no script/effect — clean host
DIATRYMON = "BT4-040"      # Lv.4 Yellow, no script/effect — alternate host
AGUMON_ST1 = "ST1-03"      # Lv.3 Red filler

# Filler deck: must have enough cards for game setup (50+)
FILLER = [AGUMON_ST1] * 50


@pytest.mark.behavioral
class TestBT7032InheritedGainMemory:
    """Inherited [When Attacking] [Once Per Turn] — gain 2 memory if sec == 3."""

    def test_gains_2_memory_when_security_is_exactly_3(self, debug_runner):
        """When attacking with a Digimon that has BT7-032 inherited, and the
        owner has exactly 3 security cards, owner should gain 2 memory."""
        runner = debug_runner(
            deck1=[BT7_032] * 4 + [REPPAMON] * 4 + [AGUMON_ST1] * 42,
            deck2=FILLER,
            initial_memory=0,
        )
        runner.set_phase("Main")

        # Stack: BT7-032 (bottom) -> Reppamon (top)
        runner.place_on_field(1, [BT7_032, REPPAMON])

        # Force exactly 3 security cards for P1
        runner.clear_zone(1, "security")
        for _ in range(3):
            runner.inject_card(1, AGUMON_ST1, "security_top")

        # Give opponent security so attack doesn't end the game
        runner.clear_zone(2, "security")
        for _ in range(5):
            runner.inject_card(2, AGUMON_ST1, "security_top")

        before = runner.snapshot()
        memory_before = before.memory

        attack_action = runner.find_action("Attack")
        assert attack_action is not None, (
            f"Reppamon should be able to attack. Actions: {runner.actions()}"
        )
        runner.execute(attack_action)
        runner.auto_resolve()

        after = runner.snapshot()
        # DCGO: AddMemory(2) — P1 gains 2 memory (memory sign matters)
        # Engine convention: memory >= 0 is current player's memory, so after
        # P1's effect adds 2, P1's memory should have increased by 2 (unless
        # the attack's rule-process passed turn).
        # The simplest check: look for the +2 change attributable to the
        # inherited effect. We snapshot before/after and compare.
        delta = after.memory - memory_before
        assert delta >= 2, (
            f"Expected owner to gain at least 2 memory from BT7-032 inherited "
            f"effect (security == 3). Memory before={memory_before}, "
            f"after={after.memory}, delta={delta}"
        )

    def test_no_memory_gain_when_security_is_less_than_3(self, debug_runner):
        """When security count is 2 (not 3), the effect should NOT trigger."""
        runner = debug_runner(
            deck1=[BT7_032] * 4 + [REPPAMON] * 4 + [AGUMON_ST1] * 42,
            deck2=FILLER,
            initial_memory=0,
        )
        runner.set_phase("Main")

        runner.place_on_field(1, [BT7_032, REPPAMON])

        # Exactly 2 security cards — under the threshold
        runner.clear_zone(1, "security")
        for _ in range(2):
            runner.inject_card(1, AGUMON_ST1, "security_top")

        runner.clear_zone(2, "security")
        for _ in range(5):
            runner.inject_card(2, AGUMON_ST1, "security_top")

        before = runner.snapshot()
        memory_before = before.memory

        attack_action = runner.find_action("Attack")
        assert attack_action is not None
        runner.execute(attack_action)
        runner.auto_resolve()

        after = runner.snapshot()
        delta = after.memory - memory_before
        # The only non-combat memory change from this effect would be +2.
        # With security == 2, the effect should not fire, so delta should not
        # include the +2 bonus. Accept <=0 (turn pass) or <2 (no bonus).
        assert delta < 2, (
            f"With security == 2, BT7-032 should NOT fire. "
            f"Memory delta={delta} suggests the effect triggered incorrectly."
        )

    def test_no_memory_gain_when_security_is_more_than_3(self, debug_runner):
        """When security count is 4 (more than 3), the effect should NOT trigger.

        C# checks `SecurityCards.Count == 3` strictly — 4 is not 3.
        """
        runner = debug_runner(
            deck1=[BT7_032] * 4 + [REPPAMON] * 4 + [AGUMON_ST1] * 42,
            deck2=FILLER,
            initial_memory=0,
        )
        runner.set_phase("Main")

        runner.place_on_field(1, [BT7_032, REPPAMON])

        # Exactly 4 security cards — over the threshold
        runner.clear_zone(1, "security")
        for _ in range(4):
            runner.inject_card(1, AGUMON_ST1, "security_top")

        runner.clear_zone(2, "security")
        for _ in range(5):
            runner.inject_card(2, AGUMON_ST1, "security_top")

        before = runner.snapshot()
        memory_before = before.memory

        attack_action = runner.find_action("Attack")
        assert attack_action is not None
        runner.execute(attack_action)
        runner.auto_resolve()

        after = runner.snapshot()
        delta = after.memory - memory_before
        assert delta < 2, (
            f"With security == 4, BT7-032 should NOT fire (strict == 3). "
            f"Memory delta={delta} suggests the effect triggered incorrectly."
        )

    def test_no_memory_gain_when_security_is_zero(self, debug_runner):
        """When security count is 0, the effect should NOT trigger."""
        runner = debug_runner(
            deck1=[BT7_032] * 4 + [REPPAMON] * 4 + [AGUMON_ST1] * 42,
            deck2=FILLER,
            initial_memory=0,
        )
        runner.set_phase("Main")

        runner.place_on_field(1, [BT7_032, REPPAMON])

        runner.clear_zone(1, "security")
        runner.clear_zone(2, "security")
        for _ in range(5):
            runner.inject_card(2, AGUMON_ST1, "security_top")

        before = runner.snapshot()
        memory_before = before.memory

        attack_action = runner.find_action("Attack")
        assert attack_action is not None
        runner.execute(attack_action)
        runner.auto_resolve()

        after = runner.snapshot()
        delta = after.memory - memory_before
        assert delta < 2, (
            f"With security == 0, BT7-032 should NOT fire. "
            f"Memory delta={delta}"
        )

    def test_once_per_turn(self, debug_runner):
        """[Once Per Turn] — should not fire more than once per turn."""
        runner = debug_runner(
            deck1=[BT7_032] * 4 + [REPPAMON] * 4 + [AGUMON_ST1] * 42,
            deck2=FILLER,
            initial_memory=0,
        )
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [BT7_032, REPPAMON])

        runner.clear_zone(1, "security")
        for _ in range(3):
            runner.inject_card(1, AGUMON_ST1, "security_top")

        runner.clear_zone(2, "security")
        for _ in range(10):
            runner.inject_card(2, AGUMON_ST1, "security_top")

        memory_before_first = runner.snapshot().memory

        # First attack — should trigger the effect
        attack_action = runner.find_action("Attack")
        assert attack_action is not None
        runner.execute(attack_action)
        runner.auto_resolve()

        memory_after_first = runner.snapshot().memory
        first_gain = memory_after_first - memory_before_first

        # Reset: unsuspend and restore to Main so we can attack again
        perm.unsuspend()
        runner.set_phase("Main")
        # Make sure opponent still has 3+ security
        while len(runner.game.player2.security_cards) < 3:
            runner.inject_card(2, AGUMON_ST1, "security_top")
        # Ensure P1 still has exactly 3 security for the condition
        while len(runner.game.player1.security_cards) < 3:
            runner.inject_card(1, AGUMON_ST1, "security_top")
        while len(runner.game.player1.security_cards) > 3:
            runner.game.player1.security_cards.pop()

        memory_before_second = runner.snapshot().memory
        attack_action2 = runner.find_action("Attack")
        if attack_action2 is not None:
            runner.execute(attack_action2)
            runner.auto_resolve()

            memory_after_second = runner.snapshot().memory
            second_gain = memory_after_second - memory_before_second

            # The second attack should NOT gain another +2 from the inherited
            # OPT effect. It's ok for memory to otherwise shift from combat.
            assert second_gain < 2, (
                f"Second attack this turn should NOT trigger BT7-032 OPT. "
                f"First gain={first_gain}, second gain={second_gain}"
            )

    def test_is_inherited_effect_flag(self, debug_runner):
        """BT7-032's effect must have is_inherited_effect = True (it's only
        an inherited effect, per card text)."""
        runner = debug_runner(
            deck1=[BT7_032] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=0,
        )

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(BT7_032, runner.game.player1)
        effects = cs.effect_list(None)

        assert len(effects) >= 1, "BT7-032 should have at least 1 effect"
        for eff in effects:
            assert eff.is_inherited_effect is True, (
                f"All effects on BT7-032 should be inherited. "
                f"Found non-inherited: {eff.effect_name}"
            )

    def test_is_on_attack_flag(self, debug_runner):
        """The BT7-032 inherited effect must have is_on_attack = True
        (engine gate for OnUseAttack)."""
        runner = debug_runner(
            deck1=[BT7_032] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=0,
        )

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(BT7_032, runner.game.player1)
        effects = cs.effect_list(None)

        on_attack = [e for e in effects if getattr(e, 'is_on_attack', False)]
        assert len(on_attack) >= 1, (
            "BT7-032 should have an is_on_attack=True effect (OnUseAttack)"
        )

    def test_effect_does_not_fire_as_top_card(self, debug_runner):
        """When BT7-032 is the top of a stack (not inherited), its inherited
        effect is NOT active — inherited effects only fire from the underlayer.

        This is critical: Permanent.effect_list() skips is_inherited_effect
        effects from the top card.
        """
        runner = debug_runner(
            deck1=[BT7_032] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
            initial_memory=10,  # enough to play Pulsemon (cost 3) + attack
        )
        runner.set_phase("Main")

        # Place BT7-032 as the sole card (no stack beneath), so it's the top.
        runner.place_on_field(1, [BT7_032])

        runner.clear_zone(1, "security")
        for _ in range(3):
            runner.inject_card(1, AGUMON_ST1, "security_top")

        runner.clear_zone(2, "security")
        for _ in range(5):
            runner.inject_card(2, AGUMON_ST1, "security_top")

        memory_before = runner.snapshot().memory

        attack_action = runner.find_action("Attack")
        # Pulsemon is DP 2000 and may or may not be able to attack player —
        # let it try. If not available, the test passes trivially.
        if attack_action is None:
            pytest.skip("Pulsemon has no available attack in this state.")
        runner.execute(attack_action)
        runner.auto_resolve()

        memory_after = runner.snapshot().memory
        delta = memory_after - memory_before
        # The inherited effect should NOT fire from the top card.
        assert delta < 2, (
            f"Inherited effect fired from top card! delta={delta}. "
            f"Inherited effects should only fire from the under-stack."
        )

    def test_opponent_attacker_does_not_trigger_p1_pulsemon(self, debug_runner):
        """P2 attacking with their own Digimon does NOT trigger P1's BT7-032
        inherited effect. The engine's `perm is attacker` gate in
        _effect_matches_timing(OnUseAttack) ensures only the attacking
        permanent's own effects fire.
        """
        runner = debug_runner(
            deck1=[BT7_032] * 4 + [REPPAMON] * 4 + [AGUMON_ST1] * 42,
            deck2=FILLER,
            initial_memory=0,
        )
        runner.set_phase("Main")

        # P1 has BT7-032 under Reppamon on their field.
        runner.place_on_field(1, [BT7_032, REPPAMON])
        # P2 has a vanilla attacker (no BT7-032 in stack)
        runner.place_on_field(2, [REPPAMON])

        # P1 has exactly 3 security (if condition were misgated, effect
        # would fire).
        runner.clear_zone(1, "security")
        for _ in range(3):
            runner.inject_card(1, AGUMON_ST1, "security_top")

        runner.clear_zone(2, "security")
        for _ in range(5):
            runner.inject_card(2, AGUMON_ST1, "security_top")

        # Drive P2 into an attack via the engine's execute_effects hook, which
        # is what combat.declare_attack() does internally. We bypass the
        # higher-level action API (which requires full turn-player setup)
        # and directly fire the OnUseAttack timing with P2's Reppamon as the
        # attacker. If the gating is correct, P1's BT7-032 will NOT activate.
        from engine_py_legacy.engine.data.enums import EffectTiming
        p2_attacker = runner.game.player2.battle_area[0]

        # Capture the OPT state of the BT7-032 inherited effect before/after.
        p1_perm = runner.game.player1.battle_area[0]
        pulsemon_source = p1_perm.card_sources[0]
        effects = pulsemon_source.effect_list(None)
        pulse_effect = next(
            (e for e in effects if getattr(e, 'is_on_attack', False)),
            None,
        )
        assert pulse_effect is not None, "BT7-032 OnUseAttack effect missing"
        assert pulse_effect.can_activate_this_turn(), "Pre-check: should be usable"

        # Fire OnUseAttack with P2's Reppamon as attacker — simulates what
        # combat.py does when an attack is declared.
        runner.game.execute_effects(
            EffectTiming.OnUseAttack, {"attacker": p2_attacker}
        )

        # P1's BT7-032 inherited effect should not have fired — the engine
        # gates by `perm is attacker`. If it had fired, can_activate_this_turn
        # would be False (OPT exhausted).
        assert pulse_effect.can_activate_this_turn(), (
            "P1's BT7-032 must NOT fire when P2's Digimon is the attacker "
            "(engine gate: perm is attacker)."
        )


@pytest.mark.behavioral
class TestBT7032Metadata:
    """Script metadata: OPT hash, max_count_per_turn, description."""

    def test_opt_max_count_per_turn_is_1(self, debug_runner):
        """The effect should have max_count_per_turn = 1 for OPT enforcement."""
        runner = debug_runner(
            deck1=[BT7_032] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
        )
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(BT7_032, runner.game.player1)
        effects = cs.effect_list(None)
        opt_effects = [
            e for e in effects
            if getattr(e, 'max_count_per_turn', 0) == 1
        ]
        assert len(opt_effects) >= 1, (
            "BT7-032 should have at least one effect with "
            "max_count_per_turn=1 (OPT)"
        )

    def test_hash_string_matches_csharp(self, debug_runner):
        """The OPT hash string must match the C# source ("Memory+2_BT7_032")
        for cross-implementation traceability and duplicate tracking."""
        runner = debug_runner(
            deck1=[BT7_032] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
        )
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(BT7_032, runner.game.player1)
        effects = cs.effect_list(None)

        opt_eff = next(
            (e for e in effects
             if getattr(e, 'max_count_per_turn', 0) == 1),
            None,
        )
        assert opt_eff is not None, "Expected an OPT-flagged effect"
        assert opt_eff.hash_string == "Memory+2_BT7_032", (
            f"Hash string should match C# ('Memory+2_BT7_032'), "
            f"got {opt_eff.hash_string!r}"
        )

    def test_timing_is_on_use_attack(self, debug_runner):
        """The effect timing should be OnUseAttack (28), not OnAllyAttack."""
        runner = debug_runner(
            deck1=[BT7_032] * 4 + [AGUMON_ST1] * 46,
            deck2=FILLER,
        )
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.enums import EffectTiming
        db = CardDatabase()
        cs = db.create_card_source(BT7_032, runner.game.player1)
        effects = cs.effect_list(None)

        opt_eff = next(
            (e for e in effects
             if getattr(e, 'is_on_attack', False)),
            None,
        )
        assert opt_eff is not None
        assert opt_eff.timing == EffectTiming.OnUseAttack, (
            f"Expected EffectTiming.OnUseAttack (28), got {opt_eff.timing}"
        )
