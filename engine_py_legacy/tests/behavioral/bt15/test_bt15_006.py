"""Behavioral tests for BT15-006 DemiMeramon (Lv.2 Purple Digi-Egg).

Card text:
  Inherited [On Deletion] By trashing 1 level 5 or higher Digimon card
  in your hand, <Draw 2>.

Key behaviors:
  - Inherited effect only (Digi-Egg)
  - On Deletion timing (OnDestroyedAnyone)
  - Optional effect (player may decline)
  - COST: trash 1 Lv.5+ Digimon card from hand
  - REWARD: Draw 2 cards
  - Draw only happens if a card was actually trashed (pay-before-reward)
  - Must check both level >= 5 AND card is a Digimon
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT15006InheritedFlags:
    """Verify effect metadata: inherited, on_deletion, timing, optional."""

    def test_effect_is_inherited(self, debug_runner):
        """Effect must be flagged as inherited (Digi-Egg)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-006", "ST1-03"])

        # Get effects from the digi-egg card (bottom of stack)
        egg_card = perm.card_sources[0]
        assert egg_card.c_entity_base.card_id == "BT15-006"
        effects = egg_card.effect_list(None)
        od_effects = [e for e in effects if getattr(e, 'is_on_deletion', False)]
        assert len(od_effects) >= 1, "BT15-006 should have an On Deletion effect"
        assert od_effects[0].is_inherited_effect, "Effect must be inherited"

    def test_effect_timing_on_destroyed(self, debug_runner):
        """Effect timing should be OnDestroyedAnyone."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-006", "ST1-03"])

        egg_card = perm.card_sources[0]
        effects = egg_card.effect_list(None)
        od_effects = [e for e in effects if getattr(e, 'is_on_deletion', False)]
        assert od_effects[0].timing == EffectTiming.OnDestroyedAnyone

    def test_effect_is_optional(self, debug_runner):
        """Effect should be optional (player can decline)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-006", "ST1-03"])

        egg_card = perm.card_sources[0]
        effects = egg_card.effect_list(None)
        od_effects = [e for e in effects if getattr(e, 'is_on_deletion', False)]
        assert od_effects[0].is_optional, "Effect should be optional"


@pytest.mark.behavioral
class TestBT15006HandFilter:
    """Verify the hand filter requires Lv.5+ Digimon cards."""

    def test_filter_accepts_lv5_digimon(self, debug_runner):
        """A level 5 Digimon card should pass the hand filter."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-006", "ST1-03"])

        egg_card = perm.card_sources[0]
        effects = egg_card.effect_list(None)
        od_effect = [e for e in effects if getattr(e, 'is_on_deletion', False)][0]

        game = runner.game
        player = game.player1

        # Inject a Lv.5 Digimon into hand
        runner.inject_card(1, "ST1-09", "hand")  # MetalGreymon, Lv.5

        # Check the filter by invoking the callback and tracking what gets selected
        selected_cards = []
        original_select = game.effect_select_hand_card

        def mock_select(p, filter_fn, callback, is_optional=False, prompt=None):
            for card in p.hand_cards:
                if filter_fn(card):
                    selected_cards.append(card)
            # Don't actually select — just record
        game.effect_select_hand_card = mock_select

        od_effect.on_process_callback({
            'player': player, 'game': game, 'permanent': perm,
        })

        game.effect_select_hand_card = original_select

        lv5_ids = [c.c_entity_base.card_id for c in selected_cards]
        assert "ST1-09" in lv5_ids, "Lv.5 Digimon should pass the hand filter"

    def test_filter_rejects_lv4_digimon(self, debug_runner):
        """A level 4 Digimon card should be rejected by the hand filter."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-006", "ST1-03"])

        egg_card = perm.card_sources[0]
        effects = egg_card.effect_list(None)
        od_effect = [e for e in effects if getattr(e, 'is_on_deletion', False)][0]

        game = runner.game
        player = game.player1

        # Clear hand and inject only a Lv.4 Digimon
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-05", "hand")  # Greymon, Lv.4

        selected_cards = []

        def mock_select(p, filter_fn, callback, is_optional=False, prompt=None):
            for card in p.hand_cards:
                if filter_fn(card):
                    selected_cards.append(card)
        game.effect_select_hand_card = mock_select

        od_effect.on_process_callback({
            'player': player, 'game': game, 'permanent': perm,
        })

        assert len(selected_cards) == 0, "Lv.4 Digimon should NOT pass the hand filter"

    def test_filter_rejects_non_digimon_lv5(self, debug_runner):
        """A level 5+ non-Digimon card (e.g., Option or Tamer) should be rejected.

        C# reference checks IsDigimon explicitly. The filter must not accept
        cards that happen to have level >= 5 but are not Digimon type.
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-006", "ST1-03"])

        egg_card = perm.card_sources[0]
        effects = egg_card.effect_list(None)
        od_effect = [e for e in effects if getattr(e, 'is_on_deletion', False)][0]

        game = runner.game
        player = game.player1

        # Clear hand and inject a Tamer card (no level/not a Digimon)
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT1-085", "hand")  # Tai Kamiya Tamer

        selected_cards = []

        def mock_select(p, filter_fn, callback, is_optional=False, prompt=None):
            for card in p.hand_cards:
                if filter_fn(card):
                    selected_cards.append(card)
        game.effect_select_hand_card = mock_select

        od_effect.on_process_callback({
            'player': player, 'game': game, 'permanent': perm,
        })

        assert len(selected_cards) == 0, (
            "Tamer/Option cards should NOT pass the filter even if they had level >= 5"
        )


@pytest.mark.behavioral
class TestBT15006DrawOnlyAfterTrash:
    """Draw 2 must only happen AFTER successfully trashing a card.

    The C# reference uses a `discarded` boolean flag: the draw only fires
    if the player actually selected and trashed a Lv.5+ Digimon card.
    If the player declines (optional) or has no valid targets, no draw.
    """

    def test_draw_2_after_trashing_lv5_digimon(self, debug_runner):
        """Trashing a Lv.5+ Digimon from hand should Draw 2."""
        runner = debug_runner(initial_memory=5)
        # Stack: BT15-006 (egg) under a Lv.4 Digimon
        perm = runner.place_on_field(1, ["BT15-006", "ST1-05"])

        game = runner.game
        player = game.player1

        # Clear hand, inject exactly 1 Lv.5 Digimon
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-09", "hand")  # MetalGreymon Lv.5

        hand_before = len(player.hand_cards)
        deck_before = len(player.library_cards)

        # Trigger deletion of the permanent
        player.delete_permanent(perm)
        runner.auto_resolve()

        # After trash 1 from hand (-1) and draw 2 (+2), net hand change = +1
        hand_after = len(player.hand_cards)
        assert hand_after == hand_before - 1 + 2, (
            f"After trashing 1 and drawing 2, hand should go from {hand_before} to "
            f"{hand_before + 1}, got {hand_after}"
        )

    def test_no_draw_when_no_valid_targets_in_hand(self, debug_runner):
        """With no Lv.5+ Digimon in hand, no trash and no draw should occur."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-006", "ST1-05"])

        game = runner.game
        player = game.player1

        # Clear hand, inject only a Lv.3 Digimon (not valid)
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-03", "hand")  # Agumon Lv.3

        hand_before = len(player.hand_cards)
        deck_before = len(player.library_cards)

        player.delete_permanent(perm)
        runner.auto_resolve()

        hand_after = len(player.hand_cards)
        deck_after = len(player.library_cards)

        # No valid target -> no trash -> no draw
        assert hand_after == hand_before, (
            f"With no Lv.5+ Digimon in hand, hand should remain {hand_before}, "
            f"got {hand_after}"
        )
        assert deck_after == deck_before, (
            "Deck size should not change when no draw occurs"
        )

    def test_no_draw_when_hand_is_empty(self, debug_runner):
        """With an empty hand, the effect should not draw."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-006", "ST1-05"])

        game = runner.game
        player = game.player1

        runner.clear_zone(1, "hand")
        assert len(player.hand_cards) == 0

        deck_before = len(player.library_cards)

        player.delete_permanent(perm)
        runner.auto_resolve()

        deck_after = len(player.library_cards)
        assert deck_after == deck_before, (
            "With empty hand, deck should not change (no draw should occur)"
        )


@pytest.mark.behavioral
class TestBT15006FullDeletionFlow:
    """End-to-end deletion flow: inherited On Deletion triggers when
    the Digimon carrying DemiMeramon in its digi-sources is deleted."""

    def test_inherited_triggers_on_carrier_deletion(self, debug_runner):
        """When a Digimon with BT15-006 in its stack is deleted,
        the inherited On Deletion effect should trigger."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        # Place a Digimon stack with BT15-006 as inherited source
        perm = runner.place_on_field(1, ["BT15-006", "ST1-03", "ST1-05"])

        game = runner.game
        player = game.player1

        # Inject a Lv.5 Digimon into hand for trashing
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "ST1-09", "hand")

        hand_before = len(player.hand_cards)

        # Delete the permanent
        player.delete_permanent(perm)
        runner.auto_resolve()

        hand_after = len(player.hand_cards)
        # If effect triggered: -1 (trash) +2 (draw) = net +1
        assert hand_after == hand_before + 1, (
            f"Inherited On Deletion should trigger on carrier deletion. "
            f"Hand: {hand_before} -> {hand_after} (expected {hand_before + 1})"
        )
