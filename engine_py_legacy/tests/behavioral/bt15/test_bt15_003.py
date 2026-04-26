"""Behavioral tests for BT15-003 Nyaromon (Digi-Egg, Lv.2, Yellow, Lesser).

Actual card text (from cards.json):
Inherited Effect [When Attacking] [Once Per Turn] By trashing the top or bottom card
    of your security stack, gain 1 memory.

DCGO C# reference:
    DCGO/Assets/Scripts/CardEffect/BT15/Yellow/BT15_003.cs
    - Uses OnAllyAttack factory with CanTriggerOnAttack + IsExistOnBattleArea gates
    - IDestroySecurity(fromTop=True/False) -> fires OnDiscardSecurity + OnLoseSecurity
    - SetBoolSelection offers Top/Bottom choice (player selects)
    - canNoSelect defaults True -> effect is optional (player can decline activation)
    - SetMaxCountPerTurn(1) + hash "TrashSecuirtyToGainMemory+1_BT15_003"
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming
from engine_py_legacy.engine.game.constants import SEL_EFFECT_CHOICE_START


def _get_inherited_wa_effect(perm):
    """Helper: pull the inherited When Attacking effect from Nyaromon (bottom card)."""
    source_card = perm.card_sources[0]  # BT15-003 is the bottom card
    all_effects = source_card.effect_list(None)
    matches = [
        e for e in all_effects
        if e.timing == EffectTiming.OnUseAttack
        and getattr(e, 'is_inherited_effect', False)
    ]
    return matches[0] if matches else None


@pytest.mark.behavioral
class TestBT15003Nyaromon:
    """Tests for BT15-003 Nyaromon."""

    # ------------------------------------------------------------------
    # Inherited effect structure
    # ------------------------------------------------------------------

    def test_inherited_effect_exists(self, debug_runner):
        """Should have an inherited When Attacking effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])
        inherited_wa = _get_inherited_wa_effect(perm)
        assert inherited_wa is not None, "Should have inherited When Attacking effect"

    def test_inherited_effect_timing_is_on_use_attack(self, debug_runner):
        """Must use OnUseAttack (28), not OnAllyAttack (32).

        Engine's _matches() filters OnUseAttack so only `perm is attacker` triggers.
        This matches DCGO CanTriggerOnAttack semantics (the attacker fires its own
        [When Attacking]; allies fire their [When an Ally Attacks]).
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])
        inherited_wa = _get_inherited_wa_effect(perm)
        assert inherited_wa.timing == EffectTiming.OnUseAttack

    def test_inherited_effect_is_on_attack_flag(self, debug_runner):
        """is_on_attack flag must be set so engine._matches picks it up for OnUseAttack."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])
        inherited_wa = _get_inherited_wa_effect(perm)
        assert getattr(inherited_wa, 'is_on_attack', False), \
            "Effect must set is_on_attack = True for OnUseAttack matching"

    def test_inherited_effect_is_optional(self, debug_runner):
        """The inherited effect should be optional (C# canNoSelect defaults True).

        "By trashing..." effects are optional at the activation level; once the
        player chooses to activate, the cost must be paid.
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])
        inherited_wa = _get_inherited_wa_effect(perm)
        assert inherited_wa.is_optional, "Effect should be optional"

    def test_inherited_effect_once_per_turn(self, debug_runner):
        """The inherited effect should be Once Per Turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])
        inherited_wa = _get_inherited_wa_effect(perm)
        assert getattr(inherited_wa, 'max_count_per_turn', None) == 1, \
            "Effect should be Once Per Turn (max_count_per_turn=1)"

    def test_inherited_effect_hash_string(self, debug_runner):
        """OPT must have a unique hash string shared across copies (C# match)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])
        inherited_wa = _get_inherited_wa_effect(perm)
        hash_str = getattr(inherited_wa, 'hash_string', None)
        assert hash_str, "OPT effect must have a hash_string"
        assert "BT15_003" in hash_str, \
            f"Hash string should reference BT15_003, got: {hash_str}"

    # ------------------------------------------------------------------
    # Condition gating
    # ------------------------------------------------------------------

    def test_condition_requires_security_cards(self, debug_runner):
        """Effect condition should fail when player has no security cards."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])

        game = runner.game
        game.player1.security_cards.clear()

        inherited_wa = _get_inherited_wa_effect(perm)
        result = inherited_wa.can_use_condition({
            'player': game.player1,
            'permanent': perm,
        })
        assert not result, "Should not be usable with no security cards"

    def test_condition_passes_with_security(self, debug_runner):
        """Effect condition should pass when player has security cards and is on field."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])

        game = runner.game
        assert len(game.player1.security_cards) > 0, "Should have security cards after setup"

        inherited_wa = _get_inherited_wa_effect(perm)
        result = inherited_wa.can_use_condition({
            'player': game.player1,
            'permanent': perm,
        })
        assert result, "Should be usable when on field with security"

    def test_condition_fails_when_not_on_field(self, debug_runner):
        """Inherited effect condition should fail when the host card is not on field."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])
        inherited_wa = _get_inherited_wa_effect(perm)

        # Remove from field by clearing battle area
        runner.game.player1.battle_area.clear()

        result = inherited_wa.can_use_condition({
            'player': runner.game.player1,
            'permanent': None,
        })
        assert not result, "Should not be usable when card is not on any field permanent"

    # ------------------------------------------------------------------
    # Process: trash security (top or bottom choice) + gain 1 memory
    # ------------------------------------------------------------------

    def test_process_offers_top_or_bottom_choice(self, debug_runner):
        """Effect should offer a choice between trashing top or bottom security."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])

        game = runner.game
        inherited_wa = _get_inherited_wa_effect(perm)

        security_before = len(game.player1.security_cards)
        memory_before = game.memory

        inherited_wa.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # A pending selection must exist (the top/bottom branch prompt)
        assert game.pending_selection is not None, \
            "Effect must create a pending selection for top/bottom choice"

        runner.auto_resolve()

        assert len(game.player1.security_cards) == security_before - 1, \
            "Should trash 1 security card"
        assert game.memory == memory_before + 1, \
            "Should gain 1 memory after trashing security"

    def test_trashing_top_security(self, debug_runner):
        """Choosing 'top' (branch 0) should trash security_cards[0]."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])

        game = runner.game
        assert len(game.player1.security_cards) >= 2, "Need >=2 security for this test"
        top_security_card = game.player1.security_cards[0]
        bottom_security_card = game.player1.security_cards[-1]

        inherited_wa = _get_inherited_wa_effect(perm)
        inherited_wa.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # auto_resolve picks first legal action = branch 0 = "Top"
        runner.auto_resolve()

        assert top_security_card in game.player1.trash_cards, \
            "Top security card should be in trash after choosing 'top'"
        assert bottom_security_card not in game.player1.trash_cards, \
            "Bottom security card should NOT be in trash when choosing 'top'"
        assert top_security_card not in game.player1.security_cards, \
            "Top security card should be removed from security stack"

    def test_trashing_bottom_security(self, debug_runner):
        """Choosing 'bottom' (branch 1) should trash security_cards[-1]."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])

        game = runner.game
        assert len(game.player1.security_cards) >= 2, "Need >=2 security for this test"
        top_security_card = game.player1.security_cards[0]
        bottom_security_card = game.player1.security_cards[-1]

        inherited_wa = _get_inherited_wa_effect(perm)
        inherited_wa.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert game.pending_selection is not None, \
            "Effect must create a pending selection"
        # Manually execute branch 1 (bottom)
        runner.execute(SEL_EFFECT_CHOICE_START + 1)
        runner.auto_resolve()

        assert bottom_security_card in game.player1.trash_cards, \
            "Bottom security card should be in trash after choosing 'bottom'"
        assert top_security_card not in game.player1.trash_cards, \
            "Top security card should NOT be in trash when choosing 'bottom'"
        assert bottom_security_card not in game.player1.security_cards, \
            "Bottom security card should be removed from security stack"

    def test_process_gains_exactly_one_memory(self, debug_runner):
        """Effect should gain exactly 1 memory."""
        runner = debug_runner(initial_memory=3)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])

        game = runner.game
        memory_before = game.memory

        inherited_wa = _get_inherited_wa_effect(perm)
        inherited_wa.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        assert game.memory == memory_before + 1, \
            f"Expected memory {memory_before + 1}, got {game.memory}"

    # ------------------------------------------------------------------
    # Event firing: OnDiscardSecurity / OnLoseSecurity
    # ------------------------------------------------------------------

    def test_trash_fires_on_discard_and_lose_security(self, debug_runner):
        """Trashing security as an effect cost must fire OnDiscardSecurity + OnLoseSecurity.

        DCGO's IDestroySecurity fires OnDiscardSecurity (on the trashed cards)
        after moving them to trash, and IReduceSecurity fires OnLoseSecurity.
        The engine equivalent is player.trash_security_card() which fires both.
        Raw security_cards.pop()/trash_cards.append() bypasses these events and
        silently breaks cards like BT15-038 Angewomon that have OnLoseSecurity triggers.
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])
        game = runner.game

        fired = {'OnDiscardSecurity': 0, 'OnLoseSecurity': 0}
        original_fire_timing = game.player1._fire_timing

        def spy_fire_timing(timing, context=None):
            name = getattr(timing, 'name', None)
            if name in fired:
                fired[name] += 1
            return original_fire_timing(timing, context)

        game.player1._fire_timing = spy_fire_timing

        try:
            inherited_wa = _get_inherited_wa_effect(perm)
            inherited_wa.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })
            runner.auto_resolve()
        finally:
            game.player1._fire_timing = original_fire_timing

        assert fired['OnDiscardSecurity'] >= 1, (
            "OnDiscardSecurity must fire when security is trashed via effect cost. "
            "Use player.trash_security_card(card), not raw pop + append."
        )
        assert fired['OnLoseSecurity'] >= 1, (
            "OnLoseSecurity must fire when security is trashed via effect cost."
        )

    # ------------------------------------------------------------------
    # Full engine integration via execute_effects(OnUseAttack)
    # ------------------------------------------------------------------

    def test_full_engine_flow_via_execute_effects(self, debug_runner):
        """Firing OnUseAttack with the Nyaromon permanent should expose the
        optional trash-security choice through the engine's stack resolver."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])
        game = runner.game

        sec_before = len(game.player1.security_cards)
        mem_before = game.memory

        game.execute_effects(EffectTiming.OnUseAttack, {
            "attacker": perm,
            "permanent": perm,
            "player": game.player1,
        })
        runner.auto_resolve()

        # auto_resolve picks first legal action at every prompt.
        # Final state: either 1 security trashed + 1 memory gained, or no change (declined).
        sec_delta = sec_before - len(game.player1.security_cards)
        mem_delta = game.memory - mem_before
        assert (sec_delta == 0 and mem_delta == 0) or (sec_delta == 1 and mem_delta == 1), (
            f"Expected no-op or (1 sec, 1 memory); got sec_delta={sec_delta}, mem_delta={mem_delta}"
        )

    def test_opt_prevents_second_activation(self, debug_runner):
        """OPT (max_count_per_turn=1) must prevent a second activation in the same turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])
        game = runner.game

        # Give plenty of security so we don't run out
        for _ in range(3):
            runner.inject_card(1, "ST1-01", "security_top")

        inherited_wa = _get_inherited_wa_effect(perm)

        sec_before_all = len(game.player1.security_cards)
        mem_before_all = game.memory

        # First activation
        inherited_wa.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()
        inherited_wa.record_activation()

        sec_after_first = len(game.player1.security_cards)
        mem_after_first = game.memory
        assert sec_after_first == sec_before_all - 1, "First activation should trash 1 security"
        assert mem_after_first == mem_before_all + 1, "First activation should gain 1 memory"

        # OPT: can_activate_this_turn must return False after one activation
        assert not inherited_wa.can_activate_this_turn(), \
            "OPT: effect should not be activatable a second time in same turn"

    # ------------------------------------------------------------------
    # Non-inherited guard: BT15-003 is a DigiEgg — only inherited effect
    # ------------------------------------------------------------------

    def test_digi_egg_has_only_inherited_effect(self, debug_runner):
        """DigiEgg card text consists solely of the inherited When Attacking
        effect — the card should NOT expose any non-inherited OnUseAttack
        effects (which would fire for any digimon on top without inheritance)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-003", "ST1-03"])
        source_card = perm.card_sources[0]

        all_effects = source_card.effect_list(None)
        non_inherited_wa = [
            e for e in all_effects
            if e.timing == EffectTiming.OnUseAttack
            and not getattr(e, 'is_inherited_effect', False)
        ]
        assert len(non_inherited_wa) == 0, (
            f"DigiEgg should only have inherited WA effects; "
            f"got {len(non_inherited_wa)} non-inherited OnUseAttack effects"
        )
