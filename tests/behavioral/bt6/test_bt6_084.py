"""Behavioral tests for BT6-084 Sistermon Ciel (Digimon, Lv.4, Yellow, DP 5000, Cost 4).

Card text:
[All Turns] All of your Digimon with [Huckmon] in their name or [Royal Knight]
    trait get +2000 DP.
[On Play] Gain 1 memory.

Inherited: Same as above (both effects).
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


FILLER_DECK = ["BT6-084"] * 50


@pytest.mark.behavioral
class TestBT6084SistermonCiel:
    """Tests for BT6-084 Sistermon Ciel."""

    # ── DP Aura: Huckmon ────────────────────────────────────────────

    def test_dp_aura_huckmon_gets_plus_2000(self, debug_runner):
        """Huckmon on field should get +2000 DP from Sistermon Ciel's aura."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT6-084"])  # Sistermon Ciel (aura source)
        runner.place_on_field(1, ["BT6-009"])  # Huckmon (aura target)

        snap = runner.snapshot()
        huckmon = [f for f in snap.p1_field if f.card_id == "BT6-009"][0]
        # BT6-009 Huckmon base DP is 2000, should get +2000 = 4000
        assert huckmon.dp is not None
        assert huckmon.dp >= 4000, (
            f"Huckmon should have at least 4000 DP (2000 + 2000 aura), got {huckmon.dp}"
        )

    def test_dp_aura_royal_knight_gets_plus_2000(self, debug_runner):
        """Royal Knight trait Digimon should get +2000 DP from Sistermon Ciel's aura."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT6-084"])  # Sistermon Ciel (aura source)
        omnimon = runner.place_on_field(1, ["BT1-084"])  # Omnimon (Royal Knight trait)

        snap = runner.snapshot()
        omni = [f for f in snap.p1_field if f.card_id == "BT1-084"][0]
        # BT1-084 Omnimon base DP is 15000, should get +2000 = 17000
        assert omni.dp is not None
        assert omni.dp >= 17000, (
            f"Omnimon (Royal Knight) should have at least 17000 DP (15000 + 2000), got {omni.dp}"
        )

    # ── DP Aura: Negative tests ─────────────────────────────────────

    def test_dp_aura_no_effect_on_non_matching(self, debug_runner):
        """Non-Huckmon, non-Royal-Knight Digimon should NOT get +2000 DP."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT6-084"])  # Sistermon Ciel
        runner.place_on_field(1, ["ST1-03"])   # Agumon (not Huckmon, not RK)

        snap = runner.snapshot()
        agumon = [f for f in snap.p1_field if f.card_id == "ST1-03"][0]
        # ST1-03 Agumon base DP is 2000, should NOT get any aura bonus
        assert agumon.dp is not None
        assert agumon.dp == 2000, (
            f"Agumon should have 2000 DP (no aura), got {agumon.dp}"
        )

    def test_dp_aura_does_not_apply_to_opponent(self, debug_runner):
        """Aura should not apply to opponent's Digimon."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT6-084"])  # Sistermon Ciel on P1 field
        runner.place_on_field(2, ["BT6-009"])  # Huckmon on P2 (opponent) field

        snap = runner.snapshot()
        opp_huckmon = [f for f in snap.p2_field if f.card_id == "BT6-009"][0]
        # Opponent's Huckmon base DP is 2000, should NOT get aura
        assert opp_huckmon.dp is not None
        assert opp_huckmon.dp == 2000, (
            f"Opponent's Huckmon should have 2000 DP (no aura from your Ciel), "
            f"got {opp_huckmon.dp}"
        )

    # ── DP Aura: Both turns ─────────────────────────────────────────

    def test_dp_aura_active_on_opponent_turn(self, debug_runner):
        """DP aura should be active on opponent's turn (All Turns)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT6-084"])
        runner.place_on_field(1, ["BT6-009"])

        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        snap = runner.snapshot()
        huckmon = [f for f in snap.p1_field if f.card_id == "BT6-009"][0]
        assert huckmon.dp >= 4000, (
            f"Huckmon DP aura should be active on opponent's turn, got {huckmon.dp}"
        )

    # ── DP modifier effect structure ────────────────────────────────

    def test_dp_effect_has_correct_modifier_value(self, debug_runner):
        """DP modifier should be +2000."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()

        cs = db.create_card_source("BT6-084", runner.game.player1)
        effects = cs.effect_list(None)
        dp_effects = [
            e for e in effects
            if e.dp_modifier != 0
        ]
        assert len(dp_effects) >= 1, "Should have at least one DP modifier effect"
        assert dp_effects[0].dp_modifier == 2000, (
            f"DP modifier should be +2000, got {dp_effects[0].dp_modifier}"
        )

    def test_dp_effect_applies_to_all_own_digimon_flag(self, debug_runner):
        """DP aura effect should have _applies_to_all_own_digimon=True."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()

        cs = db.create_card_source("BT6-084", runner.game.player1)
        effects = cs.effect_list(None)
        dp_effects = [
            e for e in effects if e.dp_modifier != 0
        ]
        assert len(dp_effects) >= 1
        assert getattr(dp_effects[0], '_applies_to_all_own_digimon', False), (
            "DP aura effect should have _applies_to_all_own_digimon=True"
        )

    # ── On Play: Gain 1 memory ──────────────────────────────────────

    def test_on_play_gain_1_memory(self, debug_runner):
        """[On Play] should gain 1 memory."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()

        cs = db.create_card_source("BT6-084", runner.game.player1)
        effects = cs.effect_list(None)
        on_play_effects = [
            e for e in effects
            if getattr(e, 'is_on_play', False)
            and e.on_process_callback is not None
        ]
        assert len(on_play_effects) >= 1, "Should have On Play effect"

        # Simulate on-play: put card on field first
        perm = runner.place_on_field(1, ["BT6-084"])
        memory_before = runner.game.memory

        # Execute On Play process callback
        on_play_effects[0].on_process_callback({
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': perm,
        })

        assert runner.game.memory == memory_before + 1, (
            f"Memory should increase by 1, was {memory_before}, now {runner.game.memory}"
        )

    def test_on_play_effect_has_field_condition(self, debug_runner):
        """On Play condition should require card to be on field."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()

        cs = db.create_card_source("BT6-084", runner.game.player1)
        effects = cs.effect_list(None)
        on_play_effects = [
            e for e in effects if getattr(e, 'is_on_play', False)
        ]
        assert len(on_play_effects) >= 1

        # When in hand (not on field), condition should be False
        result = on_play_effects[0].can_use_condition({})
        assert result is False, (
            "On Play condition should be False when card is not on field"
        )
