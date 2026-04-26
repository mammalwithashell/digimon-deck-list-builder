"""Behavioral tests for BT13-034 Kudamon (Digimon, Yellow, Lv.3, Holy Beast, Vaccine).

Card text:
    [On Play] Reveal the top 3 cards of your deck. Add 1 yellow Digimon card with
        the [Vaccine] trait and 1 yellow Tamer card among them to your hand.
        Place the rest at the bottom of your deck in any order.
    Inherited: [When Attacking] [Once Per Turn] If the total cards in both
        players' security stacks is 6 or less, 1 of your opponent's Digimon
        gets -2000 DP for the turn.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming
from digimon_gym.engine.interfaces.modifiers import ModifierType


# Test data: yellow Vaccine Digimon and yellow Tamers known to exist.
YELLOW_VACCINE_DIGIMON = "BT1-049"   # Labramon (Yellow / Lv.3 / Vaccine)
YELLOW_TAMER = "BT1-087"             # T.K. Takaishi (Yellow Tamer)
YELLOW_NON_VACCINE_DIGIMON = "BT1-045"  # Tsukaimon (Yellow / Virus)
NON_YELLOW_VACCINE_DIGIMON = "AD1-001"  # Greymon (Red / Vaccine)
VANILLA_DIGIMON = "ST1-03"           # Agumon (Red rookie)


@pytest.mark.behavioral
class TestBT13034Kudamon:
    """Tests for BT13-034 Kudamon (Lv.3 Yellow Vaccine)."""

    # ------------------------------------------------------------------
    # Effect discovery / shape
    # ------------------------------------------------------------------

    def test_has_on_play_effect(self, debug_runner):
        """Should expose an [On Play] effect at OnEnterFieldAnyone timing."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT13-034"])
        top = perm.top_card
        effects = top.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        assert len(on_play) == 1, "Should have exactly 1 [On Play] effect"

    def test_has_when_attacking_inherited_effect(self, debug_runner):
        """Inherited [When Attacking] effect must be OnUseAttack + is_on_attack + is_inherited."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT13-034"])
        top = perm.top_card
        effects = top.effect_list(None)
        wa = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_on_attack', False)
        ]
        assert len(wa) == 1, "Should have exactly 1 [When Attacking] effect"
        assert wa[0].is_inherited_effect is True, "Must be an inherited effect"
        assert wa[0].timing.value == 28, "EffectTiming.OnUseAttack must equal 28"

    def test_when_attacking_is_once_per_turn(self, debug_runner):
        """The inherited [When Attacking] effect must have OPT (max_count=1 + hash)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT13-034"])
        top = perm.top_card
        effects = top.effect_list(None)
        wa = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_on_attack', False)
        ][0]
        assert getattr(wa, 'max_count_per_turn', None) == 1, "Must be Once Per Turn"
        assert getattr(wa, 'hash_string', None), "Must have a hash string for OPT tracking"

    # ------------------------------------------------------------------
    # C1 [On Play] reveal 3 + yellow Vaccine + yellow Tamer
    # ------------------------------------------------------------------

    def _get_on_play(self, perm):
        top = perm.top_card
        effects = top.effect_list(None)
        return [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

    def test_on_play_adds_yellow_vaccine_and_yellow_tamer_to_hand(self, debug_runner):
        """If top-3 revealed contains both a yellow Vaccine Digimon and a yellow
        Tamer, both must end up in hand and any remainder at deck bottom."""
        runner = debug_runner(initial_memory=5)
        # Clear default library and set up top-3 deterministically
        runner.clear_zone(1, "library")
        # Library top-3 (top is index 0): yellow vaccine digi, yellow tamer, vanilla red
        runner.inject_card(1, YELLOW_VACCINE_DIGIMON, "library_top")   # appears at idx 2
        runner.inject_card(1, YELLOW_TAMER, "library_top")             # appears at idx 1
        runner.inject_card(1, VANILLA_DIGIMON, "library_top")          # appears at idx 0
        # Reverse via re-inject: insert order means last inserted is at index 0.
        # Actually inject_card "library_top" inserts at position 0, so last call is top.
        # That places VANILLA at index 0, YELLOW_TAMER at 1, YELLOW_VACCINE at 2.

        perm = runner.place_on_field(1, ["BT13-034"])
        game = runner.game
        hand_before = list(game.player1.hand_cards)
        lib_size_before = len(game.player1.library_cards)

        on_play = self._get_on_play(perm)
        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # Both a yellow vaccine digi and a yellow tamer should now be in hand
        hand_ids = [c.c_entity_base.card_id for c in game.player1.hand_cards]
        assert YELLOW_VACCINE_DIGIMON in hand_ids, \
            f"Yellow Vaccine Digimon {YELLOW_VACCINE_DIGIMON} should be added to hand"
        assert YELLOW_TAMER in hand_ids, \
            f"Yellow Tamer {YELLOW_TAMER} should be added to hand"

        # The remaining vanilla digimon should NOT be in hand
        assert VANILLA_DIGIMON not in hand_ids, \
            f"Non-matching {VANILLA_DIGIMON} should not enter hand"

        # Library size dropped by 2 (2 cards moved to hand; 1 remaining back to deck bottom)
        assert len(game.player1.library_cards) == lib_size_before - 2, \
            "Exactly 2 cards should leave the library net (2 to hand, 1 back to bottom)"

    def test_on_play_non_vaccine_yellow_digimon_ignored(self, debug_runner):
        """A yellow Digimon WITHOUT the Vaccine attribute must NOT be addable
        as the Vaccine pass target (the attribute check must be attribute_eng,
        not type_eng/card_traits)."""
        runner = debug_runner(initial_memory=5)
        runner.clear_zone(1, "library")
        # Top-3: yellow NON-vaccine digi, vanilla, vanilla
        runner.inject_card(1, VANILLA_DIGIMON, "library_top")
        runner.inject_card(1, VANILLA_DIGIMON, "library_top")
        runner.inject_card(1, YELLOW_NON_VACCINE_DIGIMON, "library_top")
        # After these 3 inserts, index 0 = YELLOW_NON_VACCINE, 1 = VANILLA, 2 = VANILLA

        perm = runner.place_on_field(1, ["BT13-034"])
        game = runner.game
        lib_size_before = len(game.player1.library_cards)

        on_play = self._get_on_play(perm)
        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        hand_ids = [c.c_entity_base.card_id for c in game.player1.hand_cards]
        assert YELLOW_NON_VACCINE_DIGIMON not in hand_ids, \
            "Yellow but NON-vaccine digimon must not match the [Vaccine] trait filter"
        # All three revealed cards should have been returned to deck bottom
        assert len(game.player1.library_cards) == lib_size_before, \
            "No cards matched either pass — all 3 revealed should return to bottom of deck"

    def test_on_play_non_yellow_vaccine_digimon_ignored(self, debug_runner):
        """A non-yellow Vaccine Digimon must NOT be addable — must be yellow."""
        runner = debug_runner(initial_memory=5)
        runner.clear_zone(1, "library")
        runner.inject_card(1, VANILLA_DIGIMON, "library_top")
        runner.inject_card(1, VANILLA_DIGIMON, "library_top")
        runner.inject_card(1, NON_YELLOW_VACCINE_DIGIMON, "library_top")

        perm = runner.place_on_field(1, ["BT13-034"])
        game = runner.game

        on_play = self._get_on_play(perm)
        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        hand_ids = [c.c_entity_base.card_id for c in game.player1.hand_cards]
        assert NON_YELLOW_VACCINE_DIGIMON not in hand_ids, \
            "Non-yellow Vaccine Digimon must NOT satisfy the color+trait filter"

    # ------------------------------------------------------------------
    # C2 Inherited [When Attacking][OPT] condition & DP -2000
    # ------------------------------------------------------------------

    def _get_when_attacking(self, perm):
        top = perm.top_card
        effects = top.effect_list(None)
        return [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_on_attack', False)
        ][0]

    def test_when_attacking_condition_requires_field_presence(self, debug_runner):
        """Condition must short-circuit if the source card is not on the field."""
        runner = debug_runner(initial_memory=5)
        # Don't place on field — inject in hand instead
        runner.inject_card(1, "BT13-034", "hand")
        game = runner.game

        kudamon = next(
            c for c in game.player1.hand_cards
            if c.c_entity_base and c.c_entity_base.card_id == "BT13-034"
        )
        effects = kudamon.effect_list(None)
        wa = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_on_attack', False)
        ][0]
        result = wa.can_use_condition({'player': game.player1, 'game': game})
        assert result is False, "Condition must fail when Kudamon is not on the field"

    def test_when_attacking_condition_security_gate(self, debug_runner):
        """Condition must check combined security stack size (<=6 passes, >6 fails)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT13-034"])
        game = runner.game

        # Clear both security stacks to deterministic counts
        runner.clear_zone(1, "security")
        runner.clear_zone(2, "security")
        for _ in range(3):
            runner.inject_card(1, VANILLA_DIGIMON, "security_top")
            runner.inject_card(2, VANILLA_DIGIMON, "security_top")
        # Total = 6
        wa = self._get_when_attacking(perm)
        ok = wa.can_use_condition({
            'player': game.player1, 'game': game,
            'permanent': perm, 'attacker': perm,
        })
        assert ok, "Condition must pass when total security == 6"

        # Bump to 7 total
        runner.inject_card(1, VANILLA_DIGIMON, "security_top")
        not_ok = wa.can_use_condition({
            'player': game.player1, 'game': game,
            'permanent': perm, 'attacker': perm,
        })
        assert not_ok is False, "Condition must fail when total security > 6"

    def test_when_attacking_applies_dp_minus_2000_to_selected_target(self, debug_runner):
        """Process must reduce selected opponent Digimon's DP by 2000 (end of turn)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT13-034"])
        # Put an opponent Digimon on field (ST1-03 Agumon = 2000 DP rookie)
        opp = runner.place_on_field(2, ["ST1-03"])
        game = runner.game

        # Clear security to ensure condition passes
        runner.clear_zone(1, "security")
        runner.clear_zone(2, "security")

        dp_before = opp.dp
        assert dp_before is not None and dp_before > 0, "Opponent should have positive DP"

        wa = self._get_when_attacking(perm)
        wa.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacker': perm,
        })
        runner.auto_resolve()

        dp_after = opp.dp
        assert dp_after == dp_before - 2000, \
            f"Opponent DP should drop by 2000 (before={dp_before}, after={dp_after})"

    def test_when_attacking_dp_modifier_does_not_leak_to_other_permanents(self, debug_runner):
        """CRITICAL: the -2000 DP must ONLY apply to the selected target, not all perms."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT13-034"])
        target = runner.place_on_field(2, ["ST1-03"])
        bystander = runner.place_on_field(2, ["ST1-03"])
        game = runner.game

        runner.clear_zone(1, "security")
        runner.clear_zone(2, "security")

        bystander_dp_before = bystander.dp

        wa = self._get_when_attacking(perm)
        wa.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacker': perm,
        })
        runner.auto_resolve()

        # Bystander must be unaffected
        assert bystander.dp == bystander_dp_before, \
            f"Bystander DP must NOT change (before={bystander_dp_before}, after={bystander.dp}) — "\
            f"CHANGE_DP modifier must have target-equality guard"

    def test_when_attacking_dp_modifier_expires_end_of_turn(self, debug_runner):
        """The -2000 DP effect must expire at end of turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT13-034"])
        target = runner.place_on_field(2, ["ST1-03"])
        game = runner.game

        runner.clear_zone(1, "security")
        runner.clear_zone(2, "security")

        dp_before = target.dp

        wa = self._get_when_attacking(perm)
        wa.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacker': perm,
        })
        runner.auto_resolve()
        assert target.dp == dp_before - 2000, "DP reduced immediately"

        # Simulate end-of-turn modifier expiry
        game.modifiers.clear_expiry('end_of_turn')
        assert target.dp == dp_before, \
            "DP modifier should have expired after end of turn"

    def test_when_attacking_skips_own_permanents(self, debug_runner):
        """Only opponent permanents are valid targets."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT13-034"])
        own_other = runner.place_on_field(1, ["ST1-03"])  # own digimon
        game = runner.game

        runner.clear_zone(1, "security")
        runner.clear_zone(2, "security")

        own_dp_before = own_other.dp
        wa = self._get_when_attacking(perm)
        wa.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacker': perm,
        })
        runner.auto_resolve()
        assert own_other.dp == own_dp_before, \
            "Own Digimon must not be targetable by the opponent-select effect"
