"""Behavioral tests for EX11-005 Yaamon (Digi-Egg, Purple, Lesser/LIBERATOR).

Card text:
Inherited Effect [Start of Your Main Phase] This Digimon may digivolve into
a [Dark Dragon] or [Evil Dragon] trait Digimon card in the trash with the
digivolution cost reduced by 1. If this effect digivolve, trash 2 cards in
your hand.

Key behaviors:
- Inherited only (Digi-Egg)
- Digivolve from TRASH only (not hand)
- Must be a Digimon with [Dark Dragon] or [Evil Dragon] trait
- Must satisfy standard digivolve legality (level/color from evo_costs)
- Cost = play_cost - 1 (minimum 0)
- On success: trash 2 cards from hand (mandatory)
- Effect is optional (player may decline)

Test flow:
  The inherited effect fires at OnStartMainPhase timing. The engine calls
  execute_effects(OnStartMainPhase) during phase_main(). We trigger it
  manually via game.execute_effects() then resolve the selection phases.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase
from engine_py_legacy.engine.game.constants import SEL_TRASH_START


# Minimal decks for testing
YAAMON_DECK = ["EX11-005"] * 4 + ["BT2-013"] * 46  # Growlmon Lv.4 Red Dark Dragon


@pytest.mark.behavioral
class TestEX11005DigivolveFromTrash:
    """Inherited [Start of Your Main Phase] -- digivolve from trash."""

    def _setup_board(self, debug_runner, top_card_id="BT1-009", trash_card_id="BT2-013",
                     hand_count=3, hand_card_id="BT2-013"):
        """Helper: create a runner with Yaamon inherited stack on field + card in trash."""
        runner = debug_runner(
            deck1=YAAMON_DECK, deck2=YAAMON_DECK, initial_memory=10
        )
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.core.permanent import Permanent

        db = CardDatabase()
        p1 = runner.game.player1

        yaamon_cs = db.create_card_source("EX11-005", p1)
        top_cs = db.create_card_source(top_card_id, p1)
        perm = Permanent([yaamon_cs, top_cs])
        p1.battle_area.append(perm)

        if trash_card_id:
            trash_cs = db.create_card_source(trash_card_id, p1)
            p1.trash_cards.append(trash_cs)

        p1.hand_cards.clear()
        for _ in range(hand_count):
            cs = db.create_card_source(hand_card_id, p1)
            p1.hand_cards.append(cs)

        return runner, perm

    def test_digivolve_from_trash_dark_dragon(self, debug_runner):
        """Should digivolve into a Dark Dragon Digimon (BT2-013 Growlmon) from trash.

        Board: Yaamon + BT1-009 (Monodramon Lv.3 Red) on field.
        Trash: BT2-013 (Growlmon Lv.4 Red Dark Dragon, evo from Lv.3 Red).
        Expected: Growlmon added to stack, cost 1 (evo_cost 2 - 1), draw 1, trash 2.
        """
        runner, perm = self._setup_board(debug_runner)
        p1 = runner.game.player1

        hand_before = len(p1.hand_cards)
        memory_before = runner.game.memory

        # Trigger Start of Main Phase
        runner.set_phase("Main")
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        # Should be in SelectTarget phase for choosing trash card
        assert runner.game.current_phase == GamePhase.SelectTarget, \
            f"Expected SelectTarget phase, got {runner.game.current_phase}"

        # Select Growlmon from trash (index 0)
        runner.execute(SEL_TRASH_START + 0)

        # Should now be in SelectHand phase for trashing cards
        assert runner.game.current_phase == GamePhase.SelectHand, \
            f"Expected SelectHand phase, got {runner.game.current_phase}"

        # Auto-resolve trashing 2 cards from hand
        runner.auto_resolve()

        # Verify digivolve: Growlmon is now the top card
        assert perm.top_card is not None
        assert perm.top_card.c_entity_base.card_id == "BT2-013", \
            f"Top card should be BT2-013, got {perm.top_card.c_entity_base.card_id}"

        # Verify stack: [EX11-005, BT1-009, BT2-013]
        stack_ids = [cs.c_entity_base.card_id for cs in perm.card_sources]
        assert "BT2-013" in stack_ids, f"Growlmon should be in stack: {stack_ids}"

        # Verify cost: evo_cost 2 - 1 = 1
        assert runner.game.memory == memory_before - 1, \
            f"Memory should be {memory_before - 1} (evo_cost 2 - 1 = 1), got {runner.game.memory}"

        # Verify hand: started with 3, drew 1, trashed 2 = 2
        assert len(p1.hand_cards) == hand_before + 1 - 2, \
            f"Hand should have {hand_before + 1 - 2} cards, got {len(p1.hand_cards)}"

    def test_digivolve_from_trash_evil_dragon(self, debug_runner):
        """Should digivolve into an Evil Dragon Digimon (BT3-081 Devidramon) from trash.

        Board: Yaamon + BT2-069 (Gabumon Lv.3 Purple) on field.
        Trash: BT3-081 (Devidramon Lv.4 Purple Evil Dragon, evo from Lv.3 Purple).
        """
        runner, perm = self._setup_board(
            debug_runner,
            top_card_id="BT2-069",      # Gabumon Lv.3 Purple
            trash_card_id="BT3-081",     # Devidramon Lv.4 Purple Evil Dragon
        )
        p1 = runner.game.player1

        runner.set_phase("Main")
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        # Should be in SelectTarget phase (qualifying card in trash)
        assert runner.game.current_phase == GamePhase.SelectTarget, \
            f"Expected SelectTarget phase, got {runner.game.current_phase}"

        # Select Devidramon from trash
        runner.execute(SEL_TRASH_START + 0)
        runner.auto_resolve()

        # Verify Devidramon is top card
        assert perm.top_card.c_entity_base.card_id == "BT3-081", \
            f"Top card should be BT3-081, got {perm.top_card.c_entity_base.card_id}"

    def test_no_activation_when_no_qualifying_cards_in_trash(self, debug_runner):
        """Effect should not activate when no Dark Dragon/Evil Dragon in trash."""
        runner, perm = self._setup_board(
            debug_runner, trash_card_id=None  # Empty trash
        )
        p1 = runner.game.player1
        p1.trash_cards.clear()

        runner.set_phase("Main")
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        # Should still be in Main phase (no selection phase created)
        assert runner.game.current_phase == GamePhase.Main, \
            f"Should stay in Main phase when no qualifying trash, got {runner.game.current_phase}"

    def test_no_activation_for_non_matching_trait(self, debug_runner):
        """Non-Dark Dragon/Evil Dragon Digimon in trash should not trigger the effect."""
        runner, perm = self._setup_board(debug_runner, trash_card_id=None)
        p1 = runner.game.player1
        p1.trash_cards.clear()

        # Add a non-qualifying Digimon to trash (BT1-009 Monodramon = Mini Dragon)
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        non_dragon = db.create_card_source("BT1-009", p1)
        p1.trash_cards.append(non_dragon)

        runner.set_phase("Main")
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        assert runner.game.current_phase == GamePhase.Main, \
            f"Should stay in Main phase for non-qualifying trait, got {runner.game.current_phase}"

    def test_effect_is_optional_can_decline(self, debug_runner):
        """Player should be able to decline the digivolve (pass action)."""
        runner, perm = self._setup_board(debug_runner)
        p1 = runner.game.player1

        memory_before = runner.game.memory
        stack_before = [cs.c_entity_base.card_id for cs in perm.card_sources]

        runner.set_phase("Main")
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        assert runner.game.current_phase == GamePhase.SelectTarget

        # Find pass action (action 62 = Pass turn, which serves as decline)
        actions = runner.actions()
        # Action 0 is pass in selection phases, or there may be a decline action
        pass_action = None
        for aid, desc in actions.items():
            if "decline" in desc.lower() or "pass" in desc.lower():
                pass_action = aid
                break
        assert pass_action is not None, f"Should have a pass/decline action: {actions}"

        runner.execute(pass_action)

        # Stack should be unchanged
        stack_after = [cs.c_entity_base.card_id for cs in perm.card_sources]
        assert stack_after == stack_before, "Stack should be unchanged after declining"

        # Memory should be unchanged
        assert runner.game.memory == memory_before, "Memory should be unchanged after declining"

    def test_digivolve_legality_check(self, debug_runner):
        """Should not offer a card that can't legally digivolve onto the permanent.

        BT2-013 (Growlmon Lv.4 Red, evo from Lv.3 Red) cannot digivolve onto
        a Purple Lv.3 Digimon because the color doesn't match.
        """
        runner, perm = self._setup_board(
            debug_runner,
            top_card_id="BT2-069",      # Gabumon Lv.3 Purple
            trash_card_id="BT2-013",     # Growlmon Lv.4 Red (evo from Lv.3 RED, not Purple)
        )
        p1 = runner.game.player1

        runner.set_phase("Main")
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        # Should stay in Main (Growlmon can't digivolve onto Purple Lv.3)
        assert runner.game.current_phase == GamePhase.Main, \
            f"Should stay in Main when evo requirements don't match, got {runner.game.current_phase}"


@pytest.mark.behavioral
class TestEX11005TrashFromHand:
    """After successful digivolve, must trash 2 cards from hand."""

    def _setup_and_digivolve(self, debug_runner, hand_count=3):
        """Helper: set up board, trigger effect, select Growlmon from trash."""
        runner = debug_runner(
            deck1=YAAMON_DECK, deck2=YAAMON_DECK, initial_memory=10
        )
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.core.permanent import Permanent

        db = CardDatabase()
        p1 = runner.game.player1

        yaamon_cs = db.create_card_source("EX11-005", p1)
        top_cs = db.create_card_source("BT1-009", p1)
        perm = Permanent([yaamon_cs, top_cs])
        p1.battle_area.append(perm)

        growlmon_cs = db.create_card_source("BT2-013", p1)
        p1.trash_cards.append(growlmon_cs)

        p1.hand_cards.clear()
        for _ in range(hand_count):
            cs = db.create_card_source("BT2-013", p1)
            p1.hand_cards.append(cs)

        runner.set_phase("Main")
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        # Select Growlmon from trash
        runner.execute(SEL_TRASH_START + 0)

        return runner, perm

    def test_trash_two_from_hand_after_digivolve(self, debug_runner):
        """After digivolving, should trash exactly 2 cards from hand."""
        runner, perm = self._setup_and_digivolve(debug_runner, hand_count=3)
        p1 = runner.game.player1

        # After digivolve: 3 hand + 1 draw = 4 in hand
        assert len(p1.hand_cards) == 4, f"Should have 4 in hand after draw, got {len(p1.hand_cards)}"

        # Should be in SelectHand for trashing
        assert runner.game.current_phase == GamePhase.SelectHand

        # Auto-resolve trashing 2 cards
        runner.auto_resolve()

        # 4 in hand - 2 trashed = 2
        assert len(p1.hand_cards) == 2, \
            f"Should have 2 cards after trashing 2, got {len(p1.hand_cards)}"

    def test_trash_fewer_if_hand_has_one_card(self, debug_runner):
        """If hand has only 1 card after draw, trash only 1."""
        runner, perm = self._setup_and_digivolve(debug_runner, hand_count=0)
        p1 = runner.game.player1

        # After digivolve: 0 hand + 1 draw = 1 in hand
        assert len(p1.hand_cards) == 1, f"Should have 1 in hand after draw, got {len(p1.hand_cards)}"

        # Auto-resolve
        runner.auto_resolve()

        # 1 in hand - 1 trashed = 0
        assert len(p1.hand_cards) == 0, \
            f"Should have 0 cards after trashing 1 (all available), got {len(p1.hand_cards)}"

    def test_trash_nothing_if_hand_empty_after_draw(self, debug_runner):
        """If hand is empty after draw (impossible with draw, but edge case)."""
        runner = debug_runner(
            deck1=YAAMON_DECK, deck2=YAAMON_DECK, initial_memory=10
        )
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.core.permanent import Permanent

        db = CardDatabase()
        p1 = runner.game.player1

        yaamon_cs = db.create_card_source("EX11-005", p1)
        top_cs = db.create_card_source("BT1-009", p1)
        perm = Permanent([yaamon_cs, top_cs])
        p1.battle_area.append(perm)

        growlmon_cs = db.create_card_source("BT2-013", p1)
        p1.trash_cards.append(growlmon_cs)

        # Empty hand AND empty library (so draw fails)
        p1.hand_cards.clear()
        p1.library_cards.clear()

        runner.set_phase("Main")
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        # Select Growlmon
        runner.execute(SEL_TRASH_START + 0)

        # Auto-resolve (no hand cards to trash)
        runner.auto_resolve()

        # Should be back in Main
        assert runner.game.current_phase == GamePhase.Main or runner.game.game_over, \
            f"Should be in Main or game over, got {runner.game.current_phase}"


@pytest.mark.behavioral
class TestEX11005CostReduction:
    """Digivolution cost = play_cost - 1 (minimum 0)."""

    def test_cost_is_evo_cost_minus_one(self, debug_runner):
        """BT2-013 Growlmon evo_cost=2, so digivolve should cost 1."""
        runner = debug_runner(
            deck1=YAAMON_DECK, deck2=YAAMON_DECK, initial_memory=10
        )
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.core.permanent import Permanent

        db = CardDatabase()
        p1 = runner.game.player1

        yaamon_cs = db.create_card_source("EX11-005", p1)
        top_cs = db.create_card_source("BT1-009", p1)
        perm = Permanent([yaamon_cs, top_cs])
        p1.battle_area.append(perm)

        growlmon_cs = db.create_card_source("BT2-013", p1)
        p1.trash_cards.append(growlmon_cs)

        p1.hand_cards.clear()
        for _ in range(3):
            cs = db.create_card_source("BT2-013", p1)
            p1.hand_cards.append(cs)

        memory_before = runner.game.memory  # 10

        runner.set_phase("Main")
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        # Select Growlmon from trash
        runner.execute(SEL_TRASH_START + 0)
        runner.auto_resolve()

        # evo_cost of BT2-013 = 2, reduced by 1 = 1
        assert runner.game.memory == memory_before - 1, \
            f"Memory should be {memory_before - 1} (10 - 1), got {runner.game.memory}"


@pytest.mark.behavioral
class TestEX11005InheritedOnly:
    """EX11-005 is a Digi-Egg -- only inherited effects."""

    def test_is_inherited_effect(self, debug_runner):
        """The effect should be marked as inherited."""
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()

        cs = db.create_card_source("EX11-005", None)
        assert cs is not None

        effects = cs.effect_list(EffectTiming.OnStartMainPhase)
        inherited_effects = [e for e in effects if getattr(e, 'is_inherited_effect', False)]

        assert len(inherited_effects) >= 1, \
            "EX11-005 should have at least 1 inherited OnStartMainPhase effect"

    def test_only_fires_as_inherited(self, debug_runner):
        """Effect should only fire when Yaamon is in the digivolution sources
        (not as the top card). Digi-Eggs on their own don't have main effects."""
        runner = debug_runner(
            deck1=YAAMON_DECK, deck2=YAAMON_DECK, initial_memory=10
        )
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.core.permanent import Permanent

        db = CardDatabase()
        p1 = runner.game.player1

        # Place Yaamon as the ONLY card (top card) -- effect should NOT fire
        # because inherited effects only fire from non-top sources
        yaamon_cs = db.create_card_source("EX11-005", p1)
        perm = Permanent([yaamon_cs])
        p1.battle_area.append(perm)

        growlmon_cs = db.create_card_source("BT2-013", p1)
        p1.trash_cards.append(growlmon_cs)

        runner.set_phase("Main")
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        # Should stay in Main (inherited effect should NOT fire from top card)
        assert runner.game.current_phase == GamePhase.Main, \
            f"Inherited effect should not fire from top card, got {runner.game.current_phase}"
