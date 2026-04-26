"""Behavioral tests for BT22-034 Reppamon.

Card text:
    When effects trash this card from the security stack, you may play this
        card without paying the cost.
    [On Play] [When Digivolving] 1 of your opponent's Digimon gets -3000 DP
        until their turn ends. By trashing your top security card, instead 1
        of your opponent's Digimon gets -6000 DP until their turn ends.

Inherited:
    [When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -2000
        DP for the turn.

Alt digivolve: Lv.3 w/[CS] trait -> cost 2.
"""

from __future__ import annotations

import pytest

from engine_py_legacy.engine.data.enums import EffectTiming


REPPAMON = "BT22-034"
AGUMON_CS = "BT22-008"   # Lv.3 CS trait Digimon (base for alt-digi / WD tests)
COREDRAMON = "ST1-06"    # Lv.4 6000 DP — opponent DP target
AGUMON_BASIC = "ST1-03"  # Lv.3 2000 DP — generic filler
FILLER = "ST1-03"


def _get_effects(perm):
    """Collect all effects across the digi-stack."""
    effects = []
    for source in perm.card_sources:
        effects.extend(source.effect_list(EffectTiming.NoTiming))
    return effects


def _find_effect(perm, *, is_on_play=False, is_when_digivolving=False,
                 is_on_attack=False):
    for eff in _get_effects(perm):
        if is_on_play and getattr(eff, "is_on_play", False):
            return eff
        if is_when_digivolving and getattr(eff, "is_when_digivolving", False):
            return eff
        if is_on_attack and getattr(eff, "is_on_attack", False):
            return eff
    return None


def _find_on_discard_security_effect_from_zone(card_source):
    """Get the OnDiscardSecurity self-trash effect on a CardSource directly."""
    for eff in card_source.effect_list(EffectTiming.NoTiming):
        if getattr(eff, "timing", None) == EffectTiming.OnDiscardSecurity:
            return eff
    return None


def _find_inherited_when_attacking(card_source):
    for eff in card_source.effect_list(EffectTiming.NoTiming):
        if (
            getattr(eff, "is_inherited_effect", False)
            and getattr(eff, "is_on_attack", False)
        ):
            return eff
    return None


def _standard_decks(reppamon_count=4):
    deck1 = [REPPAMON] * reppamon_count + [FILLER] * (50 - reppamon_count)
    deck2 = [FILLER] * 50
    return deck1, deck2


@pytest.mark.behavioral
class TestBT22034Reppamon:
    """Tests for BT22-034 Reppamon."""

    # ── Alt-digivolve requirement ────────────────────────────────────

    def test_alt_digivolve_cost_and_level(self, debug_runner):
        """Alt-digi should be Lv.3 for cost 2 with CS trait requirement."""
        deck1, deck2 = _standard_decks()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=10)

        # Inject a Reppamon card source to inspect its effects
        rep_card = runner.inject_card(1, REPPAMON, "hand")
        effects = rep_card.effect_list(EffectTiming.NoTiming)
        alt_effects = [
            e for e in effects
            if getattr(e, "_alt_digi_level", None) is not None
        ]
        assert len(alt_effects) >= 1, "Should have at least one alt-digi effect"
        alt = alt_effects[0]
        assert alt._alt_digi_cost == 2
        assert alt._alt_digi_level == 3
        # CS trait required per card text
        assert getattr(alt, "_alt_digi_trait", None) == "CS", (
            f"alt-digi should require CS trait, got "
            f"{getattr(alt, '_alt_digi_trait', None)!r}"
        )

    # ── C1: OnDiscardSecurity self-trash → optional play free ───────

    def test_on_discard_security_effect_exists(self, debug_runner):
        """Reppamon should expose an OnDiscardSecurity effect."""
        deck1, deck2 = _standard_decks()
        runner = debug_runner(deck1=deck1, deck2=deck2)
        rep_card = runner.inject_card(1, REPPAMON, "hand")
        effect = _find_on_discard_security_effect_from_zone(rep_card)
        assert effect is not None, (
            "Reppamon should have an OnDiscardSecurity effect"
        )

    def test_on_discard_security_is_optional(self, debug_runner):
        """The 'may play' clause means the effect is optional."""
        deck1, deck2 = _standard_decks()
        runner = debug_runner(deck1=deck1, deck2=deck2)
        rep_card = runner.inject_card(1, REPPAMON, "hand")
        effect = _find_on_discard_security_effect_from_zone(rep_card)
        assert effect is not None
        assert effect.is_optional, (
            "OnDiscardSecurity free-play should be optional"
        )

    def test_on_discard_security_only_fires_for_self(self, debug_runner):
        """Condition must reject when a different card was discarded."""
        deck1, deck2 = _standard_decks()
        runner = debug_runner(deck1=deck1, deck2=deck2)
        rep_card = runner.inject_card(1, REPPAMON, "hand")
        effect = _find_on_discard_security_effect_from_zone(rep_card)
        assert effect is not None

        # Simulate a DIFFERENT card being discarded
        other = runner.inject_card(1, FILLER, "trash")
        ctx = {
            "game": runner.game,
            "player": runner.game.player1,
            "card": rep_card,
            "discarded_card": other,
        }
        assert effect.can_use_condition(ctx) is False, (
            "Condition should reject when a different card was discarded"
        )

        # Simulate THIS card being discarded (must also be in trash)
        runner.game.player1.trash_cards.append(rep_card)
        ctx_self = {
            "game": runner.game,
            "player": runner.game.player1,
            "card": rep_card,
            "discarded_card": rep_card,
        }
        assert effect.can_use_condition(ctx_self) is True, (
            "Condition should accept when THIS card was discarded"
        )

    def test_on_discard_security_plays_from_trash(self, debug_runner):
        """Processing should move the card from trash to battle area."""
        deck1, deck2 = _standard_decks()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=0)
        rep_card = runner.inject_card(1, REPPAMON, "trash")
        effect = _find_on_discard_security_effect_from_zone(rep_card)
        assert effect is not None

        ctx = {
            "game": runner.game,
            "player": runner.game.player1,
            "permanent": None,
            "card": rep_card,
            "discarded_card": rep_card,
        }
        # Confirm card is in trash and not on field
        assert rep_card in runner.game.player1.trash_cards
        effect.on_process_callback(ctx)

        # Effect should have played Reppamon from trash to battle area
        assert rep_card not in runner.game.player1.trash_cards, (
            "Reppamon should have been removed from trash"
        )
        field_ids = [
            p.top_card.card_id for p in runner.game.player1.battle_area
            if p.top_card
        ]
        assert REPPAMON in field_ids, (
            f"Reppamon should be on field, got {field_ids}"
        )

    def test_on_discard_security_does_not_deduct_memory(self, debug_runner):
        """Playing free must not subtract from memory."""
        deck1, deck2 = _standard_decks()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=3)
        rep_card = runner.inject_card(1, REPPAMON, "trash")
        effect = _find_on_discard_security_effect_from_zone(rep_card)
        assert effect is not None

        mem_before = runner.game.memory
        ctx = {
            "game": runner.game,
            "player": runner.game.player1,
            "permanent": None,
            "card": rep_card,
            "discarded_card": rep_card,
        }
        effect.on_process_callback(ctx)
        assert runner.game.memory == mem_before, (
            f"Playing for free must not spend memory (was {mem_before}, "
            f"now {runner.game.memory})"
        )

    def test_on_discard_security_fired_via_trash_security_card(
        self, debug_runner
    ):
        """Full flow: trashing Reppamon from security should offer to play it."""
        deck1, deck2 = _standard_decks()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=0)
        rep_card = runner.inject_card(1, REPPAMON, "security_top")

        # Trash Reppamon from security — engine fires OnDiscardSecurity
        runner.game.player1.trash_security_card(rep_card)
        runner.auto_resolve(max_steps=15)

        # Reppamon should now be on the field (auto-resolver will pick
        # the "Yes" branch)
        field_ids = [
            p.top_card.card_id for p in runner.game.player1.battle_area
            if p.top_card
        ]
        assert REPPAMON in field_ids, (
            f"Reppamon should be on field after OnDiscardSecurity play, "
            f"got {field_ids}"
        )

    # ── C2: [On Play] -3000 DP default / -6000 DP via trash top sec ──

    def test_on_play_effect_exists_and_flag(self, debug_runner):
        """Should have an is_on_play effect (not shared with WD instance)."""
        deck1, deck2 = _standard_decks()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=5)
        rep_perm = runner.place_on_field(1, [REPPAMON])

        on_play_effects = [
            e for e in _get_effects(rep_perm)
            if getattr(e, "is_on_play", False)
        ]
        when_digivolving_effects = [
            e for e in _get_effects(rep_perm)
            if getattr(e, "is_when_digivolving", False)
        ]
        assert len(on_play_effects) == 1, (
            f"Should have exactly 1 [On Play] effect, got "
            f"{len(on_play_effects)}"
        )
        assert len(when_digivolving_effects) == 1, (
            f"Should have exactly 1 [When Digivolving] effect, got "
            f"{len(when_digivolving_effects)}"
        )
        # They must be SEPARATE instances (not the same effect with both flags)
        assert on_play_effects[0] is not when_digivolving_effects[0], (
            "[On Play] and [When Digivolving] must be separate ICardEffect "
            "instances"
        )

    def test_on_play_default_branch_minus_3000_dp(self, debug_runner):
        """Default branch: -3000 DP on chosen opponent Digimon."""
        deck1, deck2 = _standard_decks()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=10)
        rep_perm = runner.place_on_field(1, [REPPAMON])
        opp = runner.place_on_field(2, [COREDRAMON])  # 6000 DP
        assert opp.dp == 6000

        # Clear p1 security so only "default" branch is available
        runner.game.player1.security_cards.clear()

        on_play = _find_effect(rep_perm, is_on_play=True)
        assert on_play is not None
        ctx = {
            "game": runner.game,
            "player": runner.game.player1,
            "permanent": rep_perm,
            "card": rep_perm.top_card,
        }
        on_play.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)

        assert opp.dp == 3000, (
            f"Default branch should apply -3000 DP (6000 -> 3000), "
            f"got {opp.dp}"
        )

    def test_on_play_branch_choice_exposed_when_security_available(
        self, debug_runner
    ):
        """When player has security cards, a branch selection must be exposed."""
        deck1, deck2 = _standard_decks()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=10)
        rep_perm = runner.place_on_field(1, [REPPAMON])
        runner.place_on_field(2, [COREDRAMON])

        # Ensure p1 has at least 1 security card to trash
        runner.inject_card(1, FILLER, "security_top")

        on_play = _find_effect(rep_perm, is_on_play=True)
        assert on_play is not None
        ctx = {
            "game": runner.game,
            "player": runner.game.player1,
            "permanent": rep_perm,
            "card": rep_perm.top_card,
        }
        on_play.on_process_callback(ctx)

        # Engine should now be in a SelectEffectChoice phase for the branch
        from engine_py_legacy.engine.data.enums import GamePhase
        assert runner.game.current_phase == GamePhase.SelectEffectChoice, (
            f"Branch choice should be exposed, got phase "
            f"{runner.game.current_phase}"
        )

    def test_on_play_trash_sec_branch_minus_6000_dp(self, debug_runner):
        """By-cost branch: trash top security, apply -6000 DP."""
        deck1, deck2 = _standard_decks()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=10)
        rep_perm = runner.place_on_field(1, [REPPAMON])
        opp = runner.place_on_field(2, [COREDRAMON])  # 6000 DP
        assert opp.dp == 6000

        # Give p1 a distinct top security card to verify it gets trashed
        top_sec = runner.inject_card(1, FILLER, "security_top")
        sec_len_before = len(runner.game.player1.security_cards)

        on_play = _find_effect(rep_perm, is_on_play=True)
        assert on_play is not None
        ctx = {
            "game": runner.game,
            "player": runner.game.player1,
            "permanent": rep_perm,
            "card": rep_perm.top_card,
        }
        on_play.on_process_callback(ctx)

        # Pick branch 1 (trash top sec, -6000 DP)
        from engine_py_legacy.engine.data.enums import GamePhase
        assert runner.game.current_phase == GamePhase.SelectEffectChoice

        # Constants for effect-choice action range
        from engine_py_legacy.engine.game.constants import SEL_EFFECT_CHOICE_START
        runner.execute(SEL_EFFECT_CHOICE_START + 1)
        runner.auto_resolve(max_steps=10)

        # Top security card should now be in trash
        assert top_sec in runner.game.player1.trash_cards, (
            "Top security card should have been trashed"
        )
        assert len(runner.game.player1.security_cards) == sec_len_before - 1
        # Opponent DP should be 0 (6000 - 6000, floor 0)
        assert opp.dp == 0, (
            f"-6000 DP branch should reduce Coredramon from 6000 to 0, "
            f"got {opp.dp}"
        )

    def test_on_play_dp_modifier_does_not_leak_to_other_opponents(
        self, debug_runner
    ):
        """DP modifier must only affect the SELECTED target (target-equality)."""
        deck1, deck2 = _standard_decks()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=10)
        rep_perm = runner.place_on_field(1, [REPPAMON])
        opp_a = runner.place_on_field(2, [COREDRAMON])  # 6000 DP
        opp_b = runner.place_on_field(2, [COREDRAMON])  # 6000 DP

        # Clear security to force default branch (no choice prompt)
        runner.game.player1.security_cards.clear()

        on_play = _find_effect(rep_perm, is_on_play=True)
        assert on_play is not None
        ctx = {
            "game": runner.game,
            "player": runner.game.player1,
            "permanent": rep_perm,
            "card": rep_perm.top_card,
        }
        on_play.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)

        # Exactly one of them should be hit
        hit = [p for p in (opp_a, opp_b) if p.dp == 3000]
        untouched = [p for p in (opp_a, opp_b) if p.dp == 6000]
        assert len(hit) == 1 and len(untouched) == 1, (
            f"Exactly one opponent should be at 3000 DP and the other at "
            f"6000 DP (target-equality guard). "
            f"Got {[p.dp for p in (opp_a, opp_b)]}"
        )

    def test_on_play_no_opponent_digimon_no_crash(self, debug_runner):
        """No opponent Digimon: effect resolves gracefully (branch still OK)."""
        deck1, deck2 = _standard_decks()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=10)
        rep_perm = runner.place_on_field(1, [REPPAMON])
        # No opponent field
        runner.game.player1.security_cards.clear()  # force default branch

        on_play = _find_effect(rep_perm, is_on_play=True)
        assert on_play is not None
        ctx = {
            "game": runner.game,
            "player": runner.game.player1,
            "permanent": rep_perm,
            "card": rep_perm.top_card,
        }
        # Should not crash
        on_play.on_process_callback(ctx)
        runner.auto_resolve(max_steps=5)

    # ── C3: [When Digivolving] same effect ───────────────────────────

    def test_when_digivolving_default_branch_minus_3000_dp(self, debug_runner):
        """WD default branch applies -3000 DP on selected opponent Digimon."""
        deck1, deck2 = _standard_decks()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=10)
        rep_perm = runner.place_on_field(1, [REPPAMON])
        opp = runner.place_on_field(2, [COREDRAMON])  # 6000 DP

        runner.game.player1.security_cards.clear()

        wd_effect = _find_effect(rep_perm, is_when_digivolving=True)
        assert wd_effect is not None
        ctx = {
            "game": runner.game,
            "player": runner.game.player1,
            "permanent": rep_perm,
            "card": rep_perm.top_card,
        }
        wd_effect.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)

        assert opp.dp == 3000, (
            f"WD default branch should apply -3000 DP (6000 -> 3000), "
            f"got {opp.dp}"
        )

    # ── C4: Inherited [When Attacking] OPT -2000 DP ──────────────────

    def test_inherited_when_attacking_effect_exists(self, debug_runner):
        """Inherited OnUseAttack effect should be present on the CardSource."""
        deck1, deck2 = _standard_decks()
        runner = debug_runner(deck1=deck1, deck2=deck2)
        rep_card = runner.inject_card(1, REPPAMON, "hand")
        inh = _find_inherited_when_attacking(rep_card)
        assert inh is not None, (
            "Reppamon should have an inherited [When Attacking] effect"
        )
        assert inh.is_inherited_effect is True
        assert inh.is_on_attack is True
        assert inh.max_count_per_turn == 1
        assert inh.hash_string == "BT22_034_WA"

    def test_inherited_attack_minus_2000_dp_on_target(self, debug_runner):
        """When Reppamon attacks, target gets -2000 DP for the turn."""
        deck1, deck2 = _standard_decks()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=10)
        rep_perm = runner.place_on_field(1, [REPPAMON])
        opp = runner.place_on_field(2, [COREDRAMON])  # 6000 DP
        assert opp.dp == 6000

        inh = _find_effect(rep_perm, is_on_attack=True)
        assert inh is not None

        ctx = {
            "game": runner.game,
            "player": runner.game.player1,
            "permanent": rep_perm,
            "card": rep_perm.top_card,
            "attacker": rep_perm,
        }
        inh.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)

        assert opp.dp == 4000, (
            f"Inherited -2000 DP should reduce 6000 to 4000, got {opp.dp}"
        )

    def test_inherited_attack_dp_does_not_leak(self, debug_runner):
        """Target-equality guard: only the selected target is affected."""
        deck1, deck2 = _standard_decks()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=10)
        rep_perm = runner.place_on_field(1, [REPPAMON])
        opp_a = runner.place_on_field(2, [COREDRAMON])
        opp_b = runner.place_on_field(2, [COREDRAMON])

        inh = _find_effect(rep_perm, is_on_attack=True)
        assert inh is not None
        ctx = {
            "game": runner.game,
            "player": runner.game.player1,
            "permanent": rep_perm,
            "card": rep_perm.top_card,
            "attacker": rep_perm,
        }
        inh.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)

        hit = [p for p in (opp_a, opp_b) if p.dp == 4000]
        untouched = [p for p in (opp_a, opp_b) if p.dp == 6000]
        assert len(hit) == 1 and len(untouched) == 1, (
            f"Exactly one opponent should be -2000 DP; got "
            f"{[p.dp for p in (opp_a, opp_b)]}"
        )

    def test_inherited_opt_blocks_second_activation(self, debug_runner):
        """[Once Per Turn] should block repeated activations in the same turn."""
        deck1, deck2 = _standard_decks()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=10)
        rep_perm = runner.place_on_field(1, [REPPAMON])

        inh = _find_effect(rep_perm, is_on_attack=True)
        assert inh is not None
        assert inh.can_activate_this_turn()
        inh.record_activation()
        assert not inh.can_activate_this_turn(), (
            "OPT should block second activation within the turn"
        )

    # ── Verify no `is_on_deletion` or other stray flags ──────────────

    def test_no_stubbed_security_pop(self, debug_runner):
        """The script must use trash_security_card, not raw list pop."""
        import inspect
        from engine_py_legacy.engine.data.scripts.bt22 import bt22_034
        src = inspect.getsource(bt22_034)
        # No direct security list mutation should remain
        assert "security_cards.pop(" not in src, (
            "Script should use player.trash_security_card, not raw pop()"
        )
