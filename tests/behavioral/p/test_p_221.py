"""Behavioral tests for P-221 Chaosmon | Lv.7 (Yellow/Purple, Unique, Vaccine).

Card text:
  <Security A. +1>
  <Partition (Yellow Lv.6 & Purple/Black Lv.6)>
  [When Digivolving] If DNA digivolving, until your opponent's turn ends,
    their effects don't affect this Digimon.
  [When Digivolving] [When Attacking] 1 of your opponent's Digimon gets
    -10000 DP until their turn ends.

Inherited: (none)

Clauses under test:
  C1 <Security A. +1>            — factory keyword via _security_attack_modifier
  C2 <Partition>                  — documentary marker (engine gap)
  C3 [WD] DNA → effects don't affect this Digimon until end of opp turn
  C4 [WD] / [WA] → -10000 DP to 1 opp Digimon until their turn ends

Fixes being verified:
  * register_modifier called with correct positional args (perm first)
  * CHANGE_DP value_fn uses the (cur, target, ctx) arity
  * Target-equality guard on host perm for C3, on selected target for C4
  * expiry='end_of_opponent_turn' (card says "until their turn ends")
  * C3 only grants immunity on DNA digivolving (not normal alt-digi / unrelated
    digivolution)
  * C4 has BOTH a When Digivolving AND a When Attacking instance
"""

from __future__ import annotations

import pytest

from digimon_gym.engine.data.enums import EffectTiming
from digimon_gym.engine.interfaces.modifiers import ModifierType


CHAOSMON = "P-221"


def _find_wd_effect(perm, *, keyword: str):
    """Find a [When Digivolving] effect whose description contains `keyword`."""
    for e in perm.top_card.effect_list(None):
        if not getattr(e, 'is_when_digivolving', False):
            continue
        desc = (e.effect_description or '').lower()
        if keyword in desc:
            return e
    return None


def _immunity_wd(perm):
    """The DNA-immunity [WD] effect — has 'effects don't affect' in text."""
    return _find_wd_effect(perm, keyword="effects don't affect")


def _dp_wd(perm):
    """The -10000 DP [WD] effect — has '-10000 dp' in text."""
    return _find_wd_effect(perm, keyword="-10000 dp")


def _wa_effect(perm):
    """The [When Attacking] effect (OnUseAttack + is_on_attack)."""
    for e in perm.top_card.effect_list(None):
        if (
            getattr(e, 'timing', None) == EffectTiming.OnUseAttack
            and getattr(e, 'is_on_attack', False)
        ):
            return e
    return None

# Filler for decks / injection
FILLER_RED = "ST1-03"         # Agumon Lv.3 Red (no Yellow/Purple/Black)
FILLER_BLUE = "ST2-04"        # Gomamon Lv.3 Blue


def _make_deck():
    """Minimal 50-card deck containing P-221 and filler."""
    eggs = ["BT2-001"] * 4
    core = [CHAOSMON]
    filler = [FILLER_RED] * (50 - len(eggs) - len(core))
    return eggs + core + filler


# ── Script-structure tests ──────────────────────────────────────────────


@pytest.mark.behavioral
class TestP221Structure:
    """Structural tests for P-221 effect list."""

    def test_chaosmon_is_registered(self, debug_runner):
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        perm = runner.place_on_field(1, [CHAOSMON])
        assert perm.top_card is not None
        assert perm.top_card.c_entity_base.card_id == CHAOSMON

    def test_security_attack_plus_keyword_present(self, debug_runner):
        """C1: <Security A. +1> — one effect with _security_attack_modifier == 1."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        perm = runner.place_on_field(1, [CHAOSMON])
        effects = perm.top_card.effect_list(None)
        sa_effects = [
            e for e in effects
            if getattr(e, '_security_attack_modifier', 0) == 1
        ]
        assert len(sa_effects) >= 1, (
            "P-221 should have a <Security A. +1> factory effect with "
            "_security_attack_modifier = 1"
        )

    def test_partition_marker_present(self, debug_runner):
        """C2: <Partition> — documentary marker via _is_partition = True."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        perm = runner.place_on_field(1, [CHAOSMON])
        effects = perm.top_card.effect_list(None)
        partition_effects = [
            e for e in effects if getattr(e, '_is_partition', False)
        ]
        assert len(partition_effects) >= 1, (
            "P-221 should have a <Partition> marker effect with _is_partition = True"
        )

    def test_partition_conditions_metadata(self, debug_runner):
        """C2: partition conditions should be documented (Yellow Lv.6 + Purple/Black Lv.6)."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        perm = runner.place_on_field(1, [CHAOSMON])
        effects = perm.top_card.effect_list(None)
        partition_effects = [
            e for e in effects if getattr(e, '_is_partition', False)
        ]
        assert partition_effects, "Sanity: partition marker should exist"
        eff = partition_effects[0]
        # Documentary metadata — keys may be any sensible shape; verify presence.
        assert hasattr(eff, '_partition_conditions'), (
            "Partition marker should expose _partition_conditions metadata "
            "describing 'Yellow Lv.6' and 'Purple/Black Lv.6' for RL/observability"
        )
        conds = eff._partition_conditions
        assert isinstance(conds, (list, tuple)) and len(conds) == 2, (
            f"_partition_conditions should be 2-element sequence, got {conds!r}"
        )

    def test_exactly_two_when_digivolving_effects(self, debug_runner):
        """C3 + C4: there must be TWO When Digivolving effects.

        Card text has two separate [When Digivolving] clauses:
          - DNA immunity
          - -10000 DP to 1 opp Digimon
        """
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        perm = runner.place_on_field(1, [CHAOSMON])
        effects = perm.top_card.effect_list(None)
        wd_effects = [
            e for e in effects
            if getattr(e, 'is_when_digivolving', False)
        ]
        assert len(wd_effects) == 2, (
            f"P-221 should have exactly 2 [When Digivolving] effects "
            f"(DNA immunity + -10000 DP), got {len(wd_effects)}"
        )

    def test_exactly_one_when_attacking_effect(self, debug_runner):
        """C4: there must be ONE When Attacking effect (-10000 DP).

        It must use OnUseAttack (self-attacker) with is_on_attack = True,
        NOT OnAllyAttack.
        """
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        perm = runner.place_on_field(1, [CHAOSMON])
        effects = perm.top_card.effect_list(None)
        wa_effects = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnUseAttack
            and getattr(e, 'is_on_attack', False)
        ]
        assert len(wa_effects) == 1, (
            f"P-221 should have exactly 1 [When Attacking] effect using "
            f"OnUseAttack + is_on_attack, got {len(wa_effects)}"
        )

    def test_no_on_ally_attack_misuse(self, debug_runner):
        """Confirm the [When Attacking] effect is NOT wired as OnAllyAttack.

        "When Attacking" triggers when THIS Digimon attacks, not allies.
        """
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        perm = runner.place_on_field(1, [CHAOSMON])
        effects = perm.top_card.effect_list(None)
        bad = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnAllyAttack
        ]
        assert not bad, (
            "P-221's [When Attacking] must use OnUseAttack, not OnAllyAttack "
            "(per C# and 16-item checklist item #2)"
        )


# ── C3: DNA-only immunity via CANNOT_BE_AFFECTED ────────────────────────


@pytest.mark.behavioral
class TestP221DNAImmunity:
    """C3: [WD] If DNA digivolving, opponent's effects don't affect this Digimon."""

    def test_dna_digivolve_grants_cannot_be_affected(self, debug_runner):
        """Invoking the process callback with is_dna_digivolve=True should
        register a CANNOT_BE_AFFECTED modifier on the host permanent."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        perm = runner.place_on_field(1, [CHAOSMON])
        game = runner.game

        eff = _immunity_wd(perm)
        assert eff is not None, "DNA immunity WD effect not found"

        # Invoke directly with DNA flag
        eff.on_process_callback({
            'player': game.player1,
            'permanent': perm,
            'game': game,
            'is_dna_digivolve': True,
        })

        has_immunity = game.modifiers.has_modifier(
            perm, ModifierType.CANNOT_BE_AFFECTED
        )
        assert has_immunity, (
            "DNA digivolving should grant CANNOT_BE_AFFECTED to the host "
            "permanent"
        )

    def test_non_dna_digivolve_no_immunity(self, debug_runner):
        """Normal (non-DNA) digivolution path should NOT grant immunity."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        perm = runner.place_on_field(1, [CHAOSMON])
        game = runner.game

        eff = _immunity_wd(perm)
        assert eff is not None, "DNA immunity WD effect not found"

        # Invoke without DNA flag
        eff.on_process_callback({
            'player': game.player1,
            'permanent': perm,
            'game': game,
            'is_dna_digivolve': False,
        })

        has_immunity = game.modifiers.has_modifier(
            perm, ModifierType.CANNOT_BE_AFFECTED
        )
        assert not has_immunity, (
            "Non-DNA digivolving must NOT grant CANNOT_BE_AFFECTED "
            "(card says 'If DNA digivolving')"
        )

    def test_immunity_does_not_leak_to_other_permanents(self, debug_runner):
        """CANNOT_BE_AFFECTED must only apply to the host Chaosmon, not to
        every permanent in play."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        chaosmon_perm = runner.place_on_field(1, [CHAOSMON])
        bystander_perm = runner.place_on_field(1, [FILLER_RED])
        opp_perm = runner.place_on_field(2, [FILLER_RED])
        game = runner.game

        eff = _immunity_wd(chaosmon_perm)
        assert eff is not None
        eff.on_process_callback({
            'player': game.player1,
            'permanent': chaosmon_perm,
            'game': game,
            'is_dna_digivolve': True,
        })

        assert game.modifiers.has_modifier(
            chaosmon_perm, ModifierType.CANNOT_BE_AFFECTED
        ), "Chaosmon itself must have immunity"
        assert not game.modifiers.has_modifier(
            bystander_perm, ModifierType.CANNOT_BE_AFFECTED
        ), "Own bystander must NOT inherit Chaosmon's immunity"
        assert not game.modifiers.has_modifier(
            opp_perm, ModifierType.CANNOT_BE_AFFECTED
        ), "Opponent permanent must NOT inherit Chaosmon's immunity"

    def test_immunity_expiry_end_of_opponent_turn(self, debug_runner):
        """The granted immunity must have expiry='end_of_opponent_turn'
        (card says 'until your opponent's turn ends')."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        perm = runner.place_on_field(1, [CHAOSMON])
        game = runner.game

        eff = _immunity_wd(perm)
        assert eff is not None
        eff.on_process_callback({
            'player': game.player1,
            'permanent': perm,
            'game': game,
            'is_dna_digivolve': True,
        })

        # Find the entry and check expiry
        entries = game.modifiers._modifiers.get(
            ModifierType.CANNOT_BE_AFFECTED, []
        )
        relevant = [e for e in entries if e.source_permanent is perm]
        assert relevant, "No CANNOT_BE_AFFECTED entry found for Chaosmon"
        assert any(e.expiry == 'end_of_opponent_turn' for e in relevant), (
            "Immunity must expire at end of opponent's turn, got: "
            f"{[e.expiry for e in relevant]}"
        )


# ── C4: -10000 DP to 1 opp Digimon ─────────────────────────────────────


@pytest.mark.behavioral
class TestP221MinusDP:
    """C4: [WD] / [WA] 1 of your opponent's Digimon gets -10000 DP."""

    def _invoke_wd_minus_dp(self, runner, perm):
        """Invoke the [When Digivolving] -10000 DP effect."""
        eff = _dp_wd(perm)
        assert eff is not None, "DP WD effect not found"
        eff.on_process_callback({
            'player': runner.game.player1,
            'permanent': perm,
            'game': runner.game,
        })
        return eff

    def _invoke_wa_minus_dp(self, runner, perm):
        """Invoke the [When Attacking] -10000 DP effect."""
        eff = _wa_effect(perm)
        assert eff is not None, "WA effect not found"
        eff.on_process_callback({
            'player': runner.game.player1,
            'permanent': perm,
            'game': runner.game,
            'attacker': perm,
        })
        return eff

    def test_wd_minus_dp_opens_select_target(self, debug_runner):
        """The [WD] DP effect must ask the player to SELECT 1 opp Digimon
        (no auto-selection, per No-Approximations policy)."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        chaosmon = runner.place_on_field(1, [CHAOSMON])
        runner.place_on_field(2, [FILLER_RED])
        runner.place_on_field(2, [FILLER_BLUE])

        game = runner.game
        self._invoke_wd_minus_dp(runner, chaosmon)

        # After invocation, there should be a target selection pending
        from digimon_gym.engine.data.enums import GamePhase
        assert game.current_phase == GamePhase.SelectTarget, (
            f"Effect should open SelectTarget phase to choose 1 opp Digimon, "
            f"got {game.current_phase.name}"
        )
        legal = runner.action_mask()
        assert len(legal) >= 2, (
            f"SelectTarget should offer >=2 choices when 2 opp Digimon exist, "
            f"got {len(legal)}"
        )

    def test_wd_minus_dp_applies_to_selected_target_only(self, debug_runner):
        """After selecting a target, only THAT opp Digimon should have the
        CHANGE_DP -10000 modifier."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        chaosmon = runner.place_on_field(1, [CHAOSMON])
        target = runner.place_on_field(2, [FILLER_RED])
        bystander = runner.place_on_field(2, [FILLER_BLUE])

        game = runner.game
        self._invoke_wd_minus_dp(runner, chaosmon)

        # Auto-resolve the target selection (picks first legal option)
        runner.auto_resolve()

        # Check that exactly one opp perm has a -10000 CHANGE_DP modifier
        # applicable (the one selected). We check via the modifier registry
        # directly to isolate this effect's registration.
        entries = game.modifiers._modifiers.get(
            ModifierType.CHANGE_DP, []
        )
        p221_entries = [e for e in entries if e.source_permanent in (target, bystander)]
        assert len(p221_entries) == 1, (
            f"Exactly one opponent Digimon should receive the -10000 CHANGE_DP "
            f"modifier entry, got {len(p221_entries)}"
        )
        # Verify that the entry returns -10000 when queried
        entry = p221_entries[0]
        assert entry.value_fn is not None, (
            "CHANGE_DP entry must have a value_fn"
        )
        # value_fn should follow (cur, target, ctx) arity
        try:
            result = entry.value_fn(0, entry.source_permanent, {})
        except TypeError:
            pytest.fail(
                "CHANGE_DP value_fn must accept (cur, target, ctx) arity, "
                "not a zero-arg lambda"
            )
        assert result == -10000, (
            f"value_fn should produce -10000 DP change, got {result}"
        )

    def test_wd_minus_dp_reduces_effective_dp(self, debug_runner):
        """The selected opp Digimon's effective DP should drop by 10000."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        chaosmon = runner.place_on_field(1, [CHAOSMON])
        target = runner.place_on_field(2, [FILLER_RED])

        game = runner.game
        dp_before = target.dp
        assert dp_before is not None, "Target must have a DP value"

        self._invoke_wd_minus_dp(runner, chaosmon)
        runner.auto_resolve()

        dp_after = target.dp
        # Since base DP of Agumon ST1-03 < 10000, effective DP will floor at 0,
        # but the entry-level delta is -10000 (verified elsewhere). Here we
        # assert the net effect: DP is strictly reduced by the modifier.
        assert dp_after < dp_before, (
            f"Selected opp Digimon should have reduced DP after -10000 "
            f"modifier, before={dp_before}, after={dp_after}"
        )

    def test_wd_minus_dp_expiry_end_of_opponent_turn(self, debug_runner):
        """Card says 'until their turn ends' (opponent's turn). Expiry must
        be 'end_of_opponent_turn'."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        chaosmon = runner.place_on_field(1, [CHAOSMON])
        target = runner.place_on_field(2, [FILLER_RED])

        game = runner.game
        self._invoke_wd_minus_dp(runner, chaosmon)
        runner.auto_resolve()

        entries = game.modifiers._modifiers.get(
            ModifierType.CHANGE_DP, []
        )
        p221_entries = [
            e for e in entries if e.source_permanent is target
        ]
        assert p221_entries, "No CHANGE_DP entry found on target"
        assert all(e.expiry == 'end_of_opponent_turn' for e in p221_entries), (
            f"Expected expiry='end_of_opponent_turn', got "
            f"{[e.expiry for e in p221_entries]}"
        )

    def test_wa_minus_dp_opens_select_target(self, debug_runner):
        """The [WA] DP effect must also ask the player to SELECT 1 opp Digimon."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        chaosmon = runner.place_on_field(1, [CHAOSMON])
        runner.place_on_field(2, [FILLER_RED])
        runner.place_on_field(2, [FILLER_BLUE])

        game = runner.game
        self._invoke_wa_minus_dp(runner, chaosmon)

        from digimon_gym.engine.data.enums import GamePhase
        assert game.current_phase == GamePhase.SelectTarget, (
            f"[WA] should open SelectTarget, got {game.current_phase.name}"
        )

    def test_wa_minus_dp_applies_modifier(self, debug_runner):
        """The [WA] effect should register a CHANGE_DP entry as well."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        chaosmon = runner.place_on_field(1, [CHAOSMON])
        target = runner.place_on_field(2, [FILLER_RED])

        game = runner.game
        self._invoke_wa_minus_dp(runner, chaosmon)
        runner.auto_resolve()

        entries = game.modifiers._modifiers.get(
            ModifierType.CHANGE_DP, []
        )
        p221_entries = [e for e in entries if e.source_permanent is target]
        assert len(p221_entries) == 1, (
            f"[WA] should register one CHANGE_DP entry, got {len(p221_entries)}"
        )
        entry = p221_entries[0]
        result = entry.value_fn(0, entry.source_permanent, {})
        assert result == -10000

    def test_minus_dp_no_crash_when_no_opp_digimon(self, debug_runner):
        """With no opponent Digimon available, the effect should no-op, not
        crash."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        chaosmon = runner.place_on_field(1, [CHAOSMON])
        game = runner.game
        # No opp permanents
        assert not game.player2.battle_area

        self._invoke_wd_minus_dp(runner, chaosmon)
        # Should not have crashed; no modifier should exist
        entries = game.modifiers._modifiers.get(
            ModifierType.CHANGE_DP, []
        )
        assert not entries, (
            "No CHANGE_DP modifiers should be registered when no opp Digimon"
        )


# ── Guard-clause tests (hand presence, 16-item checklist #15) ─────────


@pytest.mark.behavioral
class TestP221HandGuard:
    """Field-presence guards: the triggers must not fire from hand."""

    def test_all_triggered_conditions_fail_in_hand(self, debug_runner):
        """When the card is in hand, its [When Digivolving] / [When Attacking]
        conditions must evaluate to False (no permanent_of_this_card())."""
        runner = debug_runner(deck1=_make_deck(), deck2=_make_deck(), initial_memory=10)
        runner.inject_card(1, CHAOSMON, "hand")
        game = runner.game

        hand_card = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == CHAOSMON:
                hand_card = c
                break
        assert hand_card is not None, "Sanity: Chaosmon should be in hand"

        effects = hand_card.effect_list(None)
        triggered = [
            e for e in effects
            if getattr(e, 'is_when_digivolving', False)
            or getattr(e, 'is_on_attack', False)
        ]
        for eff in triggered:
            if eff.can_use_condition is not None:
                assert not eff.can_use_condition({}), (
                    f"Triggered effect {eff.effect_name!r} must return False "
                    "from can_use_condition when host card is in hand"
                )
