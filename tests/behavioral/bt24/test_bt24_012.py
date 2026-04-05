"""Behavioral tests for BT24-012 Dimetromon (Lv.4 Red Digimon).

Card text:
  Blocker
  [All Turns] When any of your other Digimon with the [Reptile] or [Dragonkin]
  trait would leave the battle area by your opponent's effects, by returning
  this Digimon to the hand, they don't leave.

  Inherited: [Your Turn] [Once Per Turn] When your opponent's security stack
  is removed from, gain 1 memory.
"""

import pytest


@pytest.mark.behavioral
class TestBT24012Dimetromon:
    """Tests for BT24-012 Dimetromon."""

    # ── Protection Effect (WhenPermanentWouldBeDeleted) ──────────────

    def test_protection_prevents_deletion_of_reptile_by_opponent(self, debug_runner):
        """Returning Dimetromon to hand prevents Reptile ally from being deleted by opponent."""
        runner = debug_runner(initial_memory=5)

        dimetromon = runner.place_on_field(1, ["BT24-012"])
        # BT1-010 Agumon has [Reptile] trait
        reptile_ally = runner.place_on_field(1, ["BT1-010"])

        before_hand = len(runner.game.player1.hand_cards)

        # Opponent effect tries to delete the Reptile ally
        deleted = runner.game.player1.delete_permanent(reptile_ally, is_opponent_effect=True)

        assert not deleted, "Reptile ally should NOT be deleted (Dimetromon protection)"

        snap = runner.snapshot()
        # Reptile ally should still be on the field
        assert any(s.card_id == "BT1-010" for s in snap.p1_field), (
            "Agumon (Reptile) should still be on the field")
        # Dimetromon should be gone from field (returned to hand)
        assert not any(s.card_id == "BT24-012" for s in snap.p1_field), (
            "Dimetromon should have left the field (returned to hand)")
        # Dimetromon's top card should be in hand
        after_hand = len(runner.game.player1.hand_cards)
        assert after_hand > before_hand, (
            "Dimetromon should have been returned to hand")

    def test_protection_prevents_deletion_of_dragonkin_by_opponent(self, debug_runner):
        """Returning Dimetromon to hand prevents Dragonkin ally from being deleted by opponent."""
        runner = debug_runner(initial_memory=5)

        dimetromon = runner.place_on_field(1, ["BT24-012"])
        # BT1-025 WarGreymon has [Dragonkin] trait
        dragonkin_ally = runner.place_on_field(1, ["BT1-025"])

        deleted = runner.game.player1.delete_permanent(dragonkin_ally, is_opponent_effect=True)

        assert not deleted, "Dragonkin ally should NOT be deleted (Dimetromon protection)"

        snap = runner.snapshot()
        assert any(s.card_id == "BT1-025" for s in snap.p1_field), (
            "WarGreymon (Dragonkin) should still be on the field")
        assert not any(s.card_id == "BT24-012" for s in snap.p1_field), (
            "Dimetromon should be gone from field")

    def test_no_protection_by_own_effect(self, debug_runner):
        """Protection does NOT trigger when deletion is by own effect."""
        runner = debug_runner(initial_memory=5)

        dimetromon = runner.place_on_field(1, ["BT24-012"])
        reptile_ally = runner.place_on_field(1, ["BT1-010"])

        # Delete by own effect (not opponent)
        deleted = runner.game.player1.delete_permanent(reptile_ally, is_opponent_effect=False)

        assert deleted, "Reptile ally SHOULD be deleted (own effect, no protection)"

        snap = runner.snapshot()
        assert not any(s.card_id == "BT1-010" for s in snap.p1_field), (
            "Agumon should be gone from the field")
        # Dimetromon should remain on field (not triggered)
        assert any(s.card_id == "BT24-012" for s in snap.p1_field), (
            "Dimetromon should still be on the field (no trigger)")

    def test_no_protection_for_non_matching_trait(self, debug_runner):
        """Protection does NOT trigger for Digimon without Reptile/Dragonkin trait."""
        runner = debug_runner(initial_memory=5)

        dimetromon = runner.place_on_field(1, ["BT24-012"])
        # BT1-009 Monodramon has [Mini Dragon] trait — no Reptile or Dragonkin
        non_matching = runner.place_on_field(1, ["BT1-009"])

        deleted = runner.game.player1.delete_permanent(non_matching, is_opponent_effect=True)

        assert deleted, "Non-matching trait Digimon SHOULD be deleted"

        snap = runner.snapshot()
        assert not any(s.card_id == "BT1-009" for s in snap.p1_field), (
            "Monodramon should be gone from the field")
        # Dimetromon should remain on field (not triggered)
        assert any(s.card_id == "BT24-012" for s in snap.p1_field), (
            "Dimetromon should still be on the field (no trigger)")

    def test_no_self_protection(self, debug_runner):
        """Protection does NOT trigger for Dimetromon itself (must be 'other' Digimon)."""
        runner = debug_runner(initial_memory=5)

        dimetromon = runner.place_on_field(1, ["BT24-012"])
        # Dimetromon itself has [Reptile] trait but effect says "other"

        deleted = runner.game.player1.delete_permanent(dimetromon, is_opponent_effect=True)

        assert deleted, "Dimetromon SHOULD be deleted (can't protect itself)"

        snap = runner.snapshot()
        assert not any(s.card_id == "BT24-012" for s in snap.p1_field), (
            "Dimetromon should be gone from the field")

    # ── Inherited Effect (OnLoseSecurity) ────────────────────────────

    def test_inherited_gain_memory_on_opponent_security_loss(self, debug_runner):
        """Inherited: gain 1 memory when opponent's security is removed on your turn."""
        runner = debug_runner(initial_memory=5)

        # Dimetromon digivolved onto a Lv.3 (gives inherited effect)
        carrier = runner.place_on_field(1, ["BT24-012", "BT1-010"])

        # Give opponent some security
        runner.inject_card(2, "BT1-009", "security_top")
        runner.inject_card(2, "BT1-009", "security_top")

        # It's player 1's turn
        runner.set_phase("Main")
        before_mem = runner.game.memory

        # Simulate opponent losing security
        from digimon_gym.engine.data.enums import EffectTiming
        opp = runner.game.player2
        assert len(opp.security_cards) >= 1
        lost = opp.security_cards.pop(0)
        opp.trash_cards.append(lost)
        runner.game.execute_effects(EffectTiming.OnLoseSecurity, {
            "lost_card": lost, "player": opp
        })

        after_mem = runner.game.memory

        assert after_mem == before_mem + 1, (
            f"Should gain 1 memory. Before={before_mem}, After={after_mem}")

    def test_inherited_no_gain_on_own_security_loss(self, debug_runner):
        """Inherited: does NOT trigger when own security is removed."""
        runner = debug_runner(initial_memory=5)

        carrier = runner.place_on_field(1, ["BT24-012", "BT1-010"])

        # Give player 1 some security
        runner.inject_card(1, "BT1-009", "security_top")

        runner.set_phase("Main")
        before_mem = runner.game.memory

        # Player 1 (the card owner) loses security
        from digimon_gym.engine.data.enums import EffectTiming
        p1 = runner.game.player1
        assert len(p1.security_cards) >= 1
        lost = p1.security_cards.pop(0)
        p1.trash_cards.append(lost)
        runner.game.execute_effects(EffectTiming.OnLoseSecurity, {
            "lost_card": lost, "player": p1
        })

        after_mem = runner.game.memory

        assert after_mem == before_mem, (
            f"Should NOT gain memory when own security lost. Before={before_mem}, After={after_mem}")

    def test_inherited_no_gain_on_opponent_turn(self, debug_runner):
        """Inherited: does NOT trigger on opponent's turn (Your Turn only)."""
        runner = debug_runner(initial_memory=5)

        carrier = runner.place_on_field(1, ["BT24-012", "BT1-010"])

        # Give opponent some security
        runner.inject_card(2, "BT1-009", "security_top")

        # Switch to opponent's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True
        runner.game.turn_player = runner.game.player2
        runner.game.opponent_player = runner.game.player1

        before_mem = runner.game.memory

        # Opponent loses security on their own turn
        from digimon_gym.engine.data.enums import EffectTiming
        opp = runner.game.player2
        assert len(opp.security_cards) >= 1
        lost = opp.security_cards.pop(0)
        opp.trash_cards.append(lost)
        runner.game.execute_effects(EffectTiming.OnLoseSecurity, {
            "lost_card": lost, "player": opp
        })

        after_mem = runner.game.memory

        assert after_mem == before_mem, (
            f"Should NOT gain memory on opponent's turn. Before={before_mem}, After={after_mem}")

    def test_inherited_once_per_turn(self, debug_runner):
        """Inherited: gain memory only once per turn (OPT)."""
        runner = debug_runner(initial_memory=5)

        carrier = runner.place_on_field(1, ["BT24-012", "BT1-010"])

        # Give opponent plenty of security
        runner.inject_card(2, "BT1-009", "security_top")
        runner.inject_card(2, "BT1-009", "security_top")
        runner.inject_card(2, "BT1-009", "security_top")

        runner.set_phase("Main")
        before_mem = runner.game.memory

        from digimon_gym.engine.data.enums import EffectTiming

        # First security loss — should trigger
        opp = runner.game.player2
        lost1 = opp.security_cards.pop(0)
        opp.trash_cards.append(lost1)
        runner.game.execute_effects(EffectTiming.OnLoseSecurity, {
            "lost_card": lost1, "player": opp
        })

        mid_mem = runner.game.memory
        assert mid_mem == before_mem + 1, (
            f"First trigger should gain 1 memory. Before={before_mem}, After={mid_mem}")

        # Second security loss — should NOT trigger (OPT)
        lost2 = opp.security_cards.pop(0)
        opp.trash_cards.append(lost2)
        runner.game.execute_effects(EffectTiming.OnLoseSecurity, {
            "lost_card": lost2, "player": opp
        })

        final_mem = runner.game.memory
        assert final_mem == mid_mem, (
            f"OPT should prevent second trigger. Mid={mid_mem}, Final={final_mem}")
