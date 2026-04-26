"""Behavioral tests for ST10-02 Salamon (Lv.3 Yellow Mammal, DP 2000, Cost 3).

Card text:
    (No main effects.)

    Inherited Effect [End of Your Turn] This Digimon and any of your other
    Digimon may DNA digivolve into a Digimon card in the hand.

Clause analysis:
    There is exactly ONE inherited clause — an OnEndTurn trigger that opens
    a DNA-digivolve-from-hand selection. The effect is optional, requires
    the host permanent on field, requires the owner's turn, requires at
    least one OTHER Digimon on field (needed as second DNA material), and
    requires at least one Digimon card in hand (as DNA target).

Key cards used:
    - ST10-02 Salamon — card under test, lives below top of stack
    - ST10-04 Gatomon — Lv.4 Yellow, used as top card so Salamon is inherited
    - ST10-05 Angewomon — Lv.5 Yellow Digimon (DNA material candidate)
    - ST10-11 Bastemon — Lv.5 Purple Digimon (DNA material candidate)
    - ST10-06 Mastemon — Lv.6 Yellow+Purple, DNA target
        (DNA cost: Yellow Lv.5 + Purple Lv.5, memory 0)
"""

import pytest

from engine_py_legacy.engine.data.enums import EffectTiming


SALAMON = "ST10-02"
GATOMON = "ST10-04"
ANGEWOMON = "ST10-05"
MASTEMON = "ST10-06"
BASTEMON = "ST10-11"
LADYDEVIMON = "ST10-12"


def _get_inherited_eot_effect(card):
    """Return the inherited OnEndTurn effect from a CardSource, or None."""
    effects = card.effect_list(None)
    eot = [
        e for e in effects
        if e.timing == EffectTiming.OnEndTurn and e.is_inherited_effect
    ]
    return eot[0] if eot else None


@pytest.mark.behavioral
class TestST10_02_InheritedStructure:
    """Shape of the single inherited [End of Your Turn] DNA digivolve effect."""

    def test_inherited_eot_effect_exists(self, debug_runner):
        """Salamon should expose exactly 1 inherited OnEndTurn effect."""
        runner = debug_runner(initial_memory=5)
        # Place Salamon under Gatomon so it is in the digi-stack
        perm = runner.place_on_field(1, [SALAMON, GATOMON])

        salamon = perm.card_sources[0]
        assert salamon.card_id == SALAMON, "Sanity: bottom card is Salamon"

        effects = salamon.effect_list(None)
        eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn and e.is_inherited_effect
        ]
        assert len(eot) == 1, (
            f"Should have exactly 1 inherited OnEndTurn effect, got {len(eot)}"
        )

    def test_effect_flags_inherited_and_optional(self, debug_runner):
        """The effect must be marked inherited and optional (card text says 'may')."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON, GATOMON])
        salamon = perm.card_sources[0]

        eot = _get_inherited_eot_effect(salamon)
        assert eot is not None
        assert eot.is_inherited_effect, "Must be marked is_inherited_effect"
        assert eot.is_optional, "DNA digivolve is optional (card says 'may')"

    def test_effect_is_not_also_on_the_top_card_non_inherited(self, debug_runner):
        """No non-inherited OnEndTurn should exist on Salamon (card has no main text)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        salamon = perm.top_card

        effects = salamon.effect_list(None)
        non_inh_eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn and not e.is_inherited_effect
        ]
        assert len(non_inh_eot) == 0, (
            "Salamon has NO main-level effects; OnEndTurn should only be inherited"
        )


@pytest.mark.behavioral
class TestST10_02_InheritedCondition:
    """Condition gates on the inherited OnEndTurn DNA digivolve."""

    def test_condition_passes_with_full_setup(self, debug_runner):
        """All gates satisfied: on field, owner's turn, other Digimon, hand Digimon."""
        runner = debug_runner(initial_memory=5)
        # Salamon in the digi-stack under Gatomon
        perm = runner.place_on_field(1, [SALAMON, GATOMON])
        # Another Yellow Digimon on field as second DNA material
        runner.place_on_field(1, [ANGEWOMON])
        # Purple Lv.5 (Bastemon) so we can DNA into Mastemon
        runner.place_on_field(1, [BASTEMON])
        # Put Mastemon (DNA target) in hand
        runner.inject_card(1, MASTEMON, "hand")

        salamon = perm.card_sources[0]
        eot = _get_inherited_eot_effect(salamon)
        assert eot is not None

        assert runner.game.player1.is_my_turn, "Sanity: P1 is turn player"
        assert eot.can_use_condition({}), (
            "Condition should pass with field+other Digimon+hand Digimon+own turn"
        )

    def test_condition_fails_on_opponent_turn(self, debug_runner):
        """Effect only activates on the OWNER's end of turn."""
        runner = debug_runner(initial_memory=5, first_player=2)
        perm = runner.place_on_field(1, [SALAMON, GATOMON])
        runner.place_on_field(1, [ANGEWOMON])
        runner.place_on_field(1, [BASTEMON])
        runner.inject_card(1, MASTEMON, "hand")

        salamon = perm.card_sources[0]
        eot = _get_inherited_eot_effect(salamon)
        assert eot is not None

        assert not runner.game.player1.is_my_turn, "Sanity: P1 NOT turn player"
        assert not eot.can_use_condition({}), (
            "Condition should fail on opponent's turn"
        )

    def test_condition_fails_when_card_not_on_field(self, debug_runner):
        """Inherited condition must require the host card to be on the field."""
        runner = debug_runner(initial_memory=5)
        # Salamon only in hand
        runner.inject_card(1, SALAMON, "hand")

        game = runner.game
        salamon_in_hand = [
            c for c in game.player1.hand_cards if c.card_id == SALAMON
        ][0]

        eot = _get_inherited_eot_effect(salamon_in_hand)
        assert eot is not None, "Effect should still be emitted from the script"

        assert not eot.can_use_condition({}), (
            "Condition must fail when Salamon is not on the field"
        )

    def test_condition_fails_without_other_digimon_on_field(self, debug_runner):
        """'and any of your other Digimon' — DNA needs a SECOND field material."""
        runner = debug_runner(initial_memory=5)
        # ONLY the Salamon-stack permanent, nothing else on the field
        perm = runner.place_on_field(1, [SALAMON, GATOMON])
        runner.inject_card(1, MASTEMON, "hand")

        salamon = perm.card_sources[0]
        eot = _get_inherited_eot_effect(salamon)
        assert eot is not None

        # There is only 1 Digimon on the field — DNA impossible
        assert len(runner.game.player1.battle_area) == 1
        assert not eot.can_use_condition({}), (
            "Condition should fail when there is no OTHER Digimon on field"
        )

    def test_condition_fails_with_empty_hand(self, debug_runner):
        """No Digimon card in hand means nothing to DNA-digivolve INTO."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON, GATOMON])
        runner.place_on_field(1, [ANGEWOMON])
        runner.place_on_field(1, [BASTEMON])
        runner.clear_zone(1, "hand")

        salamon = perm.card_sources[0]
        eot = _get_inherited_eot_effect(salamon)
        assert eot is not None

        assert not eot.can_use_condition({}), (
            "Condition should fail with no Digimon cards in hand"
        )


@pytest.mark.behavioral
class TestST10_02_InheritedProcess:
    """Process callback invokes DNA digivolve from hand."""

    def test_process_triggers_dna_digivolve_from_hand(self, debug_runner):
        """Process should call game.effect_dna_digivolve_from_hand for the owner."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON, GATOMON])
        runner.place_on_field(1, [ANGEWOMON])  # Yellow Lv.5
        runner.place_on_field(1, [BASTEMON])   # Purple Lv.5
        runner.inject_card(1, MASTEMON, "hand")  # DNA target in hand

        game = runner.game

        # Spy: wrap game.effect_dna_digivolve_from_hand
        calls = {"count": 0, "player": None, "is_optional": None}
        original = game.effect_dna_digivolve_from_hand

        def spy(player, filter_fn, is_optional=True, prompt=""):
            calls["count"] += 1
            calls["player"] = player
            calls["is_optional"] = is_optional
            return original(player, filter_fn, is_optional=is_optional, prompt=prompt)

        game.effect_dna_digivolve_from_hand = spy  # type: ignore[assignment]

        salamon = perm.card_sources[0]
        eot = _get_inherited_eot_effect(salamon)
        assert eot is not None

        eot.on_process_callback({
            'player': game.player1,
            'game': game,
            'card': salamon,
            'permanent': perm,
        })

        assert calls["count"] == 1, (
            f"effect_dna_digivolve_from_hand should be called once, got {calls['count']}"
        )
        assert calls["player"] is game.player1, "Must be called for the owner"
        assert calls["is_optional"] is True, "Must be optional ('may')"

    def test_process_end_to_end_resolves_into_mastemon(self, debug_runner):
        """End-to-end: after process + auto_resolve, Mastemon should hit the field."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON, GATOMON])
        runner.place_on_field(1, [ANGEWOMON])
        runner.place_on_field(1, [BASTEMON])
        # Clear hand, then put ONLY Mastemon in hand so auto-resolve has no
        # ambiguity on target selection.
        runner.clear_zone(1, "hand")
        runner.inject_card(1, MASTEMON, "hand")

        game = runner.game
        salamon = perm.card_sources[0]
        eot = _get_inherited_eot_effect(salamon)
        assert eot is not None
        assert eot.can_use_condition({}), "Precondition: effect can activate"

        eot.on_process_callback({
            'player': game.player1,
            'game': game,
            'card': salamon,
            'permanent': perm,
        })
        runner.auto_resolve()

        # Mastemon should now be a top card on the field
        field_top_ids = [
            p.top_card.card_id for p in game.player1.battle_area if p.top_card
        ]
        assert MASTEMON in field_top_ids, (
            f"After DNA digivolve, Mastemon should be on field. "
            f"Top cards: {field_top_ids}"
        )
        # Mastemon should NOT still be in hand
        hand_ids = [c.card_id for c in game.player1.hand_cards]
        assert MASTEMON not in hand_ids, (
            f"Mastemon should have left the hand. Hand: {hand_ids}"
        )

    def test_process_noop_when_no_valid_target(self, debug_runner):
        """When nothing in hand qualifies, DNA call is a safe no-op."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON, GATOMON])
        runner.place_on_field(1, [ANGEWOMON])
        runner.place_on_field(1, [BASTEMON])
        runner.clear_zone(1, "hand")
        # Put only a non-DNA card in hand (regular Digimon without dna_costs)
        runner.inject_card(1, ANGEWOMON, "hand")

        game = runner.game

        field_before = [p.top_card.card_id for p in game.player1.battle_area if p.top_card]

        salamon = perm.card_sources[0]
        eot = _get_inherited_eot_effect(salamon)
        assert eot is not None

        # Safe to call even when there are no candidates
        eot.on_process_callback({
            'player': game.player1,
            'game': game,
            'card': salamon,
            'permanent': perm,
        })
        runner.auto_resolve()

        field_after = [p.top_card.card_id for p in game.player1.battle_area if p.top_card]
        assert field_after == field_before, (
            f"Field should be unchanged when no DNA target. "
            f"Before: {field_before}, After: {field_after}"
        )
