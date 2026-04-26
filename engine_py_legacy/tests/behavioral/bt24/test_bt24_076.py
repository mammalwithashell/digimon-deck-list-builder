"""Behavioral tests for BT24-076 WarGrowlmon (Lv.5, Purple, Cyborg).

Card text:
  [Trash] [Main] If you have 4 or fewer cards in your hand, play this
      card with the play cost reduced by 2.
  [On Play] [When Digivolving] Delete 1 of your opponent's level 4 or
      lower Digimon.
  Inherited: [On Deletion] You may play 1 level 4 or lower [Dark Dragon]
      or [Evil Dragon] Digimon card from your trash without paying the cost.

Clause breakdown:
  C1: [Trash][Main] condition: card in trash + hand <= 4 + owner's turn
  C2: [Trash][Main] process: play self from trash with cost -2
  C3: [On Play] delete 1 opp lv4 or lower Digimon (mandatory selection)
  C4: [When Digivolving] delete 1 opp lv4 or lower Digimon (mandatory)
  C5: Inherited [On Deletion] optionally play 1 lv4 or lower [Dark Dragon]
      or [Evil Dragon] Digimon from trash free
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


# Card IDs used in tests
BT24_076 = "BT24-076"       # Card under test (WarGrowlmon, Lv.5 Purple)
AGUMON = "ST1-03"            # Lv.3 Red Rookie (opponent target, lv <= 4)
GREYMON = "ST1-07"           # Lv.4 Red Champion (opponent target, lv <= 4)
METALGREYMON = "ST1-09"      # Lv.5 Red Ultimate (should NOT be deleted by effect)
GROWLMON = "BT2-013"         # Lv.4 Dark Dragon (eligible for inherited play)
DEVIDRAMON = "BT3-081"       # Lv.4 Evil Dragon (eligible for inherited play)
BT1_010 = "BT1-010"          # Lv.3 Red Rookie (not Dark/Evil Dragon)


# ── Clause 1: [Trash][Main] Condition ──────────────────────────────────


@pytest.mark.behavioral
class TestBT24076TrashMainCondition:
    """C1: [Trash][Main] must require card in trash, hand <= 4, owner's turn."""

    def test_trash_main_effect_exists(self, debug_runner):
        """Should have an OnDeclaration effect with _is_trash_main flag."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, BT24_076, "trash")
        effects = card.effect_list(None)
        trash_main = [
            e for e in effects if getattr(e, '_is_trash_main', False)
        ]
        assert len(trash_main) >= 1, "Should have a [Trash][Main] effect"
        assert trash_main[0].timing == EffectTiming.OnDeclaration

    def test_condition_passes_when_in_trash_hand_lte_4(self, debug_runner):
        """Condition should pass: card in trash, hand has <= 4 cards, own turn."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, BT24_076, "trash")
        runner.clear_zone(1, "hand")
        # Add exactly 4 cards to hand
        for _ in range(4):
            runner.inject_card(1, AGUMON, "hand")

        effects = card.effect_list(None)
        trash_main = [
            e for e in effects if getattr(e, '_is_trash_main', False)
        ][0]

        ctx = {'game': runner.game, 'player': runner.game.player1}
        assert trash_main.can_use_condition(ctx), (
            "Should pass with card in trash and hand <= 4")

    def test_condition_fails_when_hand_over_4(self, debug_runner):
        """Condition should fail when hand has > 4 cards."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, BT24_076, "trash")
        runner.clear_zone(1, "hand")
        # Add 5 cards to hand
        for _ in range(5):
            runner.inject_card(1, AGUMON, "hand")

        effects = card.effect_list(None)
        trash_main = [
            e for e in effects if getattr(e, '_is_trash_main', False)
        ][0]

        ctx = {'game': runner.game, 'player': runner.game.player1}
        assert not trash_main.can_use_condition(ctx), (
            "Should fail with hand > 4 cards")

    def test_condition_fails_when_not_in_trash(self, debug_runner):
        """Condition should fail when card is not in trash."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, BT24_076, "hand")
        runner.clear_zone(1, "hand")
        runner.game.player1.hand_cards.append(card)

        effects = card.effect_list(None)
        trash_main = [
            e for e in effects if getattr(e, '_is_trash_main', False)
        ][0]

        ctx = {'game': runner.game, 'player': runner.game.player1}
        assert not trash_main.can_use_condition(ctx), (
            "Should fail when card is in hand, not trash")

    def test_condition_fails_on_opponent_turn(self, debug_runner):
        """Condition should fail when it's not the owner's turn."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, BT24_076, "trash")
        runner.clear_zone(1, "hand")

        # Switch to P2's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        effects = card.effect_list(None)
        trash_main = [
            e for e in effects if getattr(e, '_is_trash_main', False)
        ][0]

        ctx = {'game': runner.game, 'player': runner.game.player1}
        assert not trash_main.can_use_condition(ctx), (
            "Should fail on opponent's turn")


# ── Clause 2: [Trash][Main] Process ───────────────────────────────────


@pytest.mark.behavioral
class TestBT24076TrashMainProcess:
    """C2: Playing from trash should move card to field."""

    def test_plays_card_from_trash_to_field(self, debug_runner):
        """Process should move BT24-076 from trash to the battle area."""
        runner = debug_runner(initial_memory=10)
        card = runner.inject_card(1, BT24_076, "trash")
        runner.clear_zone(1, "hand")
        runner.set_phase("Main")

        assert card in runner.game.player1.trash_cards
        field_before = len(runner.game.player1.battle_area)

        effects = card.effect_list(None)
        trash_main = [
            e for e in effects if getattr(e, '_is_trash_main', False)
        ][0]

        ctx = {
            'player': runner.game.player1,
            'game': runner.game,
        }
        trash_main.on_process_callback(ctx)

        assert card not in runner.game.player1.trash_cards, (
            "Card should no longer be in trash after playing")
        assert len(runner.game.player1.battle_area) == field_before + 1, (
            "Battle area should have 1 more permanent")

    def test_cost_reduction_effect_exists(self, debug_runner):
        """Should have a BeforePayCost effect for the cost reduction."""
        runner = debug_runner(initial_memory=10)
        card = runner.inject_card(1, BT24_076, "trash")
        effects = card.effect_list(None)
        bpc = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.BeforePayCost
        ]
        assert len(bpc) >= 1, "Should have a BeforePayCost cost reduction effect"
        assert getattr(bpc[0], 'cost_reduction', 0) == 2, (
            "Cost reduction should be 2")

    def test_cost_reduction_has_leak_guard(self, debug_runner):
        """BeforePayCost condition MUST check card_source is card (leak guard)."""
        runner = debug_runner(initial_memory=10)
        card = runner.inject_card(1, BT24_076, "trash")
        runner.clear_zone(1, "hand")

        effects = card.effect_list(None)
        bpc = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.BeforePayCost
        ][0]

        # With a different card_source, should return False (leak guard)
        other_card = runner.inject_card(1, AGUMON, "hand")
        assert not bpc.can_use_condition({'card_source': other_card}), (
            "BeforePayCost MUST reject different card_source (leak guard)")

        # With the correct card_source, should pass
        assert bpc.can_use_condition({'card_source': card}), (
            "BeforePayCost should pass for own card_source")


# ── Clause 3: [On Play] Delete opp lv4 or lower ──────────────────────


@pytest.mark.behavioral
class TestBT24076OnPlay:
    """C3: [On Play] Delete 1 of opponent's level 4 or lower Digimon."""

    def test_on_play_effect_exists(self, debug_runner):
        """Should have an OnEnterFieldAnyone effect with is_on_play."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [BT24_076])
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if getattr(e, 'is_on_play', False)]
        assert len(on_play) >= 1, "Should have an [On Play] effect"
        assert on_play[0].timing == EffectTiming.OnEnterFieldAnyone

    def test_on_play_condition_requires_field_presence(self, debug_runner):
        """Condition should check card is on the field."""
        runner = debug_runner(initial_memory=10)
        card = runner.inject_card(1, BT24_076, "hand")
        effects = card.effect_list(None)
        on_play = [e for e in effects if getattr(e, 'is_on_play', False)][0]

        # Card in hand, not on field — condition should fail
        assert not on_play.can_use_condition({}), (
            "Should fail when card is not on field")

        # Place on field
        perm = runner.place_on_field(1, [BT24_076])
        card2 = perm.top_card
        effects2 = card2.effect_list(None)
        on_play2 = [e for e in effects2 if getattr(e, 'is_on_play', False)][0]
        assert on_play2.can_use_condition({}), (
            "Should pass when card is on field")

    def test_on_play_deletes_lv4_or_lower_digimon(self, debug_runner):
        """Process should delete 1 opponent Digimon with level 4 or lower."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [BT24_076])
        opp_lv3 = runner.place_on_field(2, [AGUMON])   # Lv.3
        opp_lv4 = runner.place_on_field(2, [GREYMON])   # Lv.4
        game = runner.game

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if getattr(e, 'is_on_play', False)][0]

        opp_field_before = len(game.player2.battle_area)

        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # At least 1 opponent Digimon should have been deleted
        assert len(game.player2.battle_area) < opp_field_before, (
            "Should have deleted at least 1 opponent Digimon")

    def test_on_play_does_not_delete_lv5_or_higher(self, debug_runner):
        """Should NOT delete opponent Digimon with level 5 or higher."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [BT24_076])
        opp_lv5 = runner.place_on_field(2, [METALGREYMON])  # Lv.5
        game = runner.game

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if getattr(e, 'is_on_play', False)][0]

        opp_field_before = len(game.player2.battle_area)

        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # No Digimon should be deleted (only Lv.5 present)
        assert len(game.player2.battle_area) == opp_field_before, (
            "Should NOT delete opponent's Lv.5 Digimon")


# ── Clause 4: [When Digivolving] Delete opp lv4 or lower ─────────────


@pytest.mark.behavioral
class TestBT24076WhenDigivolving:
    """C4: [When Digivolving] Delete 1 of opponent's level 4 or lower Digimon."""

    def test_when_digivolving_effect_exists(self, debug_runner):
        """Should have an OnEnterFieldAnyone effect with is_when_digivolving."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [BT24_076])
        card = perm.top_card
        effects = card.effect_list(None)
        wd = [e for e in effects if getattr(e, 'is_when_digivolving', False)]
        assert len(wd) >= 1, "Should have a [When Digivolving] effect"
        assert wd[0].timing == EffectTiming.OnEnterFieldAnyone

    def test_when_digivolving_deletes_lv4_or_lower(self, debug_runner):
        """Process should delete 1 opponent Digimon with level 4 or lower."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [BT24_076])
        opp_lv4 = runner.place_on_field(2, [GREYMON])
        game = runner.game

        card = perm.top_card
        effects = card.effect_list(None)
        wd = [e for e in effects if getattr(e, 'is_when_digivolving', False)][0]

        opp_field_before = len(game.player2.battle_area)

        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        assert len(game.player2.battle_area) < opp_field_before, (
            "Should have deleted opponent's Lv.4 Digimon")

    def test_when_digivolving_is_not_optional(self, debug_runner):
        """Delete effect should be mandatory (not optional)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [BT24_076])
        card = perm.top_card
        effects = card.effect_list(None)
        wd = [e for e in effects if getattr(e, 'is_when_digivolving', False)][0]
        assert not getattr(wd, 'is_optional', False), (
            "Delete effect should be mandatory (not optional)")


# ── Clause 5: Inherited [On Deletion] ─────────────────────────────────


@pytest.mark.behavioral
class TestBT24076InheritedOnDeletion:
    """C5: Inherited [On Deletion] play lv4- Dark Dragon/Evil Dragon from trash free."""

    def test_inherited_effect_exists(self, debug_runner):
        """Should have an OnDestroyedAnyone inherited effect."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [BT24_076])
        card = perm.top_card
        effects = card.effect_list(None)
        inherited = [
            e for e in effects
            if getattr(e, 'is_inherited_effect', False)
            and getattr(e, 'is_on_deletion', False)
        ]
        assert len(inherited) >= 1, (
            "Should have inherited [On Deletion] effect")
        assert inherited[0].timing == EffectTiming.OnDestroyedAnyone

    def test_inherited_is_optional(self, debug_runner):
        """Inherited On Deletion is optional ('You may')."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [BT24_076])
        card = perm.top_card
        effects = card.effect_list(None)
        inherited = [
            e for e in effects
            if getattr(e, 'is_inherited_effect', False)
            and getattr(e, 'is_on_deletion', False)
        ][0]
        assert getattr(inherited, 'is_optional', False), (
            "Inherited On Deletion should be optional")

    def test_inherited_plays_dark_dragon_from_trash(self, debug_runner):
        """Should be able to play a lv4 or lower [Dark Dragon] from trash."""
        runner = debug_runner(initial_memory=10)
        # Place BT24-076 under another Digimon (as inherited)
        perm = runner.place_on_field(1, [BT24_076, METALGREYMON])
        # Put a Dark Dragon lv4 in P1's trash
        dark_dragon = runner.inject_card(1, GROWLMON, "trash")
        game = runner.game

        # Get the inherited effect from BT24-076 (under card)
        source_card = perm.card_sources[0]  # BT24-076
        effects = source_card.effect_list(None)
        inherited = [
            e for e in effects
            if getattr(e, 'is_inherited_effect', False)
            and getattr(e, 'is_on_deletion', False)
        ][0]

        field_before = len(game.player1.battle_area)

        inherited.on_process_callback({
            'player': game.player1,
            'game': game,
        })

        # Selection phase should offer the Dark Dragon from trash
        from engine_py_legacy.engine.data.enums import GamePhase
        assert game.current_phase == GamePhase.SelectTarget, (
            "Should be in selection phase for trash play")
        legal = runner.action_mask()
        # Find the trash selection action (SEL_TRASH_START = 130)
        trash_actions = [a for a in legal if a >= 130 and a < 180]
        assert len(trash_actions) >= 1, (
            "Should have at least 1 valid trash card to select")
        runner.execute(trash_actions[0])
        runner.auto_resolve()

        # The Dark Dragon should have been played from trash to field
        assert dark_dragon not in game.player1.trash_cards, (
            "Dark Dragon should no longer be in trash")
        assert len(game.player1.battle_area) > field_before, (
            "Battle area should have gained a permanent")

    def test_inherited_plays_evil_dragon_from_trash(self, debug_runner):
        """Should be able to play a lv4 or lower [Evil Dragon] from trash."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [BT24_076, METALGREYMON])
        evil_dragon = runner.inject_card(1, DEVIDRAMON, "trash")
        game = runner.game

        source_card = perm.card_sources[0]
        effects = source_card.effect_list(None)
        inherited = [
            e for e in effects
            if getattr(e, 'is_inherited_effect', False)
            and getattr(e, 'is_on_deletion', False)
        ][0]

        field_before = len(game.player1.battle_area)

        inherited.on_process_callback({
            'player': game.player1,
            'game': game,
        })

        # Selection phase should offer the Evil Dragon from trash
        legal = runner.action_mask()
        trash_actions = [a for a in legal if a >= 130 and a < 180]
        assert len(trash_actions) >= 1, (
            "Should have at least 1 valid trash card to select")
        runner.execute(trash_actions[0])
        runner.auto_resolve()

        assert evil_dragon not in game.player1.trash_cards, (
            "Evil Dragon should no longer be in trash")
        assert len(game.player1.battle_area) > field_before, (
            "Battle area should have gained a permanent")

    def test_inherited_rejects_non_dark_evil_dragon(self, debug_runner):
        """Should NOT play a Digimon that lacks [Dark Dragon] or [Evil Dragon] trait."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [BT24_076, METALGREYMON])
        # Put a non-Dark/Evil Dragon lv3 in trash
        non_dragon = runner.inject_card(1, BT1_010, "trash")
        game = runner.game

        source_card = perm.card_sources[0]
        effects = source_card.effect_list(None)
        inherited = [
            e for e in effects
            if getattr(e, 'is_inherited_effect', False)
            and getattr(e, 'is_on_deletion', False)
        ][0]

        field_before = len(game.player1.battle_area)

        inherited.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        runner.auto_resolve()

        # Non-dragon should stay in trash
        assert non_dragon in game.player1.trash_cards, (
            "Non-Dark/Evil Dragon should remain in trash")
        assert len(game.player1.battle_area) == field_before, (
            "Field should not gain a permanent")

    def test_inherited_rejects_lv5_dark_dragon(self, debug_runner):
        """Should NOT play a lv5 or higher Dark Dragon (only lv4 or lower)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [BT24_076, METALGREYMON])
        # BT4-058 is Lv.5 Dark Dragon
        lv5_dragon = runner.inject_card(1, "BT4-058", "trash")
        game = runner.game

        source_card = perm.card_sources[0]
        effects = source_card.effect_list(None)
        inherited = [
            e for e in effects
            if getattr(e, 'is_inherited_effect', False)
            and getattr(e, 'is_on_deletion', False)
        ][0]

        field_before = len(game.player1.battle_area)

        inherited.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        runner.auto_resolve()

        # Lv.5 Dark Dragon should stay in trash
        assert lv5_dragon in game.player1.trash_cards, (
            "Lv.5 Dark Dragon should remain in trash (only lv4 or lower)")
        assert len(game.player1.battle_area) == field_before, (
            "Field should not gain a permanent")
