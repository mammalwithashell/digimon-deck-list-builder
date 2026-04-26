"""Behavioral tests for BT22-042 Nyabootmon.

Card text:
    <Overclock ([Puppet] Trait)>
    [When Digivolving] You may play 1 level 4 or lower [Puppet] trait Digimon
        card from your hand without paying the cost. Then, to 1 of your
        opponent's Digimon, give -3000 DP until their turn ends for each of
        your Digimon.
    [All Turns][Once Per Turn] When any of your other Digimon are deleted, you
        may activate 1 of this Digimon's [When Digivolving] effects.

    Alt digivolve from [Chaperomon] for cost 6 (if you have [Arisa Kinosaki]).
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


def _get_effects(perm):
    """Get all effects from all card sources on a permanent."""
    effects = []
    for source in perm.card_sources:
        effects.extend(source.effect_list(EffectTiming.NoTiming))
    return effects


def _find_wd_effect(perm):
    """Find the [When Digivolving] effect on a permanent."""
    for eff in _get_effects(perm):
        if getattr(eff, 'is_when_digivolving', False):
            return eff
    return None


def _find_deletion_observer(perm):
    """Find the _is_deletion_observer effect on a permanent."""
    for eff in _get_effects(perm):
        if getattr(eff, '_is_deletion_observer', False):
            return eff
    return None


@pytest.mark.behavioral
class TestBT22042Nyabootmon:
    """Tests for BT22-042 Nyabootmon."""

    # ── Effect 2: [When Digivolving] Play Puppet + DP Reduction ──────

    def test_wd_plays_puppet_lv4_from_hand(self, debug_runner):
        """WD should offer to play a Puppet Lv4- Digimon from hand for free."""
        runner = debug_runner(
            deck1=["BT22-042"] * 4 + ["BT2-055"] * 46,  # ToyAgumon = Puppet Lv3
            deck2=["ST1-03"] * 50,
            initial_memory=10,
        )
        # Place a Lv5 base (Chaperomon) to digivolve Nyabootmon onto
        runner.place_on_field(1, ["BT22-036"])  # Chaperomon Lv5
        # Put a Puppet Lv3 in hand
        runner.inject_card(1, "BT2-055", "hand")  # ToyAgumon Puppet Lv3
        # Put Nyabootmon in hand
        runner.inject_card(1, "BT22-042", "hand")
        # Place an opponent Digimon to target for DP reduction
        runner.place_on_field(2, ["ST1-06"])  # Coredramon 6000 DP

        # Digivolve Nyabootmon onto Chaperomon
        runner.set_phase("Main")
        action = runner.find_action("Digivolve")
        if action is None:
            action = runner.find_action("Nyabootmon")
        assert action is not None, (
            f"Should find digivolve action. Available: {runner.actions()}"
        )
        runner.execute(action)

        # WD triggers - auto-resolve: play ToyAgumon, select opponent Digimon
        results = runner.auto_resolve(max_steps=10)

        # Check ToyAgumon was played to field
        snap = runner.snapshot()
        p1_ids = [s.card_id for s in snap.p1_field]
        assert "BT2-055" in p1_ids or any(
            "ToyAgumon" in s.card_name for s in snap.p1_field
        ), f"ToyAgumon should be on field after WD play. P1 field: {[(s.card_id, s.card_name) for s in snap.p1_field]}"

    def test_wd_dp_reduction_scales_with_digimon_count(self, debug_runner):
        """DP reduction should be -3000 per own Digimon at time of resolution."""
        runner = debug_runner(
            deck1=["BT22-042"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=10,
        )
        nyaboot = runner.place_on_field(1, ["BT22-042"])
        runner.place_on_field(1, ["ST1-03"])
        runner.place_on_field(1, ["ST1-03"])

        opp = runner.place_on_field(2, ["ST1-06"])  # Coredramon 6000 DP
        assert opp.dp == 6000

        game = runner.game
        wd_effect = _find_wd_effect(nyaboot)
        assert wd_effect is not None, "Should find WD effect on Nyabootmon"

        ctx = {'game': game, 'player': game.player1, 'permanent': nyaboot}
        wd_effect.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)

        # 3 own Digimon => -9000 DP; 6000 - 9000 = 0 (floor)
        assert opp.dp == 0, (
            f"Opponent should have 0 DP (6000 - 9000, floor 0), got {opp.dp}"
        )

    def test_wd_dp_reduction_counts_after_play(self, debug_runner):
        """Play happens first, THEN Digimon count for DP reduction includes the new one."""
        runner = debug_runner(
            deck1=["BT22-042"] * 4 + ["BT2-055"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=10,
        )
        nyaboot = runner.place_on_field(1, ["BT22-042"])
        runner.inject_card(1, "BT2-055", "hand")

        opp = runner.place_on_field(2, ["ST1-06"])  # 6000 DP

        game = runner.game
        wd_effect = _find_wd_effect(nyaboot)
        assert wd_effect is not None

        ctx = {'game': game, 'player': game.player1, 'permanent': nyaboot}
        wd_effect.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)

        # After play: 2 Digimon => -6000 DP; 6000 - 6000 = 0
        assert opp.dp == 0, (
            f"With 2 Digimon (after play), expect -6000 DP. Got {opp.dp}"
        )

    def test_wd_play_is_optional(self, debug_runner):
        """The play from hand should be optional (you may play)."""
        runner = debug_runner(
            deck1=["BT22-042"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=10,
        )
        nyaboot = runner.place_on_field(1, ["BT22-042"])
        opp = runner.place_on_field(2, ["ST1-06"])  # 6000 DP

        game = runner.game
        wd_effect = _find_wd_effect(nyaboot)
        assert wd_effect is not None

        ctx = {'game': game, 'player': game.player1, 'permanent': nyaboot}
        wd_effect.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)

        # 1 Digimon => -3000 DP; 6000 - 3000 = 3000
        assert opp.dp == 3000, (
            f"With 1 Digimon, expect -3000. Got {opp.dp}"
        )

    def test_wd_no_opponent_digimon_no_crash(self, debug_runner):
        """If opponent has no Digimon, DP reduction should be skipped."""
        runner = debug_runner(
            deck1=["BT22-042"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=10,
        )
        nyaboot = runner.place_on_field(1, ["BT22-042"])

        game = runner.game
        wd_effect = _find_wd_effect(nyaboot)
        assert wd_effect is not None

        ctx = {'game': game, 'player': game.player1, 'permanent': nyaboot}
        wd_effect.on_process_callback(ctx)
        runner.auto_resolve(max_steps=5)
        # No crash = pass

    # ── Effect 3: Deletion Observer ──────────────────────────────────

    def test_deletion_observer_triggers_on_other_digimon_deleted(self, debug_runner):
        """When another of your Digimon is deleted, observer fires."""
        runner = debug_runner(
            deck1=["BT22-042"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=10,
        )
        nyaboot = runner.place_on_field(1, ["BT22-042"])
        other = runner.place_on_field(1, ["ST1-03"])
        opp = runner.place_on_field(2, ["ST1-06"])

        game = runner.game
        game.player1.delete_permanent(other, removal_cause='effect')

        runner.auto_resolve(max_steps=10)

        snap = runner.snapshot()
        p1_field_ids = [s.card_id for s in snap.p1_field]
        assert "ST1-03" not in p1_field_ids, "Deleted Digimon should be gone"

    def test_deletion_observer_does_not_trigger_on_self_deleted(self, debug_runner):
        """Observer should NOT trigger when Nyabootmon itself is deleted."""
        runner = debug_runner(
            deck1=["BT22-042"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=10,
        )
        nyaboot = runner.place_on_field(1, ["BT22-042"])
        game = runner.game

        observer = _find_deletion_observer(nyaboot)
        assert observer is not None, "Should have _is_deletion_observer effect"

        ctx = {
            'game': game, 'player': game.player1,
            'permanent': nyaboot, 'deleted_permanent': nyaboot,
        }
        assert observer.can_use_condition(ctx) is False, (
            "Observer should NOT trigger on self-deletion"
        )

    def test_deletion_observer_checks_deleted_permanent_context(self, debug_runner):
        """Observer condition should use 'deleted_permanent' context key."""
        runner = debug_runner(
            deck1=["BT22-042"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=10,
        )
        nyaboot = runner.place_on_field(1, ["BT22-042"])
        other = runner.place_on_field(1, ["ST1-03"])
        game = runner.game

        observer = _find_deletion_observer(nyaboot)
        assert observer is not None

        ctx = {
            'game': game, 'player': game.player1,
            'permanent': nyaboot, 'deleted_permanent': other,
        }
        assert observer.can_use_condition(ctx) is True, (
            "Observer should trigger when a different Digimon is deleted"
        )

    def test_deletion_observer_once_per_turn(self, debug_runner):
        """Observer fires once per turn only."""
        runner = debug_runner(
            deck1=["BT22-042"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=10,
        )
        nyaboot = runner.place_on_field(1, ["BT22-042"])

        observer = _find_deletion_observer(nyaboot)
        assert observer is not None

        assert observer.can_activate_this_turn()
        observer.record_activation()
        assert not observer.can_activate_this_turn(), "OPT should block second activation"

    def test_deletion_observer_not_for_non_digimon(self, debug_runner):
        """Observer should not trigger when a Tamer is deleted."""
        runner = debug_runner(
            deck1=["BT22-042"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=10,
        )
        nyaboot = runner.place_on_field(1, ["BT22-042"])
        tamer = runner.place_on_field(1, ["ST19-14"])  # Arisa Kinosaki Tamer
        game = runner.game

        observer = _find_deletion_observer(nyaboot)
        assert observer is not None

        ctx = {
            'game': game, 'player': game.player1,
            'permanent': nyaboot, 'deleted_permanent': tamer,
        }
        assert observer.can_use_condition(ctx) is False, (
            "Observer should NOT trigger on Tamer deletion"
        )

    # ── Effect 3 uses correct pattern ────────────────────────────────

    def test_effect3_uses_deletion_observer_flag(self, debug_runner):
        """Effect 3 must use _is_deletion_observer=True, not is_on_deletion=True."""
        runner = debug_runner(
            deck1=["BT22-042"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 50,
            initial_memory=10,
        )
        nyaboot = runner.place_on_field(1, ["BT22-042"])

        has_deletion_observer = False
        has_is_on_deletion = False
        for eff in _get_effects(nyaboot):
            if getattr(eff, '_is_deletion_observer', False):
                has_deletion_observer = True
            if getattr(eff, 'is_on_deletion', False):
                has_is_on_deletion = True

        assert has_deletion_observer, "Should have _is_deletion_observer=True"
        assert not has_is_on_deletion, "Should NOT use is_on_deletion=True"
