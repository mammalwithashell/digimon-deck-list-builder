"""Behavioral tests for EX7-049 Metallicdramon (Lv.6 Black Sky Dragon).

Card text:
  [On Play] [When Attacking] <De-Digivolve 4> 1 of your opponent's Digimon.
  [When Digivolving] None of your opponent's level 4 or lower Digimon can
      digivolve until the end of their turn.
  [All Turns] [Once Per Turn] When this Digimon would leave battle area other
      than by one of your effects, you may play 1 Digimon card with the
      [Rock Dragon] or [Earth Dragon] trait from your trash without paying
      the cost.
"""

import pytest

from digimon_gym.engine.data.enums import GamePhase


def _select_non_decline(runner):
    """After a WhenRemoveField trigger, pick the non-decline action if
    the engine is in a SelectTarget phase with an optional selection.
    Returns True if a card was selected, False otherwise."""
    if runner.game.current_phase != GamePhase.SelectTarget:
        return False
    legal = runner.action_mask()
    # 62 is typically the Decline/Pass action
    play_actions = [a for a in legal if a != 62]
    if not play_actions:
        return False
    runner.execute(play_actions[0])
    runner.auto_resolve()
    return True


@pytest.mark.behavioral
class TestEX7049DeDigivolve:
    """Tests for [On Play] / [When Attacking] <De-Digivolve 4> effect."""

    def test_on_play_de_digivolve_4(self, debug_runner):
        """On Play should De-Digivolve 4 an opponent Digimon."""
        runner = debug_runner(initial_memory=20)

        # Opponent: stacked Digimon with 4 evo cards underneath
        # Stack (bottom-to-top): BT1-003, BT1-009, BT1-015, BT1-020, BT1-020
        # That's 4 evo cards under top card BT1-020
        opp_perm = runner.place_on_field(2, [
            "BT1-003", "BT1-009", "BT1-015", "BT1-020", "BT1-020"
        ])
        before_stack = len(opp_perm.card_sources)
        assert before_stack == 5, f"Opponent should have 5 cards in stack, got {before_stack}"

        # Play Metallicdramon
        runner.inject_card(1, "EX7-049", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Metallicdramon")
        assert play is not None, f"Should be able to play Metallicdramon. Actions: {runner.actions()}"
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Opponent Digimon should have lost up to 4 cards from top
        opp_field = [s for s in snap.p2_field if s.is_digimon]
        if opp_field:
            # Remaining stack should be smaller
            assert len(opp_field[0].stack_ids) < before_stack, (
                "Opponent Digimon should have fewer cards after De-Digivolve 4"
            )
        # Trashed cards should be in opponent's trash
        assert len(snap.p2_trash) > 0, (
            "De-Digivolved cards should be in opponent's trash"
        )


@pytest.mark.behavioral
class TestEX7049WhenDigivolving:
    """Tests for [When Digivolving] opponent Lv.4 or lower can't digivolve."""

    def test_when_digivolving_blocks_opponent_lv4_digivolve(self, debug_runner):
        """When Metallicdramon digivolves, opponent's Lv.4 or lower Digimon
        should have CANNOT_DIGIVOLVE modifier."""
        runner = debug_runner(initial_memory=10)

        # Opponent has a Lv.4 Digimon on field
        opp_perm = runner.place_on_field(2, ["BT1-015"])  # Lv.4
        assert opp_perm.level == 4

        # Place Black Lv.5 on our field, then digivolve into Metallicdramon
        runner.place_on_field(1, ["BT2-060"])  # Megadramon, Black Lv.5
        runner.inject_card(1, "EX7-049", "hand")
        runner.set_phase("Main")

        digivolve = runner.find_action("Digivolve")
        assert digivolve is not None, (
            f"Should be able to digivolve into Metallicdramon. Actions: {runner.actions()}"
        )
        runner.execute(digivolve)
        runner.auto_resolve()

        # Check that the opponent Lv.4 has CANNOT_DIGIVOLVE modifier
        from digimon_gym.engine.interfaces.modifiers import ModifierType
        has_block = runner.game.modifiers.has_modifier(
            opp_perm, ModifierType.CANNOT_DIGIVOLVE, {"permanent": opp_perm}
        )
        assert has_block, (
            "Opponent's Lv.4 Digimon should have CANNOT_DIGIVOLVE after "
            "Metallicdramon's When Digivolving effect"
        )


@pytest.mark.behavioral
class TestEX7049WhenLeaveField:
    """Tests for [All Turns] [Once Per Turn] When this Digimon would leave
    battle area other than by one of your effects, play Rock/Earth Dragon
    from trash."""

    def test_triggers_on_opponent_effect_removal(self, debug_runner):
        """Effect SHOULD trigger when Metallicdramon is removed by opponent's effect.
        This is the primary case: 'other than by one of your effects' means
        opponent effects are allowed."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX7-049"])

        # Put a Rock Dragon Digimon in our trash (BT2-011 Vorvomon, Lv.3 Rock Dragon)
        runner.inject_card(1, "BT2-011", "trash")

        p1 = runner.game.player1
        before_trash = [c.c_entity_base.card_id for c in p1.trash_cards]
        assert "BT2-011" in before_trash, "Vorvomon should be in trash"

        # Delete Metallicdramon by opponent's effect
        p1.delete_permanent(perm, is_opponent_effect=True)

        # The effect creates an optional selection — explicitly select the play action
        selected = _select_non_decline(runner)
        assert selected, "WhenRemoveField should create a selection to play from trash"

        snap = runner.snapshot()
        # Vorvomon should be played from trash to field
        field_ids = [s.card_id for s in snap.p1_field]
        assert "BT2-011" in field_ids, (
            f"Vorvomon (Rock Dragon) should be played from trash when Metallicdramon "
            f"is removed by opponent effect. Field: {field_ids}"
        )

    def test_does_not_trigger_on_own_effect_removal(self, debug_runner):
        """Effect should NOT trigger when Metallicdramon is removed by own effects.
        Card text: 'other than by one of your effects'."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX7-049"])
        runner.inject_card(1, "BT2-011", "trash")

        p1 = runner.game.player1

        # Delete by own effect (is_opponent_effect=False -> is_own_effect=True)
        p1.delete_permanent(perm, is_opponent_effect=False)
        runner.auto_resolve()

        snap = runner.snapshot()
        field_ids = [s.card_id for s in snap.p1_field]
        assert "BT2-011" not in field_ids, (
            f"Vorvomon should NOT be played when Metallicdramon is removed by own "
            f"effect. Field: {field_ids}"
        )

    def test_triggers_on_battle_removal(self, debug_runner):
        """Effect SHOULD trigger when Metallicdramon is deleted by battle.
        Battle is not 'one of your effects'."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX7-049"])
        runner.inject_card(1, "BT2-011", "trash")

        p1 = runner.game.player1

        # Delete by battle
        p1.delete_permanent(perm, is_battle=True, removal_cause='battle')

        selected = _select_non_decline(runner)
        assert selected, "WhenRemoveField should trigger on battle removal"

        snap = runner.snapshot()
        field_ids = [s.card_id for s in snap.p1_field]
        assert "BT2-011" in field_ids, (
            f"Vorvomon should be played from trash when Metallicdramon is removed "
            f"by battle. Field: {field_ids}"
        )

    def test_triggers_on_rule_removal(self, debug_runner):
        """Effect SHOULD trigger when Metallicdramon is removed by rule (e.g. DP=0)."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX7-049"])
        runner.inject_card(1, "BT2-011", "trash")

        p1 = runner.game.player1

        # Delete by rule (is_opponent_effect defaults to False,
        # but removal_cause='rule' is not own effect)
        p1.delete_permanent(perm, removal_cause='rule')

        selected = _select_non_decline(runner)
        assert selected, "WhenRemoveField should trigger on rule removal"

        snap = runner.snapshot()
        field_ids = [s.card_id for s in snap.p1_field]
        assert "BT2-011" in field_ids, (
            f"Vorvomon should be played from trash when Metallicdramon is removed "
            f"by rule. Field: {field_ids}"
        )

    def test_plays_earth_dragon_from_trash(self, debug_runner):
        """Effect should also accept Earth Dragon trait Digimon from trash."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX7-049"])
        # BT1-020 Groundramon is Lv.5 Earth Dragon
        runner.inject_card(1, "BT1-020", "trash")

        p1 = runner.game.player1

        p1.delete_permanent(perm, is_opponent_effect=True)

        selected = _select_non_decline(runner)
        assert selected, "WhenRemoveField should trigger for Earth Dragon trash target"

        snap = runner.snapshot()
        field_ids = [s.card_id for s in snap.p1_field]
        assert "BT1-020" in field_ids, (
            f"Groundramon (Earth Dragon) should be played from trash. Field: {field_ids}"
        )

    def test_does_not_play_non_qualifying_digimon(self, debug_runner):
        """Effect should NOT play Digimon without Rock Dragon or Earth Dragon trait."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX7-049"])
        # BT1-009 Agumon has no Rock/Earth Dragon trait
        runner.inject_card(1, "BT1-009", "trash")

        p1 = runner.game.player1

        p1.delete_permanent(perm, is_opponent_effect=True)
        runner.auto_resolve()

        snap = runner.snapshot()
        field_ids = [s.card_id for s in snap.p1_field]
        assert "BT1-009" not in field_ids, (
            f"Non-qualifying Digimon should NOT be played. Field: {field_ids}"
        )

    def test_once_per_turn_limit(self, debug_runner):
        """Effect should only trigger once per turn."""
        runner = debug_runner(initial_memory=5)

        perm1 = runner.place_on_field(1, ["EX7-049"])
        runner.inject_card(1, "BT2-011", "trash")

        p1 = runner.game.player1

        # First removal - should trigger
        p1.delete_permanent(perm1, is_opponent_effect=True)
        selected = _select_non_decline(runner)
        assert selected, "First WhenRemoveField trigger should create a selection"

        snap1 = runner.snapshot()
        field_ids1 = [s.card_id for s in snap1.p1_field]
        assert "BT2-011" in field_ids1, "First trigger should play Vorvomon"

        # Place another Metallicdramon and another trash target
        perm2 = runner.place_on_field(1, ["EX7-049"])
        runner.inject_card(1, "BT1-020", "trash")

        # Second removal same turn - should NOT trigger (once per turn)
        p1.delete_permanent(perm2, is_opponent_effect=True)
        runner.auto_resolve()

        snap2 = runner.snapshot()
        field_ids2 = [s.card_id for s in snap2.p1_field]
        # BT1-020 should still be in trash, not on field
        assert "BT1-020" not in field_ids2, (
            f"Second Metallicdramon's leave trigger should NOT fire (OPT). "
            f"Field: {field_ids2}"
        )

    def test_effect_is_optional(self, debug_runner):
        """The 'you may play' indicates the effect is optional."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX7-049"])
        runner.inject_card(1, "BT2-011", "trash")

        # Verify the effect has is_optional=True
        from digimon_gym.engine.data.enums import EffectTiming
        effects = perm.effect_list(EffectTiming.WhenRemoveField)
        leave_effects = [e for e in effects
                         if "Rock Dragon" in (e.effect_name or "")
                         or "Play Rock" in (e.effect_name or "")]
        assert len(leave_effects) >= 1, (
            f"Should have the WhenRemoveField Rock/Earth Dragon effect. "
            f"Effects: {[(e.effect_name, e.timing) for e in effects]}"
        )
        assert leave_effects[0].is_optional, (
            "The effect should be optional ('you may play')"
        )

    def test_no_trash_target_means_no_trigger(self, debug_runner):
        """If there are no qualifying cards in trash, the effect condition
        should return False and not trigger at all."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX7-049"])
        # No qualifying cards in trash

        p1 = runner.game.player1

        field_before = len(p1.battle_area)
        p1.delete_permanent(perm, is_opponent_effect=True)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Nothing should have been played
        assert len(snap.p1_field) < field_before, (
            "With no qualifying trash cards, nothing should be played"
        )
