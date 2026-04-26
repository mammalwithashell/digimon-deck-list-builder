"""Behavioral tests for BT22-084 Nokia Shiramine (Tamer Red/Blue, Cost 5).

Card text (from cards.json):
[Start of Your Turn] If you have 2 or less memory, set it to 3.
[Start of Your Main Phase] [On Play] If you have 1 or fewer Digimon,
    you may play 1 [Agumon] or [Gabumon] from your hand without paying the cost.
[All Turns] All your Digimon with [Greymon], [Garurumon] or [Omnimon]
    in their names get +1000 DP.
[Security] Play this card without paying the cost.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT22084NokiaShiramine:
    """Tests for BT22-084 Nokia Shiramine."""

    # ── [Start of Your Turn] Memory set to 3 if <= 2 ─────────────────

    def test_memory_set_to_3_when_at_2(self, debug_runner):
        """If memory is 2 at start of turn, it should become 3."""
        runner = debug_runner(initial_memory=2)
        runner.place_on_field(1, ["BT22-084"])
        game = runner.game

        card = None
        for p in game.player1.battle_area:
            if p.top_card and p.top_card.card_id == "BT22-084":
                card = p.top_card
                break
        assert card is not None

        effects = card.effect_list(None)
        sot_effects = [e for e in effects if e.timing == EffectTiming.OnStartTurn]
        assert len(sot_effects) == 1, "Should have 1 Start of Turn effect"

        sot = sot_effects[0]
        assert sot.can_use_condition({}), "Condition should pass on own turn"

        sot.on_process_callback({'player': game.player1, 'game': game})
        assert game.memory == 3, f"Memory should be set to 3, got {game.memory}"

    def test_memory_set_to_3_when_at_0(self, debug_runner):
        """If memory is 0 at start of turn, it should become 3."""
        runner = debug_runner(initial_memory=0)
        runner.place_on_field(1, ["BT22-084"])
        game = runner.game

        card = [p.top_card for p in game.player1.battle_area
                if p.top_card and p.top_card.card_id == "BT22-084"][0]

        sot = [e for e in card.effect_list(None)
               if e.timing == EffectTiming.OnStartTurn][0]

        sot.on_process_callback({'player': game.player1, 'game': game})
        assert game.memory == 3, f"Memory should be set to 3 when at 0, got {game.memory}"

    def test_memory_not_changed_when_at_3(self, debug_runner):
        """If memory is already 3 or more, it should NOT be changed."""
        runner = debug_runner(initial_memory=3)
        runner.place_on_field(1, ["BT22-084"])
        game = runner.game

        card = [p.top_card for p in game.player1.battle_area
                if p.top_card and p.top_card.card_id == "BT22-084"][0]

        sot = [e for e in card.effect_list(None)
               if e.timing == EffectTiming.OnStartTurn][0]

        sot.on_process_callback({'player': game.player1, 'game': game})
        assert game.memory == 3, f"Memory should stay at 3, got {game.memory}"

    def test_memory_not_changed_when_at_5(self, debug_runner):
        """If memory is 5, it should NOT be reduced to 3."""
        runner = debug_runner(initial_memory=5)
        runner.place_on_field(1, ["BT22-084"])
        game = runner.game

        card = [p.top_card for p in game.player1.battle_area
                if p.top_card and p.top_card.card_id == "BT22-084"][0]

        sot = [e for e in card.effect_list(None)
               if e.timing == EffectTiming.OnStartTurn][0]

        sot.on_process_callback({'player': game.player1, 'game': game})
        assert game.memory == 5, f"Memory should stay at 5, got {game.memory}"

    # ── [Start of Main] / [On Play] Play Agumon or Gabumon ────────────

    def test_start_main_play_agumon_from_hand(self, debug_runner):
        """[Start of Main Phase] Should allow playing Agumon from hand free
        when player has 1 or fewer Digimon."""
        runner = debug_runner(initial_memory=5)
        runner.place_on_field(1, ["BT22-084"])
        # Inject an Agumon into hand
        runner.inject_card(1, "BT1-010", "hand")  # Agumon Lv.3 dp=2000
        game = runner.game

        card = [p.top_card for p in game.player1.battle_area
                if p.top_card and p.top_card.card_id == "BT22-084"][0]

        effects = card.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase]
        assert len(main_effects) == 1, "Should have 1 Start of Main Phase effect"

        main_eff = main_effects[0]
        # Player has 0 Digimon (only a tamer), so condition should pass
        assert main_eff.can_use_condition({}), "Condition should pass on own turn"

        memory_before = game.memory
        main_eff.on_process_callback({'player': game.player1, 'game': game})
        runner.auto_resolve()

        # Should have played Agumon
        snap = runner.snapshot()
        agumon_on_field = any(
            s.card_id == "BT1-010" and s.is_digimon
            for s in snap.p1_field
        )
        assert agumon_on_field, "Agumon should be played from hand onto field"

        # Should be free (no memory cost)
        assert game.memory == memory_before, (
            f"Should play without paying cost, memory was {memory_before}, now {game.memory}"
        )

    def test_start_main_no_play_with_2_digimon(self, debug_runner):
        """Should NOT play from hand if player already has 2 Digimon."""
        runner = debug_runner(initial_memory=5)
        runner.place_on_field(1, ["BT22-084"])
        runner.place_on_field(1, ["BT1-010"])  # Digimon 1
        runner.place_on_field(1, ["BT1-012"])  # Digimon 2
        runner.inject_card(1, "BT1-029", "hand")  # Gabumon
        game = runner.game

        card = [p.top_card for p in game.player1.battle_area
                if p.top_card and p.top_card.card_id == "BT22-084"][0]

        main_eff = [e for e in card.effect_list(None)
                    if e.timing == EffectTiming.OnStartMainPhase][0]

        hand_before = len(game.player1.hand_cards)
        main_eff.on_process_callback({'player': game.player1, 'game': game})
        runner.auto_resolve()

        # Gabumon should still be in hand (condition failed: >1 Digimon)
        assert len(game.player1.hand_cards) == hand_before, (
            "No card should be played when player has 2 Digimon"
        )

    def test_on_play_effect_exists(self, debug_runner):
        """Should have an On Play effect with the same play logic."""
        runner = debug_runner(initial_memory=5)
        runner.place_on_field(1, ["BT22-084"])
        game = runner.game

        card = [p.top_card for p in game.player1.battle_area
                if p.top_card and p.top_card.card_id == "BT22-084"][0]

        effects = card.effect_list(None)
        on_play = [e for e in effects
                   if getattr(e, 'is_on_play', False)
                   and e.timing == EffectTiming.OnEnterFieldAnyone]
        assert len(on_play) >= 1, "Should have On Play effect"

    def test_play_filter_rejects_non_agumon_gabumon(self, debug_runner):
        """Play filter should reject Digimon that are not named Agumon or Gabumon."""
        runner = debug_runner(initial_memory=5)
        runner.place_on_field(1, ["BT22-084"])
        # Inject a non-Agumon/Gabumon Digimon
        runner.inject_card(1, "BT1-009", "hand")  # Monodramon
        game = runner.game

        card = [p.top_card for p in game.player1.battle_area
                if p.top_card and p.top_card.card_id == "BT22-084"][0]

        main_eff = [e for e in card.effect_list(None)
                    if e.timing == EffectTiming.OnStartMainPhase][0]

        hand_before = len(game.player1.hand_cards)
        main_eff.on_process_callback({'player': game.player1, 'game': game})
        runner.auto_resolve()

        # Monodramon should NOT be played (not Agumon or Gabumon)
        snap = runner.snapshot()
        monodramon = any(s.card_id == "BT1-009" for s in snap.p1_field)
        assert not monodramon, "Should not play non-Agumon/Gabumon cards"

    # ── [All Turns] DP +1000 to Greymon/Garurumon/Omnimon ────────────

    def test_dp_boost_greymon(self, debug_runner):
        """Digimon with [Greymon] in name should get +1000 DP."""
        runner = debug_runner(initial_memory=10)
        runner.place_on_field(1, ["BT22-084"])
        # AD1-001 Greymon = 5000 DP
        runner.place_on_field(1, ["AD1-001"])

        snap = runner.snapshot()
        greymon = [s for s in snap.p1_field if s.card_id == "AD1-001"][0]
        assert greymon.dp == 6000, (
            f"Greymon should have 6000 DP (5000 base + 1000 Nokia aura), got {greymon.dp}"
        )

    def test_dp_boost_garurumon(self, debug_runner):
        """Digimon with [Garurumon] in name should get +1000 DP."""
        runner = debug_runner(initial_memory=10)
        runner.place_on_field(1, ["BT22-084"])
        # AD1-010 Garurumon = 5000 DP
        runner.place_on_field(1, ["AD1-010"])

        snap = runner.snapshot()
        garurumon = [s for s in snap.p1_field if s.card_id == "AD1-010"][0]
        assert garurumon.dp == 6000, (
            f"Garurumon should have 6000 DP (5000 + 1000 aura), got {garurumon.dp}"
        )

    def test_dp_boost_omnimon(self, debug_runner):
        """Digimon with [Omnimon] in name should get +1000 DP."""
        runner = debug_runner(initial_memory=10)
        runner.place_on_field(1, ["BT22-084"])
        # BT22-015 Omnimon = 15000 DP
        runner.place_on_field(1, ["BT22-015"])

        snap = runner.snapshot()
        omnimon = [s for s in snap.p1_field if s.card_id == "BT22-015"][0]
        assert omnimon.dp == 16000, (
            f"Omnimon should have 16000 DP (15000 + 1000 aura), got {omnimon.dp}"
        )

    def test_dp_boost_not_applied_to_non_matching_name(self, debug_runner):
        """Digimon without Greymon/Garurumon/Omnimon in name gets no boost."""
        runner = debug_runner(initial_memory=10)
        runner.place_on_field(1, ["BT22-084"])
        # BT1-010 Agumon = 2000 DP (no matching name)
        runner.place_on_field(1, ["BT1-010"])

        snap = runner.snapshot()
        agumon = [s for s in snap.p1_field if s.card_id == "BT1-010"][0]
        assert agumon.dp == 2000, (
            f"Agumon should have base 2000 DP (no Nokia boost), got {agumon.dp}"
        )

    def test_dp_boost_not_applied_to_opponent(self, debug_runner):
        """Opponent's Greymon should NOT get the boost."""
        runner = debug_runner(initial_memory=10)
        runner.place_on_field(1, ["BT22-084"])
        # Place Greymon on opponent's field
        runner.place_on_field(2, ["AD1-001"])  # Greymon 5000 DP

        snap = runner.snapshot()
        opp_greymon = [s for s in snap.p2_field if s.card_id == "AD1-001"][0]
        assert opp_greymon.dp == 5000, (
            f"Opponent's Greymon should have base 5000 DP (no Nokia boost), "
            f"got {opp_greymon.dp}"
        )

    # ── Security effect ───────────────────────────────────────────────

    def test_security_effect_exists(self, debug_runner):
        """Should have a security play effect."""
        runner = debug_runner(initial_memory=10)
        runner.inject_card(1, "BT22-084", "hand")
        game = runner.game
        card = [c for c in game.player1.hand_cards if c.card_id == "BT22-084"][0]
        effects = card.effect_list(None)
        sec_effs = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effs) >= 1, "Should have at least 1 security effect"
