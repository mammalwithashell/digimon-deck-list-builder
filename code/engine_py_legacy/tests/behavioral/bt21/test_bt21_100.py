"""Behavioral tests for BT21-100 The Digimon I Designed (Option, Purple, Cost 2).

Card text:
While you have [Takato Matsuki], you can ignore this card's color requirements.
[Main] <Draw 1> (Draw 1 card from your deck.) and trash 1 card in your hand.
    Then, place this card in the battle area.
[Your Turn] When effects delete Digimon, <Delay> (By trashing this card after
    the placing turn, activate the effect below.)
    - 1 of your Digimon with [Guilmon] or [Growlmon] in its name may digivolve
      into a Digimon card with [Growlmon], [Gallantmon] or [Megidramon] in its
      name in the trash without paying the cost.
[Security] Gain 1 memory. Then, place this card in the battle area.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


CARD_ID = "BT21-100"

# Purple Digimon for color requirement
PURPLE_LV3 = "BT2-067"  # DemiDevimon (Purple Lv3)

# Takato Matsuki tamer
TAKATO = "BT12-089"

# Guilmon/Growlmon line for digivolve targets
GUILMON_LV3 = "BT2-009"      # Guilmon (Red, Lv3)
GROWLMON_LV4 = "BT2-013"     # Growlmon (Red, Lv4)
GALLANTMON_LV6 = "BT2-020"   # Gallantmon (Red, Lv6)
MEGIDRAMON_LV6 = "BT5-083"   # Megidramon (Red, Lv6)

# Non-matching Digimon
GENERIC_LV4 = "ST1-07"       # Greymon (Red, Lv4 - no Guilmon/Growlmon in name)


def _card_ids_in_zone(cards):
    """Extract card_ids from a list of CardSource objects."""
    return [c.c_entity_base.card_id for c in cards if c.c_entity_base]


def _field_card_ids(player):
    """Extract top card_ids from battle_area permanents."""
    return [
        p.top_card.c_entity_base.card_id
        for p in player.battle_area
        if p.top_card and p.top_card.c_entity_base
    ]


@pytest.mark.behavioral
class TestBT21100IgnoreColorRequirement:
    """Tests for the ignore-color-requirement clause."""

    def test_can_play_with_takato_and_no_purple(self, debug_runner):
        """With Takato on field and no purple Digimon, should still be playable."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        # Place Takato (a tamer) on field, but NO purple Digimon/Tamer
        runner.place_on_field(1, [TAKATO])

        # Place a red Digimon (not purple)
        runner.place_on_field(1, [GUILMON_LV3])

        runner.inject_card(1, CARD_ID, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Digimon I Designed")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None, (
            f"Should be able to play BT21-100 with Takato on field ignoring color. "
            f"Actions: {runner.actions()}"
        )

    def test_cannot_play_without_takato_or_purple(self, debug_runner):
        """Without Takato and without purple on field, should NOT be playable."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place only a red Digimon (no Takato, no purple)
        runner.place_on_field(1, [GUILMON_LV3])

        runner.inject_card(1, CARD_ID, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Digimon I Designed")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is None, (
            f"Should NOT be able to play BT21-100 without Takato or purple on field. "
            f"Actions: {runner.actions()}"
        )


@pytest.mark.behavioral
class TestBT21100MainEffect:
    """Tests for the [Main] effect: Draw 1, trash 1, place in battle area."""

    def test_main_draws_one_card(self, debug_runner):
        """[Main] should draw 1 card."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        # Purple Digimon for color requirement
        runner.place_on_field(1, [PURPLE_LV3])

        runner.inject_card(1, CARD_ID, "hand")
        runner.set_phase("Main")

        hand_before = len(player.hand_cards)

        action = runner.find_action("Digimon I Designed")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None, f"Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        # After: drew 1, trashed 1 (from hand), and BT21-100 left hand -> battle area
        # Net hand change: -1 (played option) + 1 (draw) - 1 (trash) = -1
        # But BT21-100 is no longer in hand (it's on field), so net is hand_before - 1
        # The draw happens first, then trash, so we should have drawn
        # Verify by checking deck size decreased
        assert len(player.library_cards) < 50, "Should have drawn from deck"

    def test_main_trashes_one_from_hand(self, debug_runner):
        """[Main] should trash 1 card from hand (mandatory)."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [PURPLE_LV3])
        runner.inject_card(1, CARD_ID, "hand")
        runner.set_phase("Main")

        trash_before = len(player.trash_cards)

        action = runner.find_action("Digimon I Designed")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        # At least 1 card should be in trash (from hand trash)
        assert len(player.trash_cards) > trash_before, (
            f"Should have trashed at least 1 card. Trash before: {trash_before}, "
            f"after: {len(player.trash_cards)}"
        )

    def test_main_places_in_battle_area(self, debug_runner):
        """[Main] places this card in the battle area (Delay placement)."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [PURPLE_LV3])
        runner.inject_card(1, CARD_ID, "hand")
        runner.set_phase("Main")

        action = runner.find_action("Digimon I Designed")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        ba_ids = _field_card_ids(player)
        assert CARD_ID in ba_ids, (
            f"BT21-100 should be placed in battle area. BA: {ba_ids}"
        )


@pytest.mark.behavioral
class TestBT21100DelayTrigger:
    """Tests for the Delay trigger: [Your Turn] When effects delete Digimon."""

    def test_delay_triggers_on_effect_deletion(self, debug_runner):
        """Delay triggers when a Digimon is deleted by effect, trashes the option."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        # Place BT21-100 in battle area (played last turn)
        runner.place_on_field(1, [CARD_ID], turn_played=0)
        game.turn_count = 2

        # Guilmon on field as digivolve source
        runner.place_on_field(1, [GUILMON_LV3])

        # Growlmon in trash as digivolve target
        runner.inject_card(1, GROWLMON_LV4, "trash")

        # Some Digimon to be deleted by effect
        runner.place_on_field(2, [GENERIC_LV4])

        # Delete opponent's Digimon by effect
        opp = game.player2
        target = [p for p in opp.battle_area if p.is_digimon][0]
        opp.delete_permanent(target)

        # After deletion observer fires, the delay should trigger
        # BT21-100 should be trashed (delay cost)
        ba_ids = _field_card_ids(player)
        assert CARD_ID not in ba_ids, (
            f"BT21-100 should be trashed after delay activation. BA: {ba_ids}"
        )

        trash_ids = _card_ids_in_zone(player.trash_cards)
        assert CARD_ID in trash_ids, (
            f"BT21-100 should be in trash after delay. Trash: {trash_ids}"
        )

    def test_delay_does_not_trigger_on_battle_deletion(self, debug_runner):
        """Delay should NOT trigger when a Digimon is deleted by battle (not effect)."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        # Place BT21-100 in battle area (played last turn)
        runner.place_on_field(1, [CARD_ID], turn_played=0)
        game.turn_count = 2

        # Guilmon on field as digivolve source
        runner.place_on_field(1, [GUILMON_LV3])

        # Growlmon in trash as digivolve target
        runner.inject_card(1, GROWLMON_LV4, "trash")

        # Simulate a Digimon being deleted by battle (not effect)
        runner.place_on_field(2, [GENERIC_LV4])
        opp = game.player2
        target = [p for p in opp.battle_area if p.is_digimon][0]
        # removal_cause='battle' should NOT trigger the delay
        game.execute_deletion_effects(target, opp, removal_cause='battle')
        if target in opp.battle_area:
            opp.battle_area.remove(target)
            for cs in target.card_sources:
                opp.trash_cards.append(cs)

        # BT21-100 should still be on field
        ba_ids = _field_card_ids(player)
        assert CARD_ID in ba_ids, (
            f"BT21-100 should still be on field (battle deletion, not effect). BA: {ba_ids}"
        )

    def test_delay_not_available_on_placing_turn(self, debug_runner):
        """Delay cannot be activated on the same turn the card was placed."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        # Place BT21-100 this turn
        game.turn_count = 1
        runner.place_on_field(1, [CARD_ID], turn_played=1)

        # Guilmon on field + Growlmon in trash
        runner.place_on_field(1, [GUILMON_LV3])
        runner.inject_card(1, GROWLMON_LV4, "trash")

        # Delete a Digimon by effect on the same turn
        runner.place_on_field(2, [GENERIC_LV4])
        opp = game.player2
        target = [p for p in opp.battle_area if p.is_digimon][0]
        opp.delete_permanent(target)

        # BT21-100 should still be on field (placed this turn)
        ba_ids = _field_card_ids(player)
        assert CARD_ID in ba_ids, (
            f"BT21-100 should still be on field (placed this turn). BA: {ba_ids}"
        )

    def test_delay_only_triggers_on_your_turn(self, debug_runner):
        """Delay triggers only on [Your Turn], not opponent's turn."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        # Place BT21-100 in battle area (played last turn)
        runner.place_on_field(1, [CARD_ID], turn_played=0)
        game.turn_count = 2

        # Switch to opponent's turn
        player.is_my_turn = False
        game.player2.is_my_turn = True

        # Guilmon on field + Growlmon in trash
        runner.place_on_field(1, [GUILMON_LV3])
        runner.inject_card(1, GROWLMON_LV4, "trash")

        # Delete a Digimon by effect on opponent's turn
        runner.place_on_field(2, [GENERIC_LV4])
        opp = game.player2
        target = [p for p in opp.battle_area if p.is_digimon][0]
        opp.delete_permanent(target)

        # BT21-100 should still be on field (not your turn)
        ba_ids = _field_card_ids(player)
        assert CARD_ID in ba_ids, (
            f"BT21-100 should still be on field (opponent's turn). BA: {ba_ids}"
        )

    def test_delay_only_triggers_on_digimon_deletion(self, debug_runner):
        """Delay triggers when a DIGIMON is deleted, not Tamer/Option."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        # Place BT21-100 in battle area (played last turn)
        runner.place_on_field(1, [CARD_ID], turn_played=0)
        game.turn_count = 2

        # Guilmon on field + Growlmon in trash
        runner.place_on_field(1, [GUILMON_LV3])
        runner.inject_card(1, GROWLMON_LV4, "trash")

        # Place and delete an opponent's Tamer (not Digimon)
        runner.place_on_field(2, [TAKATO])
        opp = game.player2
        tamer = [p for p in opp.battle_area if p.is_tamer][0]
        opp.delete_permanent(tamer)

        # BT21-100 should still be on field (Tamer deleted, not Digimon)
        ba_ids = _field_card_ids(player)
        assert CARD_ID in ba_ids, (
            f"BT21-100 should still be on field (Tamer deleted, not Digimon). BA: {ba_ids}"
        )


@pytest.mark.behavioral
class TestBT21100DelayDigivolve:
    """Tests for the Delay digivolve effect."""

    def test_digivolve_guilmon_into_growlmon_from_trash(self, debug_runner):
        """Guilmon can digivolve into Growlmon from trash without paying cost."""
        from engine_py_legacy.engine.game.constants import SEL_TRASH_START
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        # Place BT21-100 in battle area (played last turn)
        runner.place_on_field(1, [CARD_ID], turn_played=0)
        game.turn_count = 2

        # Guilmon on field
        guilmon_perm = runner.place_on_field(1, [GUILMON_LV3])

        # Growlmon in trash
        runner.inject_card(1, GROWLMON_LV4, "trash")

        # Delete an opponent Digimon by effect to trigger the delay
        runner.place_on_field(2, [GENERIC_LV4])
        opp = game.player2
        target = [p for p in opp.battle_area if p.is_digimon][0]
        opp.delete_permanent(target)

        # The delay fires, entering SelectTarget for the own permanent.
        # Select Guilmon (non-decline action in the field slot range)
        legal = runner.action_mask()
        field_action = [a for a in legal if a != 62]  # 62 = pass/decline
        assert field_action, f"Should have a field selection action. Legal: {legal}"
        runner.execute(field_action[0])

        # Now select Growlmon from trash
        legal = runner.action_mask()
        trash_action = [a for a in legal if a >= SEL_TRASH_START]
        assert trash_action, f"Should have a trash selection action. Legal: {legal}"
        runner.execute(trash_action[0])
        runner.auto_resolve()

        # Check that Growlmon is now on top of the stack
        found_growlmon = False
        for perm in player.battle_area:
            if perm.top_card and perm.top_card.c_entity_base:
                if perm.top_card.c_entity_base.card_id == GROWLMON_LV4:
                    found_growlmon = True
                    # Should have Guilmon underneath
                    stack_ids = [cs.c_entity_base.card_id for cs in perm.card_sources if cs.c_entity_base]
                    assert GUILMON_LV3 in stack_ids, (
                        f"Guilmon should be under Growlmon. Stack: {stack_ids}"
                    )
                    break

        assert found_growlmon, (
            f"Growlmon should be on field after digivolve from trash. "
            f"Field: {_field_card_ids(player)}"
        )

    def test_digivolve_into_gallantmon_from_trash(self, debug_runner):
        """Growlmon can digivolve into Gallantmon from trash."""
        from engine_py_legacy.engine.game.constants import SEL_TRASH_START
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        # Place BT21-100 in battle area (played last turn)
        runner.place_on_field(1, [CARD_ID], turn_played=0)
        game.turn_count = 2

        # Growlmon on field (has "Growlmon" in name)
        runner.place_on_field(1, [GROWLMON_LV4])

        # Gallantmon in trash
        runner.inject_card(1, GALLANTMON_LV6, "trash")

        # Delete opponent Digimon to trigger delay
        runner.place_on_field(2, [GENERIC_LV4])
        opp = game.player2
        target = [p for p in opp.battle_area if p.is_digimon][0]
        opp.delete_permanent(target)

        # Select Growlmon from field
        legal = runner.action_mask()
        field_action = [a for a in legal if a != 62]
        assert field_action, f"Should have a field selection action. Legal: {legal}"
        runner.execute(field_action[0])

        # Select Gallantmon from trash
        legal = runner.action_mask()
        trash_action = [a for a in legal if a >= SEL_TRASH_START]
        assert trash_action, f"Should have a trash selection action. Legal: {legal}"
        runner.execute(trash_action[0])
        runner.auto_resolve()

        # Check Gallantmon is on field
        found_gallantmon = any(
            p.top_card.c_entity_base.card_id == GALLANTMON_LV6
            for p in player.battle_area
            if p.top_card and p.top_card.c_entity_base
        )
        assert found_gallantmon, (
            f"Gallantmon should be on field after digivolve from trash. "
            f"Field: {_field_card_ids(player)}"
        )

    def test_no_digivolve_without_guilmon_growlmon_on_field(self, debug_runner):
        """Without Guilmon or Growlmon on field, no digivolve happens."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        # Place BT21-100 in battle area (played last turn)
        runner.place_on_field(1, [CARD_ID], turn_played=0)
        game.turn_count = 2

        # Generic Digimon on field (not Guilmon/Growlmon)
        runner.place_on_field(1, [GENERIC_LV4])

        # Growlmon in trash
        runner.inject_card(1, GROWLMON_LV4, "trash")

        # Delete opponent Digimon to trigger delay
        runner.place_on_field(2, [GENERIC_LV4])
        opp = game.player2
        target = [p for p in opp.battle_area if p.is_digimon][0]
        opp.delete_permanent(target)
        runner.auto_resolve()

        # The generic Digimon should NOT have been digivolved
        for perm in player.battle_area:
            if perm.top_card and perm.top_card.c_entity_base:
                assert perm.top_card.c_entity_base.card_id != GROWLMON_LV4, (
                    "Should not digivolve non-Guilmon/Growlmon Digimon"
                )

    def test_digivolve_is_optional(self, debug_runner):
        """The digivolve effect is optional (may digivolve)."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        player = game.player1

        # Place BT21-100 in battle area (played last turn)
        runner.place_on_field(1, [CARD_ID], turn_played=0)
        game.turn_count = 2

        # Guilmon on field
        runner.place_on_field(1, [GUILMON_LV3])

        # Growlmon in trash
        runner.inject_card(1, GROWLMON_LV4, "trash")

        # Delete opponent Digimon to trigger delay
        runner.place_on_field(2, [GENERIC_LV4])
        opp = game.player2
        target = [p for p in opp.battle_area if p.is_digimon][0]
        opp.delete_permanent(target)

        # The delay triggers and trashes the option; digivolve selection should
        # be optional. Check that the effect has is_optional flag.
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(CARD_ID, player)
        all_effects = cs.effect_list(EffectTiming.NoTiming)
        deletion_observers = [e for e in all_effects if getattr(e, '_is_deletion_observer', False)]
        assert len(deletion_observers) >= 1, "Should have a deletion observer effect"
        assert deletion_observers[0].is_optional, "Delay digivolve effect should be optional"


@pytest.mark.behavioral
class TestBT21100SecurityEffect:
    """Tests for [Security] Gain 1 memory. Then, place this card in the battle area."""

    def test_security_gains_memory(self, debug_runner):
        """[Security] should gain 1 memory."""
        runner = debug_runner(initial_memory=0)
        game = runner.game
        player = game.player1

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(CARD_ID, player)

        effects = cs.effect_list(EffectTiming.SecuritySkill)
        security_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(security_effects) >= 1, "Should have a security effect"

        # Fire the security effect
        ctx = {
            'game': game,
            'player': player,
            'permanent': None,
            'card': cs,
        }
        mem_before = game.memory
        security_effects[0].on_process_callback(ctx)

        assert game.memory == mem_before + 1, (
            f"Should gain 1 memory. Before: {mem_before}, after: {game.memory}"
        )

    def test_security_places_in_battle_area(self, debug_runner):
        """[Security] should place this card in the battle area."""
        runner = debug_runner(initial_memory=0)
        game = runner.game
        player = game.player1

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(CARD_ID, player)

        effects = cs.effect_list(EffectTiming.SecuritySkill)
        security_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(security_effects) >= 1

        ctx = {
            'game': game,
            'player': player,
            'permanent': None,
            'card': cs,
        }
        security_effects[0].on_process_callback(ctx)

        # Card should be placed in battle area
        ba_ids = _field_card_ids(player)
        assert CARD_ID in ba_ids, (
            f"BT21-100 should be placed in battle area from security. BA: {ba_ids}"
        )

        # Card should be marked as security_played to avoid being trashed
        assert getattr(cs, '_security_played', False), (
            "Card should be marked _security_played so it isn't trashed by security_attack"
        )
