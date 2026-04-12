"""Behavioral tests for BT15-072 Vilemon (Lv.4 Purple, Cost 4, Evil trait).

Card text:
    <Blocker>
    [All Turns] When one of your [Apocalymon] or Digimon with the [Dark Masters]
    trait would leave the battle area other than by one of your effects, by
    deleting this Digimon, prevent 1 of those Digimon from leaving.

Key behaviors:
    - Blocker keyword
    - WhenPermanentWouldBeDeleted interrupter that protects Apocalymon/Dark Masters allies
    - Cost: delete Vilemon itself
    - Condition: only by opponent's effects (not own)
    - Once per turn (OPT)
    - Optional
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT15072Vilemon:
    """Tests for BT15-072 Vilemon."""

    # ---- Effect 0: Blocker ----

    def test_has_blocker(self, debug_runner):
        """Vilemon should have the Blocker keyword."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-072"])
        assert perm.has_keyword("_is_blocker"), "Vilemon must have Blocker"

    # ---- Effect 1: Protect Apocalymon/Dark Masters from opponent deletion ----

    def test_protect_dark_masters_from_opponent_deletion(self, debug_runner):
        """When opponent deletes a Dark Masters Digimon, Vilemon can sacrifice
        itself to prevent the deletion."""
        runner = debug_runner(initial_memory=5)

        # Place Vilemon and a Dark Masters Digimon (MetalSeadramon) on P1's field
        vilemon_perm = runner.place_on_field(1, ["BT15-072"])
        dm_perm = runner.place_on_field(1, ["BT15-031"])  # MetalSeadramon, Dark Masters trait

        # Confirm both are on the field
        assert len(runner.game.player1.battle_area) == 2

        # Simulate opponent deleting the Dark Masters Digimon
        p1 = runner.game.player1
        was_deleted = p1.delete_permanent(dm_perm, is_opponent_effect=True)

        # Resolve any selection phases (e.g. the optional interrupter)
        runner.auto_resolve()

        # If the effect triggered, Vilemon should be deleted (in trash)
        # and the Dark Masters Digimon should still be on the field
        vilemon_in_trash = any(
            c.c_entity_base and c.c_entity_base.card_id == "BT15-072"
            for c in p1.trash_cards
        )
        dm_on_field = dm_perm in p1.battle_area

        # The effect is optional, so it either:
        # - Activated: Vilemon in trash, DM on field, was_deleted=False
        # - Declined: Vilemon on field, DM in trash, was_deleted=True
        # Auto-resolve picks first legal action which should accept the optional effect
        if vilemon_in_trash:
            assert dm_on_field, (
                "If Vilemon was sacrificed, the Dark Masters Digimon should remain on field"
            )
            assert not was_deleted, (
                "delete_permanent should return False when deletion was prevented"
            )
        else:
            # Effect might not have triggered (engine gap) - this test checks it CAN trigger
            pytest.fail(
                "Vilemon's protection effect did not trigger when opponent deleted a "
                "Dark Masters Digimon. The WhenPermanentWouldBeDeleted interrupter "
                "must fire and allow Vilemon to sacrifice itself."
            )

    def test_protect_apocalymon_from_opponent_deletion(self, debug_runner):
        """Vilemon should also protect [Apocalymon] by name, not just Dark Masters trait."""
        runner = debug_runner(initial_memory=5)

        vilemon_perm = runner.place_on_field(1, ["BT15-072"])
        apocalymon_perm = runner.place_on_field(1, ["BT15-102"])  # Apocalymon

        p1 = runner.game.player1
        assert len(p1.battle_area) == 2

        was_deleted = p1.delete_permanent(apocalymon_perm, is_opponent_effect=True)
        runner.auto_resolve()

        vilemon_in_trash = any(
            c.c_entity_base and c.c_entity_base.card_id == "BT15-072"
            for c in p1.trash_cards
        )
        apocalymon_on_field = apocalymon_perm in p1.battle_area

        if vilemon_in_trash:
            assert apocalymon_on_field, (
                "Apocalymon should remain on field after Vilemon sacrificed itself"
            )
        else:
            pytest.fail(
                "Vilemon did not protect Apocalymon from opponent deletion"
            )

    def test_does_not_protect_non_dark_masters_non_apocalymon(self, debug_runner):
        """Vilemon should NOT protect Digimon that are neither Apocalymon nor Dark Masters."""
        runner = debug_runner(initial_memory=5)

        vilemon_perm = runner.place_on_field(1, ["BT15-072"])
        # Place a random Digimon with no Dark Masters trait and not named Apocalymon
        other_perm = runner.place_on_field(1, ["ST1-07"])  # Greymon

        p1 = runner.game.player1
        was_deleted = p1.delete_permanent(other_perm, is_opponent_effect=True)
        runner.auto_resolve()

        # Vilemon should NOT have sacrificed itself
        vilemon_on_field = vilemon_perm in p1.battle_area
        assert vilemon_on_field, (
            "Vilemon should NOT sacrifice itself for a non-Apocalymon/non-Dark-Masters Digimon"
        )
        assert was_deleted, (
            "The non-qualifying Digimon should have been deleted normally"
        )

    def test_does_not_protect_from_own_effects(self, debug_runner):
        """Vilemon should NOT trigger when YOUR OWN effect deletes the ally."""
        runner = debug_runner(initial_memory=5)

        vilemon_perm = runner.place_on_field(1, ["BT15-072"])
        dm_perm = runner.place_on_field(1, ["BT15-031"])  # MetalSeadramon

        p1 = runner.game.player1

        # Delete by own effect (is_opponent_effect=False)
        was_deleted = p1.delete_permanent(dm_perm, is_opponent_effect=False)
        runner.auto_resolve()

        # Vilemon should NOT have triggered
        vilemon_on_field = vilemon_perm in p1.battle_area
        assert vilemon_on_field, (
            "Vilemon must NOT sacrifice itself when the ally is deleted by own effect"
        )
        assert was_deleted, (
            "Dark Masters Digimon should be deleted when removed by own effects"
        )

    def test_vilemon_is_deleted_as_cost(self, debug_runner):
        """When Vilemon protects, it should be deleted (sent to trash) as the cost."""
        runner = debug_runner(initial_memory=5)

        vilemon_perm = runner.place_on_field(1, ["BT15-072"])
        dm_perm = runner.place_on_field(1, ["BT15-031"])

        p1 = runner.game.player1
        field_count_before = len(p1.battle_area)

        p1.delete_permanent(dm_perm, is_opponent_effect=True)
        runner.auto_resolve()

        # Vilemon should be in trash
        vilemon_ids_in_trash = [
            c.c_entity_base.card_id for c in p1.trash_cards
            if c.c_entity_base and c.c_entity_base.card_id == "BT15-072"
        ]
        assert len(vilemon_ids_in_trash) == 1, (
            "Vilemon should be in trash after being deleted as cost"
        )

        # DM should still be on field, so only 1 permanent remains (DM itself)
        assert dm_perm in p1.battle_area, (
            "The protected Dark Masters Digimon should remain on field"
        )

    def test_effect_is_optional(self, debug_runner):
        """The protection effect should be optional (canNoSelect=true in C#)."""
        runner = debug_runner(initial_memory=5)

        vilemon_perm = runner.place_on_field(1, ["BT15-072"])
        dm_perm = runner.place_on_field(1, ["BT15-031"])

        # Check the effect metadata
        effects = vilemon_perm.top_card.effect_list(None)
        protect_effects = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ]
        assert len(protect_effects) == 1, (
            "Should have exactly 1 WhenPermanentWouldBeDeleted effect"
        )
        assert protect_effects[0].is_optional, (
            "The protection effect must be optional"
        )

    def test_effect_is_once_per_turn(self, debug_runner):
        """The protection effect should be once per turn (max_count=1 in C#)."""
        runner = debug_runner(initial_memory=5)

        vilemon_perm = runner.place_on_field(1, ["BT15-072"])

        effects = vilemon_perm.top_card.effect_list(None)
        protect_effects = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ]
        assert len(protect_effects) == 1
        assert protect_effects[0].max_count_per_turn == 1, (
            "Protection effect should be once per turn"
        )

    def test_does_not_protect_itself(self, debug_runner):
        """Vilemon protects OTHER Digimon, not itself. Even if Vilemon somehow had
        Dark Masters trait, it shouldn't protect itself (C# checks permanent != card.PermanentOfThisCard())."""
        runner = debug_runner(initial_memory=5)

        vilemon_perm = runner.place_on_field(1, ["BT15-072"])

        p1 = runner.game.player1
        # Try to delete Vilemon by opponent effect
        was_deleted = p1.delete_permanent(vilemon_perm, is_opponent_effect=True)
        runner.auto_resolve()

        # Vilemon should be deleted (it can't protect itself)
        assert was_deleted, (
            "Vilemon cannot protect itself, so it should be deleted"
        )

    def test_has_process_callback(self, debug_runner):
        """The WhenPermanentWouldBeDeleted effect must have a process callback.
        Without it, activating the effect does nothing."""
        runner = debug_runner(initial_memory=5)

        vilemon_perm = runner.place_on_field(1, ["BT15-072"])

        effects = vilemon_perm.top_card.effect_list(None)
        protect_effects = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ]
        assert len(protect_effects) == 1
        assert protect_effects[0].on_process_callback is not None, (
            "The protection effect MUST have a process callback. "
            "Without it, the effect activates but does nothing."
        )
