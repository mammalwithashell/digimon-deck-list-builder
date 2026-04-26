"""Behavioral tests for EX11-047 Impmon (Lv.3 Purple/Red Digimon).

Card text:
  Alt digivolve: from [Yaamon] for cost 0

  [Start of Your Main Phase] Trash 1 card in your hand. Then, gain 1 memory.

  --- Inherited ---
  [Your Turn] This Digimon gets +2000 DP.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming

_FILLER_DECK = ["ST1-02"] * 50


@pytest.mark.behavioral
class TestEX11047SoYMPTrashAndGainMemory:
    """Tests for [Start of Your Main Phase] Trash 1, gain 1 memory."""

    def test_somp_trash_one_gain_one_memory(self, debug_runner):
        """SoYMP: trash 1 hand card and gain 1 memory."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=0)

        runner.place_on_field(1, ["EX11-047"])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-02", "hand")

        game = runner.game
        hand_before = len(game.player1.hand_cards)
        assert hand_before == 1, f"Expected 1 card in hand, got {hand_before}"

        memory_before = game.memory

        # Trigger SoYMP effects
        runner.set_phase("Main")
        game.execute_effects(EffectTiming.OnStartMainPhase)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Hand should be empty (1 card trashed)
        assert len(snap.p1_hand) == 0, (
            f"Hand should be empty after trashing, got {snap.p1_hand}")
        # Card should be in trash
        assert "ST1-02" in snap.p1_trash, (
            f"Trashed card should be in trash, got {snap.p1_trash}")
        # Memory should have gained 1
        assert snap.memory == memory_before + 1, (
            f"Memory should be {memory_before + 1}, got {snap.memory}")

    def test_somp_empty_hand_still_gains_memory(self, debug_runner):
        """SoYMP: with empty hand, trash does nothing but memory is still gained.

        C# reference: memory gain is outside the hand-count check.
        Card text: 'Trash 1 card in your hand. Then, gain 1 memory.'
        The 'Then' means sequential, NOT conditional.
        """
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=0)

        runner.place_on_field(1, ["EX11-047"])
        runner.clear_zone(1, "hand")

        game = runner.game
        assert len(game.player1.hand_cards) == 0, "Hand should be empty"

        memory_before = game.memory

        # Trigger SoYMP effects
        runner.set_phase("Main")
        game.execute_effects(EffectTiming.OnStartMainPhase)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Memory should still gain 1 even with empty hand
        assert snap.memory == memory_before + 1, (
            f"Memory should be {memory_before + 1} even with empty hand, "
            f"got {snap.memory}")

    def test_somp_mandatory_trash(self, debug_runner):
        """SoYMP: trash is mandatory (not optional) — cannot decline."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=0)

        runner.place_on_field(1, ["EX11-047"])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-02", "hand")
        runner.inject_card(1, "BT1-009", "hand")

        game = runner.game
        hand_before = len(game.player1.hand_cards)
        assert hand_before == 2

        # Trigger SoYMP
        runner.set_phase("Main")
        game.execute_effects(EffectTiming.OnStartMainPhase)

        # Should be in selection phase (not auto-resolved, no decline)
        from engine_py_legacy.engine.data.enums import GamePhase
        if game.current_phase == GamePhase.SelectHand:
            # Check that the decline action (62) is NOT available
            legal = runner.action_mask()
            assert 62 not in legal, (
                "Decline (action 62) should NOT be available — trash is mandatory")

        runner.auto_resolve()

        snap = runner.snapshot()
        # Exactly 1 card should have been trashed
        assert len(snap.p1_hand) == 1, (
            f"Should have 1 card remaining in hand (trashed 1), got {len(snap.p1_hand)}")

    def test_somp_condition_requires_field(self, debug_runner):
        """SoYMP condition: effect only activates when Impmon is on the field."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=0)

        # Impmon in hand, NOT on field
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX11-047", "hand")
        runner.inject_card(1, "ST1-02", "hand")

        game = runner.game
        memory_before = game.memory

        # Get the SoYMP effect from the card in hand
        impmon_card = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "EX11-047":
                impmon_card = c
                break
        assert impmon_card is not None

        effects = impmon_card.effect_list(None)
        somp_effects = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase]
        assert len(somp_effects) == 1, "Should have 1 SoYMP effect"

        eff = somp_effects[0]
        # Condition should fail — card is in hand, not on field
        assert not eff.can_use_condition({}), (
            "SoYMP condition should fail when Impmon is not on field")

    def test_somp_condition_requires_own_turn(self, debug_runner):
        """SoYMP condition: effect only activates on owner's turn."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=0)

        runner.place_on_field(1, ["EX11-047"])

        game = runner.game

        # Get the SoYMP effect
        perm = [p for p in game.player1.battle_area
                if p.top_card and p.top_card.c_entity_base.card_id == "EX11-047"][0]
        effects = perm.top_card.effect_list(None)
        somp_effects = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase]
        eff = somp_effects[0]

        # On P1's turn — condition should pass
        game.player1.is_my_turn = True
        assert eff.can_use_condition({}), (
            "SoYMP condition should pass on owner's turn")

        # On P2's turn — condition should fail
        game.player1.is_my_turn = False
        assert not eff.can_use_condition({}), (
            "SoYMP condition should fail on opponent's turn")


@pytest.mark.behavioral
class TestEX11047InheritedDPBoost:
    """Tests for inherited [Your Turn] +2000 DP."""

    def test_inherited_dp_boost_own_turn(self, debug_runner):
        """Inherited: +2000 DP on owner's turn."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        # Stack: Impmon as inherited source under a Lv.4
        perm = runner.place_on_field(1, ["EX11-047", "BT1-019"])

        game = runner.game
        game.player1.is_my_turn = True

        dp_own_turn = perm.dp
        # DarkTyrannomon base DP is 6000; inherited +2000 should make it 8000
        assert dp_own_turn is not None, "Permanent should have DP"
        assert dp_own_turn >= 8000, (
            f"DP should be at least 8000 (6000 base + 2000 inherited) on own turn, "
            f"got {dp_own_turn}"
        )

    def test_inherited_dp_boost_not_on_opponent_turn(self, debug_runner):
        """Inherited: no DP boost on opponent's turn."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        # Stack: Impmon as inherited source under a Lv.4
        perm = runner.place_on_field(1, ["EX11-047", "BT1-019"])

        game = runner.game

        # Measure DP on own turn
        game.player1.is_my_turn = True
        dp_own_turn = perm.dp

        # Measure DP on opponent's turn
        game.player1.is_my_turn = False
        dp_opp_turn = perm.dp

        assert dp_own_turn == dp_opp_turn + 2000, (
            f"DP should be 2000 higher on own turn: own={dp_own_turn}, "
            f"opp={dp_opp_turn}")

    def test_inherited_is_marked_as_inherited(self, debug_runner):
        """Inherited effect has is_inherited_effect flag set."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        runner.place_on_field(1, ["EX11-047"])
        game = runner.game

        perm = game.player1.battle_area[0]
        effects = perm.top_card.effect_list(None)
        inherited_effects = [e for e in effects if e.is_inherited_effect]
        assert len(inherited_effects) == 1, (
            f"Should have exactly 1 inherited effect, got {len(inherited_effects)}")
        assert inherited_effects[0].dp_modifier == 2000, (
            f"Inherited DP modifier should be 2000, "
            f"got {inherited_effects[0].dp_modifier}")


@pytest.mark.behavioral
class TestEX11047AltDigivolve:
    """Tests for alternate digivolution from [Yaamon] for cost 0."""

    def test_alt_digi_attributes_set(self, debug_runner):
        """Alt-digi effect has correct _alt_digi_name and _alt_digi_cost."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        runner.inject_card(1, "EX11-047", "hand")
        game = runner.game

        impmon_card = None
        for c in game.player1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "EX11-047":
                impmon_card = c
                break
        assert impmon_card is not None

        effects = impmon_card.effect_list(None)
        alt_digi_effects = [e for e in effects
                            if getattr(e, '_alt_digi_name', None) is not None]
        assert len(alt_digi_effects) == 1, (
            f"Should have 1 alt-digi effect, got {len(alt_digi_effects)}")

        eff = alt_digi_effects[0]
        assert eff._alt_digi_name == "Yaamon", (
            f"Alt-digi name should be 'Yaamon', got '{eff._alt_digi_name}'")
        assert eff._alt_digi_cost == 0, (
            f"Alt-digi cost should be 0, got {eff._alt_digi_cost}")

    def test_alt_digi_from_yaamon(self, debug_runner):
        """Can alt-digivolve from Yaamon on field for cost 0."""
        runner = debug_runner(deck1=_FILLER_DECK, deck2=_FILLER_DECK,
                              initial_memory=5)

        # Place Yaamon (BT2-008) directly on field as a Lv.2
        runner.place_on_field(1, ["BT2-008"])

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX11-047", "hand")
        runner.set_phase("Main")

        digi = runner.find_action("Digivolve")
        assert digi is not None, (
            f"Should find digivolve action from Yaamon. Actions: {runner.actions()}")
