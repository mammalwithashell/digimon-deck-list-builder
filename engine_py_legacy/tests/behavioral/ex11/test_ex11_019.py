"""Behavioral tests for EX11-019 Shoemon (Lv.3 Yellow Digimon).

Card text:
  [On Deletion] You may play 1 [Familiar] Token. (Digimon/Yellow/3000 DP/
  [On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn.)
  Inherited: Barrier (When this Digimon would be deleted in battle, by trashing
  the top card of your security stack, prevent that deletion.)
"""

import pytest

# EX11-019 Shoemon is Yellow Lv.3, traits: Puppet, LIBERATOR
SHOEMON_DECK = ["EX11-019"] * 50


@pytest.mark.behavioral
class TestEX11019Shoemon:
    """Tests for EX11-019 Shoemon."""

    # ---- Effect structure ------------------------------------------------

    def test_on_deletion_effect_exists(self, debug_runner):
        """Shoemon should have an [On Deletion] effect with is_on_deletion=True."""
        runner = debug_runner(deck1=SHOEMON_DECK, deck2=SHOEMON_DECK, initial_memory=10)

        runner.inject_card(1, "EX11-019", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        on_del = [e for e in effects if getattr(e, 'is_on_deletion', False)]
        assert len(on_del) == 1, f"Should have 1 On Deletion effect, got {len(on_del)}"

    def test_on_deletion_effect_is_optional(self, debug_runner):
        """The [On Deletion] effect should be optional ('You may')."""
        runner = debug_runner(deck1=SHOEMON_DECK, deck2=SHOEMON_DECK, initial_memory=10)

        runner.inject_card(1, "EX11-019", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        on_del = [e for e in effects if getattr(e, 'is_on_deletion', False)]
        assert len(on_del) == 1
        assert on_del[0].is_optional, "On Deletion should be optional ('You may')"

    def test_inherited_barrier_effect_exists(self, debug_runner):
        """Shoemon should have an inherited Barrier effect."""
        runner = debug_runner(deck1=SHOEMON_DECK, deck2=SHOEMON_DECK, initial_memory=10)

        runner.inject_card(1, "EX11-019", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        barrier_inh = [
            e for e in effects
            if getattr(e, '_is_barrier', False) and e.is_inherited_effect
        ]
        assert len(barrier_inh) == 1, "Should have 1 inherited Barrier effect"

    def test_no_own_barrier(self, debug_runner):
        """Shoemon's Barrier is inherited only -- no own (non-inherited) Barrier."""
        runner = debug_runner(deck1=SHOEMON_DECK, deck2=SHOEMON_DECK, initial_memory=10)

        runner.inject_card(1, "EX11-019", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        barrier_own = [
            e for e in effects
            if getattr(e, '_is_barrier', False) and not e.is_inherited_effect
        ]
        assert len(barrier_own) == 0, "Shoemon should NOT have an own Barrier effect"

    # ---- On Deletion: plays Familiar Token ------------------------------

    def test_on_deletion_plays_familiar_token(self, debug_runner):
        """When Shoemon is deleted, accepting the optional effect should play a
        Familiar Token onto the field."""
        runner = debug_runner(deck1=SHOEMON_DECK, deck2=SHOEMON_DECK, initial_memory=10)

        # Place Shoemon on P1 field
        perm = runner.place_on_field(1, ["EX11-019"])
        field_before = len(runner.snapshot().p1_field)

        # Delete Shoemon -- triggers On Deletion
        runner.game.player1.delete_permanent(perm)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Shoemon is gone; a Familiar Token should have appeared
        familiar_on_field = [
            s for s in snap.p1_field
            if s.card_id == "TOKEN_FAMILIAR"
        ]
        assert len(familiar_on_field) == 1, (
            f"Expected 1 Familiar Token on field, found {len(familiar_on_field)}. "
            f"P1 field: {[(s.card_id, s.card_name) for s in snap.p1_field]}"
        )

    def test_on_deletion_token_has_correct_dp(self, debug_runner):
        """Familiar Token should have 3000 DP."""
        runner = debug_runner(deck1=SHOEMON_DECK, deck2=SHOEMON_DECK, initial_memory=10)

        perm = runner.place_on_field(1, ["EX11-019"])
        runner.game.player1.delete_permanent(perm)
        runner.auto_resolve()

        snap = runner.snapshot()
        familiar = [s for s in snap.p1_field if s.card_id == "TOKEN_FAMILIAR"]
        assert len(familiar) == 1
        assert familiar[0].dp == 3000, f"Familiar Token DP should be 3000, got {familiar[0].dp}"

    def test_on_deletion_shoemon_goes_to_trash(self, debug_runner):
        """After deletion, Shoemon card should end up in trash."""
        runner = debug_runner(deck1=SHOEMON_DECK, deck2=SHOEMON_DECK, initial_memory=10)

        perm = runner.place_on_field(1, ["EX11-019"])
        runner.game.player1.delete_permanent(perm)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert "EX11-019" in snap.p1_trash, (
            f"Shoemon should be in trash after deletion, trash: {snap.p1_trash}"
        )

    # ---- Inherited Barrier: active when under a digivolution ------------

    def test_inherited_barrier_active_when_stacked(self, debug_runner):
        """When Shoemon is a digivolution source (under another Digimon),
        the parent permanent should gain Barrier via inherited effect."""
        runner = debug_runner(deck1=SHOEMON_DECK, deck2=SHOEMON_DECK, initial_memory=10)

        # Stack: Shoemon (bottom) + a Lv.4 on top
        # Use a generic Lv.4 that does NOT have its own Barrier
        perm = runner.place_on_field(1, ["EX11-019", "BT1-036"])

        assert perm.has_keyword('_is_barrier'), (
            "Permanent with Shoemon underneath should have Barrier keyword"
        )

    def test_inherited_barrier_prevents_battle_deletion(self, debug_runner):
        """When the parent Digimon would be deleted in battle, Barrier should
        trash top security to prevent deletion."""
        runner = debug_runner(deck1=SHOEMON_DECK, deck2=SHOEMON_DECK, initial_memory=10)

        # Stack Shoemon under a Lv.4
        perm = runner.place_on_field(1, ["EX11-019", "BT1-036"])
        sec_before = len(runner.game.player1.security_cards)
        assert sec_before > 0, "Need security for Barrier to work"

        # Delete in battle -- Barrier should trigger
        deleted = runner.game.player1.delete_permanent(perm, is_battle=True)

        assert not deleted, "Barrier should prevent the battle deletion"
        snap = runner.snapshot()
        assert snap.p1_security_count == sec_before - 1, (
            f"Barrier should trash 1 security card: {sec_before} -> {snap.p1_security_count}"
        )
        # Permanent should still be on field
        assert len(snap.p1_field) == 1, "Digimon should survive thanks to Barrier"

    def test_inherited_barrier_no_security_no_prevention(self, debug_runner):
        """Barrier should NOT prevent deletion when there is no security to trash."""
        runner = debug_runner(deck1=SHOEMON_DECK, deck2=SHOEMON_DECK, initial_memory=10)

        perm = runner.place_on_field(1, ["EX11-019", "BT1-036"])
        runner.clear_zone(1, "security")
        assert len(runner.game.player1.security_cards) == 0

        deleted = runner.game.player1.delete_permanent(perm, is_battle=True)

        assert deleted, "Barrier should NOT prevent deletion with 0 security"
        snap = runner.snapshot()
        # The stacked Digimon is deleted; Shoemon's On Deletion fires
        # and plays a Familiar Token, so the field may have that token.
        stacked_on_field = [
            s for s in snap.p1_field if s.card_id == "BT1-036"
        ]
        assert len(stacked_on_field) == 0, "Stacked Digimon should be deleted"

    def test_inherited_barrier_not_active_for_effect_deletion(self, debug_runner):
        """Barrier only triggers during battle deletion, NOT effect deletion."""
        runner = debug_runner(deck1=SHOEMON_DECK, deck2=SHOEMON_DECK, initial_memory=10)

        perm = runner.place_on_field(1, ["EX11-019", "BT1-036"])
        sec_before = len(runner.game.player1.security_cards)

        # Delete by effect (is_battle=False) -- Barrier should NOT trigger
        deleted = runner.game.player1.delete_permanent(perm, is_battle=False)

        assert deleted, "Barrier should NOT prevent effect-based deletion"
        snap = runner.snapshot()
        assert snap.p1_security_count == sec_before, (
            "Security should not change when Barrier doesn't trigger"
        )

    def test_barrier_not_active_on_shoemon_as_top_card(self, debug_runner):
        """When Shoemon IS the top card (not stacked under), Barrier should NOT
        be active because it is an inherited-only effect."""
        runner = debug_runner(deck1=SHOEMON_DECK, deck2=SHOEMON_DECK, initial_memory=10)

        perm = runner.place_on_field(1, ["EX11-019"])

        assert not perm.has_keyword('_is_barrier'), (
            "Shoemon as top card should NOT have Barrier (it is inherited-only)"
        )
