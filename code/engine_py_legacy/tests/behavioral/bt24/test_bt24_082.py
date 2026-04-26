"""Behavioral tests for BT24-082 Owen Dreadnought (Tamer).

Card text:
[Start of Your Main Phase] By returning this Tamer to the bottom of the deck,
you may play 1 [Owen Dreadnought] from your hand without paying the cost.
Then, if you don't have a Digimon, you may play 1 [Elizamon] from your trash
without paying the cost.

[Your Turn] When any of your Digimon digivolve into a [Reptile] or [Dragonkin]
Digimon, by suspending this Tamer, that Digimon gets +3000 DP for the turn.
Then, it may attack.

Security Effect [Security] Play this card without paying the cost.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming

# BT1-010 Agumon: Lv3 Red Reptile, 2000 DP
# BT21-017 Dimetromon: Lv4 Red Reptile, 4000 DP, digi from Lv3 Red for 2
# BT24-008 Elizamon: Lv3 Red, 2000 DP
OWEN_DECK = ["BT24-082"] * 4 + ["BT1-010"] * 23 + ["BT24-008"] * 23


@pytest.mark.behavioral
class TestBT24082OwenDreadnought:
    """Tests for BT24-082 Owen Dreadnought."""

    # ── Effect 0: Start of Your Main Phase ───────���───────────────────

    def test_start_of_main_phase_return_to_deck_play_owen_from_hand(self, debug_runner):
        """Start of Main Phase: return Owen to deck bottom, play Owen from hand."""
        runner = debug_runner(deck1=OWEN_DECK, deck2=OWEN_DECK, initial_memory=5)
        runner.set_phase("Main")

        # Place Owen on P1 field
        runner.place_on_field(1, ["BT24-082"], turn_played=-1)
        # Also place a Digimon so Elizamon branch is NOT triggered
        runner.place_on_field(1, ["BT1-010"], turn_played=-1)

        # Put another Owen in hand
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT24-082", "hand")

        snap_before = runner.snapshot()
        field_count_before = len(snap_before.p1_field)
        # Should have Owen tamer + Agumon on field
        assert any(s.card_id == "BT24-082" for s in snap_before.p1_field)

        # Trigger start of main phase effects
        game = runner.game
        game.execute_effects(EffectTiming.OnStartMainPhase)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Original Owen should be gone (returned to deck bottom)
        # New Owen from hand should be on field
        owens_on_field = [s for s in snap.p1_field if s.card_id == "BT24-082"]
        assert len(owens_on_field) == 1, \
            f"Should have exactly 1 Owen on field (new from hand), got {len(owens_on_field)}"
        # Owen should not be in hand anymore
        assert "BT24-082" not in snap.p1_hand, \
            "Owen should have been played from hand"

    def test_start_of_main_phase_no_owen_in_hand_skips_play(self, debug_runner):
        """If no Owen in hand, the play-from-hand step is skipped."""
        runner = debug_runner(deck1=OWEN_DECK, deck2=OWEN_DECK, initial_memory=5)
        runner.set_phase("Main")

        runner.place_on_field(1, ["BT24-082"], turn_played=-1)
        runner.place_on_field(1, ["BT1-010"], turn_played=-1)
        runner.clear_zone(1, "hand")
        # No Owen in hand — only Agumon
        runner.inject_card(1, "BT1-010", "hand")

        game = runner.game
        game.execute_effects(EffectTiming.OnStartMainPhase)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Owen was returned to deck, but no new Owen was played
        assert not any(s.card_id == "BT24-082" for s in snap.p1_field), \
            "Owen should have been returned to deck bottom with no replacement"

    def test_start_of_main_phase_elizamon_from_trash_when_no_digimon(self, debug_runner):
        """If no Digimon after Owen play, may play Elizamon from trash."""
        runner = debug_runner(deck1=OWEN_DECK, deck2=OWEN_DECK, initial_memory=5)
        runner.set_phase("Main")

        # Owen on field, NO Digimon
        runner.place_on_field(1, ["BT24-082"], turn_played=-1)
        runner.clear_zone(1, "hand")
        # Put Owen in hand (to play from hand) -- it's a tamer, not a Digimon
        runner.inject_card(1, "BT24-082", "hand")
        # Put Elizamon in trash
        runner.inject_card(1, "BT24-008", "trash")

        game = runner.game
        game.execute_effects(EffectTiming.OnStartMainPhase)

        # First selection: pick Owen from hand (action 0 = first hand card)
        from engine_py_legacy.engine.data.enums import GamePhase
        assert game.current_phase == GamePhase.SelectTarget, \
            f"Should be in SelectTarget for Owen play, got {game.current_phase}"
        owen_action = runner.find_action("Owen Dreadnought")
        assert owen_action is not None, f"Should find Owen selection. Actions: {runner.actions()}"
        runner.execute(owen_action)

        # Second selection: Elizamon from trash should now be offered
        # (since no Digimon on field -- Owen is a tamer)
        assert game.current_phase == GamePhase.SelectTarget, \
            f"Should be in SelectTarget for Elizamon play, got {game.current_phase}"
        # Trash selections start at index 130 (SEL_TRASH_START)
        legal = runner.action_mask()
        trash_actions = [a for a in legal if a >= 130]
        assert len(trash_actions) > 0, \
            f"Should have trash selection for Elizamon. Legal: {legal}, actions: {runner.actions()}"
        runner.execute(trash_actions[0])
        runner.auto_resolve()

        snap = runner.snapshot()
        # Owen from hand should be on field (tamer)
        owens = [s for s in snap.p1_field if s.card_id == "BT24-082"]
        assert len(owens) == 1, f"New Owen should be on field, got {len(owens)}"
        # Elizamon should be on field (played from trash because no Digimon)
        elizas = [s for s in snap.p1_field if s.card_id == "BT24-008"]
        assert len(elizas) == 1, \
            f"Elizamon should have been played from trash (no Digimon), got {len(elizas)}"

    def test_start_of_main_phase_elizamon_skipped_when_digimon_exists(self, debug_runner):
        """If a Digimon exists, Elizamon play is NOT offered."""
        runner = debug_runner(deck1=OWEN_DECK, deck2=OWEN_DECK, initial_memory=5)
        runner.set_phase("Main")

        # Owen + Agumon on field (Agumon is a Digimon)
        runner.place_on_field(1, ["BT24-082"], turn_played=-1)
        runner.place_on_field(1, ["BT1-010"], turn_played=-1)

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT24-082", "hand")
        runner.inject_card(1, "BT24-008", "trash")

        game = runner.game
        game.execute_effects(EffectTiming.OnStartMainPhase)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Elizamon should NOT be on field (Digimon already exists)
        elizas = [s for s in snap.p1_field if s.card_id == "BT24-008"]
        assert len(elizas) == 0, \
            f"Elizamon should NOT be played when Digimon exists, got {len(elizas)}"
        # Elizamon should still be in trash
        assert "BT24-008" in snap.p1_trash, \
            "Elizamon should remain in trash"

    # ── Effect 1: Digivolve observer — +3000 DP and may attack ──────

    def test_digivolve_into_reptile_grants_dp_boost(self, debug_runner):
        """Digivolving into Reptile trait Digimon should grant +3000 DP."""
        # Need Agumon (Lv3) + Dimetromon (Lv4 Reptile) in deck
        deck = ["BT24-082"] * 4 + ["BT1-010"] * 23 + ["BT21-017"] * 23
        runner = debug_runner(deck1=deck, deck2=OWEN_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Place Owen tamer on P1 field (unsuspended)
        runner.place_on_field(1, ["BT24-082"], turn_played=-1)
        # Place Agumon (Lv3 Reptile) on P1 field (old enough to attack)
        runner.place_on_field(1, ["BT1-010"], turn_played=-1)

        # Inject Dimetromon (Lv4 Reptile) into hand for digivolution
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT21-017", "hand")

        # Get Agumon's base DP
        snap_before = runner.snapshot()
        agumon_slot = next(s for s in snap_before.p1_field if s.card_id == "BT1-010")
        dp_before = agumon_slot.dp

        # Digivolve Agumon into Dimetromon
        action = runner.find_action("Digivolve")
        assert action is not None, f"Should find digivolve action. Actions: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Find Dimetromon on field
        dimetromon = next((s for s in snap.p1_field if s.card_id == "BT21-017"), None)
        assert dimetromon is not None, "Dimetromon should be on field after digivolve"
        # DP should include +3000 from Owen's effect
        # Dimetromon base DP = 4000, +3000 from Owen = 7000
        assert dimetromon.dp >= 7000, \
            f"Dimetromon should have at least 7000 DP (4000 base + 3000 Owen), got {dimetromon.dp}"

    def test_digivolve_observer_suspends_owen(self, debug_runner):
        """Owen should be suspended after the digivolve observer fires."""
        deck = ["BT24-082"] * 4 + ["BT1-010"] * 23 + ["BT21-017"] * 23
        runner = debug_runner(deck1=deck, deck2=OWEN_DECK, initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["BT24-082"], turn_played=-1)
        runner.place_on_field(1, ["BT1-010"], turn_played=-1)
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT21-017", "hand")

        action = runner.find_action("Digivolve")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        owen = next((s for s in snap.p1_field if s.card_id == "BT24-082"), None)
        assert owen is not None, "Owen should still be on field"
        assert owen.is_suspended, "Owen should be suspended (cost of observer effect)"

    def test_digivolve_observer_not_triggered_for_non_reptile_dragonkin(self, debug_runner):
        """Digivolving into non-Reptile/Dragonkin should NOT trigger Owen."""
        # ST1-03 Biyomon: Lv3 Bird — not Reptile or Dragonkin
        # ST1-06 Birdramon: Lv4 Bird — not Reptile or Dragonkin
        deck = ["BT24-082"] * 4 + ["ST1-03"] * 23 + ["ST1-06"] * 23
        runner = debug_runner(deck1=deck, deck2=OWEN_DECK, initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["BT24-082"], turn_played=-1)
        runner.place_on_field(1, ["ST1-03"], turn_played=-1)
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-06", "hand")

        action = runner.find_action("Digivolve")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

        snap = runner.snapshot()
        owen = next((s for s in snap.p1_field if s.card_id == "BT24-082"), None)
        if owen is not None:
            assert not owen.is_suspended, \
                "Owen should NOT be suspended — digivolved into non-Reptile/Dragonkin"

    def test_digivolve_observer_skips_when_owen_already_suspended(self, debug_runner):
        """Owen should NOT trigger if already suspended."""
        deck = ["BT24-082"] * 4 + ["BT1-010"] * 23 + ["BT21-017"] * 23
        runner = debug_runner(deck1=deck, deck2=OWEN_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Place Owen as SUSPENDED
        runner.place_on_field(1, ["BT24-082"], is_suspended=True, turn_played=-1)
        runner.place_on_field(1, ["BT1-010"], turn_played=-1)
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT21-017", "hand")

        snap_before = runner.snapshot()

        action = runner.find_action("Digivolve")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        dimetromon = next((s for s in snap.p1_field if s.card_id == "BT21-017"), None)
        assert dimetromon is not None
        # Without Owen's +3000, Dimetromon should be at its base 4000 DP
        assert dimetromon.dp == 4000, \
            f"Dimetromon should have base 4000 DP (Owen suspended, no boost), got {dimetromon.dp}"

    # ── Effect 2: Security play ──────────────────────────────────────

    def test_security_plays_owen_on_field(self, debug_runner):
        """When revealed from security, Owen should be played onto the field."""
        runner = debug_runner(deck1=OWEN_DECK, deck2=OWEN_DECK, initial_memory=10)
        runner.set_phase("Main")

        game = runner.game

        # Clear P2 security and place Owen as the only security card
        game.player2.security_cards.clear()
        runner.inject_card(2, "BT24-082", "security_top")

        # Place a strong attacker on P1's field (no summoning sickness)
        runner.place_on_field(1, ["ST1-10"], turn_played=-1)  # Phoenixmon 12000 DP

        action = runner.find_action("Attack player with Phoenixmon")
        assert action is not None, f"Should find attack action. Actions: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Owen should be on P2's field (played from security)
        assert any(s.card_id == "BT24-082" for s in snap.p2_field), \
            "Owen should be played onto P2's field from security"
