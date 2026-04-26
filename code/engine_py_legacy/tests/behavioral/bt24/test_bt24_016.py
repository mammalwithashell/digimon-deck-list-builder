"""Behavioral tests for BT24-016 Lamiamon (Lv.5 Red Digimon).

Card text:
[Hand] [Main] If you have [Owen Dreadnought], by placing 1 [Dimetromon]
from your trash as any of your [Elizamon]'s bottom digivolution card,
it digivolves into this card for a digivolution cost of 3, ignoring
digivolution requirements.

[When Digivolving] [When Attacking] [Once Per Turn] Your opponent places
1 card from their hand as the bottom security card. Then, trash their
top security card.

Inherited Effect:
[All Turns] [Once Per Turn] When your opponent's security stack is removed
from, you may play 1 5000 DP or lower [Reptile] or [Dragonkin] Digimon
card from your hand without paying the cost.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT24016Lamiamon:
    """Tests for BT24-016 Lamiamon."""

    # ── [When Digivolving][When Attacking] security manipulation ──────

    def test_security_effect_places_bottom_trashes_top(self, debug_runner):
        """[When Digivolving] opponent places hand card as bottom security,
        then trashes their top security card."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place Lamiamon on P1 field (as if just digivolved)
        perm = runner.place_on_field(1, ["BT24-016"])

        # Give P2 a hand card and some security
        runner.inject_card(2, "ST1-03", "hand")

        # Record P2 security and hand before
        snap_before = runner.snapshot()
        p2_hand_before = len(snap_before.p2_hand)
        p2_sec_before = snap_before.p2_security_count

        # Get the top security card ID before
        game = runner.game
        p2_top_sec_before = game.player2.security_cards[-1] if game.player2.security_cards else None

        # Manually fire the When Digivolving effect
        card = perm.top_card
        effects = card.effect_list(None)
        wd_effects = [e for e in effects if e.is_when_digivolving]
        assert len(wd_effects) == 1, "Should have 1 WhenDigivolving effect"

        wd = wd_effects[0]
        ctx = {
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'card': card,
        }
        assert wd.can_use_condition(ctx), "Condition should pass with Lamiamon on field"

        wd.on_process_callback(ctx)
        # This puts us into a SelectHand phase for the opponent
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Opponent should have lost 1 hand card
        assert len(snap_after.p2_hand) == p2_hand_before - 1, (
            f"Opponent hand should decrease by 1: {p2_hand_before} -> {len(snap_after.p2_hand)}"
        )
        # The top security from before should now be in trash (trashed)
        # Security count: gained 1 (from hand), lost 1 (trash) = net 0
        assert snap_after.p2_security_count == p2_sec_before, (
            f"Security should stay same (add 1 bottom, trash 1 top): {p2_sec_before} -> {snap_after.p2_security_count}"
        )

    def test_security_effect_trashes_top_not_bottom(self, debug_runner):
        """Verify the effect trashes the TOP security card (last element, index -1),
        not the bottom one (index 0)."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        game = runner.game
        # Clear P2 security and set up known cards
        runner.clear_zone(2, "security")
        # Append puts at end of list. Last appended = index -1 = top
        runner.inject_card(2, "BT1-010", "security_bottom")  # index 0 = bottom
        runner.inject_card(2, "ST1-03", "security_bottom")   # index 1 = top (last appended)

        # Give P2 a hand card to place as bottom security
        runner.inject_card(2, "BT24-008", "hand")

        # Place Lamiamon on P1 field
        perm = runner.place_on_field(1, ["BT24-016"])

        # Record which card is at top of security (index -1, should be trashed)
        top_sec_card = game.player2.security_cards[-1]
        top_sec_id = top_sec_card.c_entity_base.card_id if top_sec_card.c_entity_base else None

        # Fire the When Digivolving effect
        card = perm.top_card
        effects = card.effect_list(None)
        wd = [e for e in effects if e.is_when_digivolving][0]
        wd.on_process_callback({
            'player': game.player1, 'game': game,
            'permanent': perm, 'card': card,
        })
        runner.auto_resolve()

        # The TOP security card (index -1) should now be in trash
        p2_trash_ids = [c.c_entity_base.card_id for c in game.player2.trash_cards if c.c_entity_base]
        assert top_sec_id in p2_trash_ids, (
            f"Top security card {top_sec_id} should be in trash, got trash: {p2_trash_ids}"
        )

    def test_security_effect_places_at_bottom(self, debug_runner):
        """Verify the hand card is placed as the BOTTOM security card (index 0)."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        game = runner.game
        # Clear P2 security and hand, set up known state
        runner.clear_zone(2, "security")
        runner.clear_zone(2, "hand")
        runner.inject_card(2, "BT1-010", "security_bottom")  # existing security

        # Give P2 only one specific hand card to place
        runner.inject_card(2, "BT24-008", "hand")

        # Place Lamiamon on P1 field
        perm = runner.place_on_field(1, ["BT24-016"])

        # Fire the When Digivolving effect
        card = perm.top_card
        effects = card.effect_list(None)
        wd = [e for e in effects if e.is_when_digivolving][0]
        wd.on_process_callback({
            'player': game.player1, 'game': game,
            'permanent': perm, 'card': card,
        })
        runner.auto_resolve()

        # After: BT24-008 should have been placed as bottom security card
        # The bottom is index 0 in the security stack
        if game.player2.security_cards:
            bottom_id = game.player2.security_cards[0].c_entity_base.card_id
            assert bottom_id == "BT24-008", (
                f"Bottom security card should be BT24-008 (placed from hand), got {bottom_id}"
            )

    def test_security_effect_no_hand_still_trashes_top(self, debug_runner):
        """If opponent has no hand, should still trash top security if available."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        game = runner.game
        runner.clear_zone(2, "hand")
        p2_sec_before = len(game.player2.security_cards)

        perm = runner.place_on_field(1, ["BT24-016"])

        card = perm.top_card
        effects = card.effect_list(None)
        wd = [e for e in effects if e.is_when_digivolving][0]
        wd.on_process_callback({
            'player': game.player1, 'game': game,
            'permanent': perm, 'card': card,
        })
        runner.auto_resolve()

        # Should trash 1 top security even with no hand
        assert len(game.player2.security_cards) == p2_sec_before - 1, (
            f"Should trash 1 top security even with no hand: {p2_sec_before} -> {len(game.player2.security_cards)}"
        )

    def test_security_effect_when_attacking(self, debug_runner):
        """[When Attacking] same security manipulation effect should fire."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        game = runner.game
        perm = runner.place_on_field(1, ["BT24-016"])

        runner.inject_card(2, "ST1-03", "hand")

        card = perm.top_card
        effects = card.effect_list(None)
        oa = [e for e in effects if e.is_on_attack]
        assert len(oa) == 1, "Should have 1 OnAttack effect"

        snap_before = runner.snapshot()
        oa[0].on_process_callback({
            'player': game.player1, 'game': game,
            'permanent': perm, 'card': card,
        })
        runner.auto_resolve()

        snap_after = runner.snapshot()
        assert len(snap_after.p2_hand) == len(snap_before.p2_hand) - 1, (
            "Opponent should lose 1 hand card from When Attacking effect"
        )

    def test_security_effect_once_per_turn_shared(self, debug_runner):
        """[Once Per Turn] should be shared between WhenDigivolving and WhenAttacking."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        game = runner.game
        perm = runner.place_on_field(1, ["BT24-016"])

        # Give P2 multiple hand cards
        runner.inject_card(2, "ST1-03", "hand")
        runner.inject_card(2, "BT1-010", "hand")

        card = perm.top_card
        effects = card.effect_list(None)
        wd = [e for e in effects if e.is_when_digivolving][0]
        oa = [e for e in effects if e.is_on_attack][0]

        # Simulate the WhenDigivolving firing (including record_activation)
        wd.record_activation()
        wd.on_process_callback({
            'player': game.player1, 'game': game,
            'permanent': perm, 'card': card,
        })
        runner.auto_resolve()

        # Now WhenAttacking should NOT fire (OPT exhausted via shared counter)
        assert not oa.can_activate_this_turn(), (
            "WhenAttacking should be blocked by shared OPT after WhenDigivolving fired"
        )

    def test_security_effect_trashes_via_proper_method(self, debug_runner):
        """Security trash should fire OnLoseSecurity timing (use trash_security_card)."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        game = runner.game
        # Give P2 a hand card
        runner.inject_card(2, "ST1-03", "hand")

        perm = runner.place_on_field(1, ["BT24-016"])

        # Track OnLoseSecurity via a flag
        on_lose_security_fired = [False]
        original_fire = game.player2._fire_timing.__func__ if hasattr(game.player2._fire_timing, '__func__') else None

        def tracking_fire(self_player, timing, context=None):
            if timing == EffectTiming.OnLoseSecurity:
                on_lose_security_fired[0] = True
            if original_fire:
                original_fire(self_player, timing, context)
            elif self_player.game and hasattr(self_player.game, 'execute_effects'):
                self_player.game.execute_effects(timing, context)

        import types
        game.player2._fire_timing = types.MethodType(tracking_fire, game.player2)

        card = perm.top_card
        effects = card.effect_list(None)
        wd = [e for e in effects if e.is_when_digivolving][0]
        wd.on_process_callback({
            'player': game.player1, 'game': game,
            'permanent': perm, 'card': card,
        })
        runner.auto_resolve()

        assert on_lose_security_fired[0], (
            "Trashing top security should fire OnLoseSecurity timing"
        )

    # ── Inherited: [All Turns] OnLoseSecurity play effect ─────────────

    def test_inherited_fires_on_opponent_security_loss(self, debug_runner):
        """Inherited should trigger when OPPONENT's security is removed."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        game = runner.game
        # Place a Digimon with Lamiamon as inherited (under another Digimon)
        # BT24-016 (Lamiamon Lv.5) under some Lv.6 top card
        # Use BT24-017 as top card since it's Lv.6 Red
        perm = runner.place_on_field(1, ["BT24-016", "BT24-017"])

        # Give P1 a Reptile Digimon with <= 5000 DP in hand
        runner.inject_card(1, "ST1-03", "hand")  # Agumon, 2000 DP, Reptile

        card = perm.card_sources[0]  # Lamiamon (bottom, inherited source)
        effects = card.effect_list(None)
        inherited_ols = [e for e in effects
                         if e.is_inherited_effect and e.timing == EffectTiming.OnLoseSecurity]
        assert len(inherited_ols) == 1, "Should have 1 inherited OnLoseSecurity effect"

        eff = inherited_ols[0]
        # Context: opponent (player2) lost security
        ctx = {
            'player': game.player1,  # owner of effect
            'game': game,
            'permanent': perm,
            'card': card,
            'event_player': game.player2,  # player who lost security
        }
        assert eff.can_use_condition(ctx), (
            "Inherited should trigger when opponent loses security"
        )

    def test_inherited_does_not_fire_on_own_security_loss(self, debug_runner):
        """Inherited should NOT trigger when YOUR OWN security is removed."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        game = runner.game
        perm = runner.place_on_field(1, ["BT24-016", "BT24-017"])
        runner.inject_card(1, "ST1-03", "hand")

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        inherited_ols = [e for e in effects
                         if e.is_inherited_effect and e.timing == EffectTiming.OnLoseSecurity]
        eff = inherited_ols[0]

        # Context: own player (player1) lost security
        ctx = {
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'card': card,
            'event_player': game.player1,  # own player lost security
        }
        assert not eff.can_use_condition(ctx), (
            "Inherited should NOT trigger when own security is removed"
        )

    def test_inherited_fires_on_opponent_turn(self, debug_runner):
        """[All Turns] inherited should fire on OPPONENT's turn too."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        game = runner.game
        perm = runner.place_on_field(1, ["BT24-016", "BT24-017"])
        runner.inject_card(1, "ST1-03", "hand")

        # Switch to P2's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        inherited_ols = [e for e in effects
                         if e.is_inherited_effect and e.timing == EffectTiming.OnLoseSecurity]
        eff = inherited_ols[0]

        # Context: opponent (player2) lost security, on player2's turn
        ctx = {
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'card': card,
            'event_player': game.player2,
        }
        assert eff.can_use_condition(ctx), (
            "[All Turns] inherited should fire even on opponent's turn"
        )

    def test_inherited_plays_reptile_or_dragonkin_5000dp_or_less(self, debug_runner):
        """Inherited should play a Reptile or Dragonkin Digimon with <= 5000 DP."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        game = runner.game
        perm = runner.place_on_field(1, ["BT24-016", "BT24-017"])

        # Inject a Reptile Digimon with <= 5000 DP into P1 hand
        runner.inject_card(1, "ST1-03", "hand")  # Agumon, 2000 DP, Reptile

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)
        field_before = len(snap_before.p1_field)

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        inherited_ols = [e for e in effects
                         if e.is_inherited_effect and e.timing == EffectTiming.OnLoseSecurity]
        eff = inherited_ols[0]

        ctx = {
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'card': card,
            'event_player': game.player2,
        }
        eff.on_process_callback(ctx)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Should have played the Reptile from hand
        assert len(snap_after.p1_hand) == hand_before - 1, (
            f"Should play 1 card from hand: {hand_before} -> {len(snap_after.p1_hand)}"
        )
        assert len(snap_after.p1_field) == field_before + 1, (
            f"Should add 1 Digimon to field: {field_before} -> {len(snap_after.p1_field)}"
        )

    # ── [Hand][Main] Special digivolution ─────────────────────────────

    def _find_hand_main_action(self, runner, card_id="BT24-016"):
        """Find the hand main action ID for a specific card.

        Hand main actions use action IDs 30 + hand_index.
        They are described as 'Trash X from hand' in action_describe.
        """
        game = runner.game
        legal = runner.action_mask()
        for i, c in enumerate(game.player1.hand_cards):
            if c.c_entity_base and c.c_entity_base.card_id == card_id:
                action_id = 30 + i  # HAND_MAIN_START + hand_idx
                if action_id in legal:
                    return action_id
        return None

    def test_hand_main_condition_requires_owen(self, debug_runner):
        """[Hand][Main] requires Owen Dreadnought on field."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        game = runner.game
        # Inject Lamiamon into hand
        runner.inject_card(1, "BT24-016", "hand")
        # Inject Dimetromon into trash
        runner.inject_card(1, "BT24-012", "trash")
        # Place Elizamon on field (but NO Owen)
        runner.place_on_field(1, ["BT24-008"])

        # The Hand Main effect should not be available (no Owen)
        action = self._find_hand_main_action(runner)
        assert action is None, (
            "Hand Main effect should not be available without Owen Dreadnought"
        )

    def test_hand_main_condition_requires_elizamon(self, debug_runner):
        """[Hand][Main] requires Elizamon on field."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        game = runner.game
        runner.inject_card(1, "BT24-016", "hand")
        runner.inject_card(1, "BT24-012", "trash")
        # Place Owen tamer but NO Elizamon
        runner.place_on_field(1, ["BT24-082"])

        action = self._find_hand_main_action(runner)
        assert action is None, (
            "Hand Main effect should not be available without Elizamon on field"
        )

    def test_hand_main_condition_requires_dimetromon_in_trash(self, debug_runner):
        """[Hand][Main] requires Dimetromon in trash."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        game = runner.game
        runner.inject_card(1, "BT24-016", "hand")
        # Owen on field, Elizamon on field, but NO Dimetromon in trash
        runner.place_on_field(1, ["BT24-082"])
        runner.place_on_field(1, ["BT24-008"])

        action = self._find_hand_main_action(runner)
        assert action is None, (
            "Hand Main effect should not be available without Dimetromon in trash"
        )

    def test_hand_main_full_setup_available(self, debug_runner):
        """[Hand][Main] should be available with Owen, Elizamon, Dimetromon in trash."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        game = runner.game
        runner.inject_card(1, "BT24-016", "hand")
        runner.inject_card(1, "BT24-012", "trash")
        runner.place_on_field(1, ["BT24-082"])  # Owen
        runner.place_on_field(1, ["BT24-008"])  # Elizamon

        action = self._find_hand_main_action(runner)
        assert action is not None, (
            "Hand Main effect should be available with full setup"
        )

    def test_hand_main_digivolve_costs_3(self, debug_runner):
        """[Hand][Main] should cost 3 memory to digivolve."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        game = runner.game
        # Clear opponent hand/security to prevent cascading effects
        runner.clear_zone(2, "hand")
        runner.clear_zone(2, "security")

        runner.inject_card(1, "BT24-016", "hand")
        runner.inject_card(1, "BT24-012", "trash")
        runner.place_on_field(1, ["BT24-082"])  # Owen
        runner.place_on_field(1, ["BT24-008"])  # Elizamon

        mem_before = game.memory

        action = self._find_hand_main_action(runner)
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Memory should decrease by 3 (no cascading effects since P2 has no hand/security)
        assert game.memory == mem_before - 3, (
            f"Digivolve cost should be 3: memory {mem_before} -> {game.memory}"
        )

    def test_hand_main_places_dimetromon_under_elizamon(self, debug_runner):
        """[Hand][Main] should place Dimetromon from trash under Elizamon."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        game = runner.game
        runner.inject_card(1, "BT24-016", "hand")
        runner.inject_card(1, "BT24-012", "trash")
        runner.place_on_field(1, ["BT24-082"])  # Owen
        elizamon_perm = runner.place_on_field(1, ["BT24-008"])  # Elizamon

        action = self._find_hand_main_action(runner)
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Dimetromon should be in the stack (placed under)
        # And Lamiamon should now be the top card (digivolved onto)
        stack_ids = [cs.c_entity_base.card_id for cs in elizamon_perm.card_sources if cs.c_entity_base]
        assert "BT24-012" in stack_ids, (
            f"Dimetromon (BT24-012) should be in the digivolution stack: {stack_ids}"
        )
        assert "BT24-016" in stack_ids, (
            f"Lamiamon (BT24-016) should be in the digivolution stack: {stack_ids}"
        )
        assert elizamon_perm.top_card.c_entity_base.card_id == "BT24-016", (
            "Lamiamon should be the top card after digivolving"
        )

    def test_hand_main_dimetromon_removed_from_trash(self, debug_runner):
        """[Hand][Main] Dimetromon should be removed from trash after placing."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        game = runner.game
        runner.inject_card(1, "BT24-016", "hand")
        runner.inject_card(1, "BT24-012", "trash")
        runner.place_on_field(1, ["BT24-082"])  # Owen
        runner.place_on_field(1, ["BT24-008"])  # Elizamon

        action = self._find_hand_main_action(runner)
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert "BT24-012" not in snap.p1_trash, (
            "Dimetromon should no longer be in trash"
        )
