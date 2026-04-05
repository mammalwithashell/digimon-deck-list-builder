"""Behavioral tests for BT22-040 Cendrillmon (Lv.6 Yellow Puppet/LIBERATOR).

Card text:
<Overclock ([Puppet] Trait)>
[On Play] [When Digivolving] You may play 1 [Familiar] Token.
    (Digimon/Yellow/3000 DP/[On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn.)
[All Turns] [Once Per Turn] When any of your other Digimon are deleted,
    you may activate 1 of this Digimon's [When Digivolving] effects.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT22040Cendrillmon:
    """Tests for BT22-040 Cendrillmon."""

    # ------------------------------------------------------------------ #
    # Effect 3: deletion observer — structure checks
    # ------------------------------------------------------------------ #

    def test_deletion_observer_uses_correct_flag(self, debug_runner):
        """Effect 3 must be a deletion observer (_is_deletion_observer=True),
        NOT is_on_deletion."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT22-040"])

        # Find the deletion observer effect
        obs_effects = []
        for source in perm.card_sources:
            for eff in source.effect_list(None):
                if getattr(eff, '_is_deletion_observer', False):
                    obs_effects.append(eff)

        assert len(obs_effects) >= 1, (
            "BT22-040 should have a _is_deletion_observer effect"
        )

        # Confirm NO effect uses is_on_deletion (that's for the deleted perm's
        # own on-deletion effects, not for observing other deletions)
        for source in perm.card_sources:
            for eff in source.effect_list(None):
                if "Activate" in (eff.effect_name or "") or "WD" in (eff.effect_name or ""):
                    assert not getattr(eff, 'is_on_deletion', False), (
                        "Deletion observer must NOT use is_on_deletion"
                    )

    # ------------------------------------------------------------------ #
    # Effect 3: triggers on own other Digimon deletion
    # ------------------------------------------------------------------ #

    def test_deletion_observer_triggers_on_own_digimon_deleted(self, debug_runner):
        """When one of player 1's other Digimon is deleted, Cendrillmon's
        observer should trigger and play a Familiar Token."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        cendrillmon = runner.place_on_field(1, ["BT22-040"])
        fodder = runner.place_on_field(1, ["BT22-040"])  # another Digimon

        field_before = len(game.player1.battle_area)

        # Delete the fodder — should trigger Cendrillmon's observer
        game.player1.delete_permanent(fodder)

        # Cendrillmon + familiar token should be on field
        # (fodder is gone, but a token was played)
        remaining_names = [
            (p.top_card.card_names[0] if p.top_card and p.top_card.card_names else "?")
            for p in game.player1.battle_area
        ]
        tokens = [p for p in game.player1.battle_area if getattr(p, 'is_token', False)]
        assert len(tokens) >= 1, (
            f"Familiar Token should have been played. Field: {remaining_names}"
        )

    # ------------------------------------------------------------------ #
    # Effect 3: does NOT trigger on opponent's Digimon deletion
    # ------------------------------------------------------------------ #

    def test_deletion_observer_no_trigger_on_opponent_digimon(self, debug_runner):
        """When an opponent's Digimon is deleted, Cendrillmon should NOT trigger."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        cendrillmon = runner.place_on_field(1, ["BT22-040"])
        opp_fodder = runner.place_on_field(2, ["BT22-040"])

        p1_field_before = len(game.player1.battle_area)

        # Delete opponent's Digimon
        game.player2.delete_permanent(opp_fodder)

        # No token should have been played on P1's field
        tokens = [p for p in game.player1.battle_area if getattr(p, 'is_token', False)]
        assert len(tokens) == 0, (
            "No token should be played when opponent's Digimon is deleted"
        )

    # ------------------------------------------------------------------ #
    # Effect 3: does NOT trigger on self-deletion
    # ------------------------------------------------------------------ #

    def test_deletion_observer_no_trigger_on_self_deletion(self, debug_runner):
        """When Cendrillmon itself is deleted, its observer should NOT trigger."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        cendrillmon = runner.place_on_field(1, ["BT22-040"])

        # Delete Cendrillmon itself
        game.player1.delete_permanent(cendrillmon)

        # No token should appear (Cendrillmon is gone, no observer to fire)
        tokens = [p for p in game.player1.battle_area if getattr(p, 'is_token', False)]
        assert len(tokens) == 0, (
            "No token should be played when Cendrillmon itself is deleted"
        )

    # ------------------------------------------------------------------ #
    # Effect 3: once per turn
    # ------------------------------------------------------------------ #

    def test_deletion_observer_once_per_turn(self, debug_runner):
        """The observer is [Once Per Turn] — deleting two Digimon should only
        produce one token from Cendrillmon's observer."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        cendrillmon = runner.place_on_field(1, ["BT22-040"])
        # Use generic Digimon for fodder (not BT22-040) to avoid extra observers
        fodder1 = runner.place_on_field(1, ["BT22-001"])  # Puyoyomon
        fodder2 = runner.place_on_field(1, ["BT22-001"])  # Puyoyomon

        # Delete both fodder Digimon
        game.player1.delete_permanent(fodder1)
        game.player1.delete_permanent(fodder2)

        tokens = [p for p in game.player1.battle_area if getattr(p, 'is_token', False)]
        assert len(tokens) == 1, (
            f"Only 1 token should be played (once per turn). Got {len(tokens)} tokens."
        )

    # ------------------------------------------------------------------ #
    # Effect 3: does NOT trigger on non-Digimon deletion (e.g. Tamer)
    # ------------------------------------------------------------------ #

    def test_deletion_observer_no_trigger_on_tamer_deletion(self, debug_runner):
        """When a Tamer (non-Digimon) is deleted, the observer should NOT trigger.
        Card text says 'when any of your other Digimon are deleted'."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        cendrillmon = runner.place_on_field(1, ["BT22-040"])
        # Place a tamer on field — BT22-083 Yuuko Kamishiro is a Tamer
        tamer = runner.place_on_field(1, ["BT22-083"])

        # Delete the tamer
        game.player1.delete_permanent(tamer)

        tokens = [p for p in game.player1.battle_area if getattr(p, 'is_token', False)]
        assert len(tokens) == 0, (
            "No token should be played when a Tamer is deleted"
        )

    # ------------------------------------------------------------------ #
    # Effects 1 & 2: [On Play] / [When Digivolving] play Familiar Token
    # ------------------------------------------------------------------ #

    def test_on_play_plays_familiar_token(self, debug_runner):
        """[On Play] effect should play a Familiar Token."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        perm = runner.place_on_field(1, ["BT22-040"])

        # Find the on-play effect
        on_play_effects = []
        for source in perm.card_sources:
            for eff in source.effect_list(EffectTiming.OnEnterFieldAnyone):
                if getattr(eff, 'is_on_play', False):
                    on_play_effects.append(eff)

        assert len(on_play_effects) == 1, "Should have exactly 1 On Play effect"
        eff = on_play_effects[0]
        assert eff.is_optional, "On Play token effect should be optional"

        # Execute it
        eff.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        tokens = [p for p in game.player1.battle_area if getattr(p, 'is_token', False)]
        assert len(tokens) == 1, "Should have played 1 Familiar Token"

    def test_when_digivolving_plays_familiar_token(self, debug_runner):
        """[When Digivolving] effect should play a Familiar Token."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        perm = runner.place_on_field(1, ["BT22-040"])

        # Find the when-digivolving effect
        wd_effects = []
        for source in perm.card_sources:
            for eff in source.effect_list(EffectTiming.OnEnterFieldAnyone):
                if getattr(eff, 'is_when_digivolving', False):
                    wd_effects.append(eff)

        assert len(wd_effects) == 1, "Should have exactly 1 When Digivolving effect"
        eff = wd_effects[0]
        assert eff.is_optional, "When Digivolving token effect should be optional"

        # Execute it
        eff.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        tokens = [p for p in game.player1.battle_area if getattr(p, 'is_token', False)]
        assert len(tokens) == 1, "Should have played 1 Familiar Token"
