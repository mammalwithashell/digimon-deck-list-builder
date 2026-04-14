"""Behavioral tests for EX11-023 Kaguyamon (Lv.6 Yellow Puppet/LIBERATOR).

Card text:
<Alliance> <Scapegoat>
[When Digivolving] [End of Opponent's Turn] [Once Per Turn] Delete 1 of your
opponent's Digimon with the lowest level.
[All Turns] [Once Per Turn] When other Digimon are deleted, you may play 1
level 4 or lower [Puppet] trait Digimon card from your trash without paying
the cost.

Clause decomposition:
 C1: Alliance keyword
 C2: Scapegoat keyword
 C3: [When Digivolving][Once Per Turn] Delete 1 opponent's lowest level Digimon (MANDATORY)
 C4: [End of Opponent's Turn][Once Per Turn] Delete 1 opponent's lowest level Digimon (MANDATORY)
 C5: WD and EoOT share the SAME Once Per Turn (same hash string)
 C6: [All Turns][Once Per Turn] When OTHER Digimon are deleted, may play Lv4- Puppet from trash
     - Uses _is_deletion_observer pattern
     - Triggers on any Digimon deletion (own or opponent's) except self
     - Optional play from trash
"""

import pytest

from digimon_gym.engine.data.enums import EffectTiming


# Use ST1-03 (Agumon Lv.3) as filler, BT6-082 (Sistermon Blanc, Lv.3 Puppet) for trash play target
# BT8-042 = Shakkoumon (Yellow Lv.5) for digivolve base (EX11-023 requires Yellow Lv.5)
# BT1-087 = T.K. Takaishi (Tamer) for non-Digimon deletion test
FILLER = ["ST1-03"] * 50
YELLOW_LV5 = "BT8-042"  # Shakkoumon, Yellow Lv.5 — valid digivolve base for Kaguyamon


@pytest.mark.behavioral
class TestEX11023Keywords:
    """C1+C2: Alliance and Scapegoat keywords."""

    def test_alliance_keyword_present(self, debug_runner):
        """Kaguyamon should have Alliance keyword in effects."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        perm = runner.place_on_field(1, [YELLOW_LV5, "EX11-023"])
        effects = perm.top_card.effect_list(None)
        alliance_effects = [e for e in effects if getattr(e, '_is_alliance', False)]
        assert len(alliance_effects) >= 1, "EX11-023 should have Alliance keyword"

    def test_scapegoat_keyword_present(self, debug_runner):
        """Kaguyamon should have Scapegoat keyword in effects."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        perm = runner.place_on_field(1, [YELLOW_LV5, "EX11-023"])
        effects = perm.top_card.effect_list(None)
        scapegoat_effects = [e for e in effects if getattr(e, '_is_scapegoat', False)]
        assert len(scapegoat_effects) >= 1, "EX11-023 should have Scapegoat keyword"


@pytest.mark.behavioral
class TestEX11023WhenDigivolving:
    """C3: [When Digivolving] Delete 1 opponent's lowest level Digimon."""

    def test_wd_fires_on_digivolve(self, debug_runner):
        """When Digivolving should trigger delete selection for opponent's lowest level."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        # Place opponent Digimon (ST1-03 = Lv.3, ST1-05 = Lv.4)
        runner.place_on_field(2, ["ST1-03"])  # lowest level
        runner.place_on_field(2, ["ST1-05"])  # higher level
        # Digivolve base (Yellow Lv.5) + Kaguyamon in hand
        runner.inject_card(1, "EX11-023", "hand")
        runner.place_on_field(1, [YELLOW_LV5])  # Yellow Lv.5 base
        runner.set_phase("Main")

        action = runner.find_action("Digivolve")
        assert action is not None, "Should find digivolve action for Kaguyamon"
        runner.execute(action)

        # Should be in selection phase to delete opponent Digimon
        snap = runner.snapshot()
        assert snap.phase == "SelectTarget", (
            f"Should be in SelectTarget for delete selection, got {snap.phase}"
        )

    def test_wd_delete_is_mandatory(self, debug_runner):
        """Delete selection is MANDATORY (canNoSelect: false in C#)."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        runner.place_on_field(2, ["ST1-03"])
        runner.inject_card(1, "EX11-023", "hand")
        runner.place_on_field(1, [YELLOW_LV5])
        runner.set_phase("Main")

        action = runner.find_action("Digivolve")
        runner.execute(action)

        ps = runner.game.pending_selection
        assert ps is not None, "Should have pending selection"
        assert ps.is_optional is False, (
            f"Delete selection should be mandatory (canNoSelect=false), got is_optional={ps.is_optional}"
        )

    def test_wd_only_lowest_level_selectable(self, debug_runner):
        """Only opponent's lowest level Digimon should be selectable."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        # Place Lv.3 and Lv.5 opponents
        runner.place_on_field(2, ["ST1-03"])  # Lv.3 - lowest
        runner.place_on_field(2, ["ST1-08"])  # Lv.5 - higher
        runner.inject_card(1, "EX11-023", "hand")
        runner.place_on_field(1, [YELLOW_LV5])
        runner.set_phase("Main")

        action = runner.find_action("Digivolve")
        runner.execute(action)

        ps = runner.game.pending_selection
        assert ps is not None, "Should have pending selection"
        from digimon_gym.engine.game.constants import SEL_OPP_FIELD_START
        opp_targets = [v for v in ps.valid_indices if v >= SEL_OPP_FIELD_START]
        assert len(opp_targets) == 1, (
            f"Should have exactly 1 valid target (Lv.3 only), got {len(opp_targets)}"
        )

    def test_wd_deletes_opponent_digimon(self, debug_runner):
        """Selecting a target should actually delete it."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        runner.place_on_field(2, ["ST1-03"])  # Lv.3 target
        runner.inject_card(1, "EX11-023", "hand")
        runner.place_on_field(1, [YELLOW_LV5])
        runner.set_phase("Main")

        action = runner.find_action("Digivolve")
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert len(snap.p2_field) == 0, (
            f"Opponent's Digimon should be deleted. P2 field: "
            f"{[(s.card_id, s.card_name) for s in snap.p2_field]}"
        )

    def test_wd_no_opponent_digimon_no_selection(self, debug_runner):
        """If opponent has no Digimon, no selection should appear."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        # No opponent Digimon
        runner.inject_card(1, "EX11-023", "hand")
        runner.place_on_field(1, [YELLOW_LV5])
        runner.set_phase("Main")

        action = runner.find_action("Digivolve")
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Should return to Main without any selection
        assert snap.phase == "Main", (
            f"Should return to Main phase when no opponents, got {snap.phase}"
        )


@pytest.mark.behavioral
class TestEX11023EndOfOpponentTurn:
    """C4: [End of Opponent's Turn] Delete 1 opponent's lowest level Digimon."""

    def test_eoot_has_end_turn_effect(self, debug_runner):
        """Kaguyamon should have an OnEndTurn timing effect."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        perm = runner.place_on_field(1, [YELLOW_LV5, "EX11-023"])
        effects = perm.top_card.effect_list(EffectTiming.OnEndTurn)
        end_turn_effects = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(end_turn_effects) >= 1, "EX11-023 should have End Turn effect"

    def test_eoot_only_on_opponent_turn(self, debug_runner):
        """End of Turn effect should only activate during opponent's turn."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        perm = runner.place_on_field(1, [YELLOW_LV5, "EX11-023"])
        effects = perm.top_card.effect_list(EffectTiming.OnEndTurn)
        end_turn_effects = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(end_turn_effects) >= 1

        # Simulate it being player 1's turn (Kaguyamon's owner)
        runner.game.player1.is_my_turn = True
        runner.game.player2.is_my_turn = False

        ctx = {'game': runner.game, 'player': runner.game.player1}
        effect = end_turn_effects[0]
        # Condition should be False on own turn
        result = effect.can_use_condition(ctx)
        assert result is False, (
            "EoOT effect should NOT activate on own turn"
        )

    def test_eoot_activates_on_opponent_turn(self, debug_runner):
        """End of Turn effect should activate during opponent's turn."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        perm = runner.place_on_field(1, [YELLOW_LV5, "EX11-023"])
        effects = perm.top_card.effect_list(EffectTiming.OnEndTurn)
        end_turn_effects = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(end_turn_effects) >= 1

        # Simulate opponent's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        ctx = {'game': runner.game, 'player': runner.game.player1}
        effect = end_turn_effects[0]
        result = effect.can_use_condition(ctx)
        assert result is True, (
            "EoOT effect should activate on opponent's turn"
        )


@pytest.mark.behavioral
class TestEX11023SharedOPT:
    """C5: WD and EoOT share the same Once Per Turn."""

    def test_shared_hash_string(self, debug_runner):
        """WD and EoOT delete effects should share the same hash string for OPT."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        perm = runner.place_on_field(1, [YELLOW_LV5, "EX11-023"])

        # Get WD effect (OnEnterFieldAnyone with is_when_digivolving)
        all_effects = perm.top_card.effect_list(None)
        wd_effects = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        assert len(wd_effects) >= 1, "Should have WD effect"

        # Get EoOT effect (OnEndTurn)
        eoot_effects = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEndTurn
        ]
        assert len(eoot_effects) >= 1, "Should have EoOT effect"

        wd_hash = wd_effects[0].hash_string
        eoot_hash = eoot_effects[0].hash_string
        assert wd_hash == eoot_hash, (
            f"WD and EoOT should share the same hash string for OPT. "
            f"WD hash: {wd_hash}, EoOT hash: {eoot_hash}"
        )

    def test_wd_then_eoot_blocked_by_opt(self, debug_runner):
        """If WD activates this turn, EoOT should be blocked by shared OPT."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        runner.place_on_field(2, ["ST1-03"])  # target
        runner.inject_card(1, "EX11-023", "hand")
        runner.place_on_field(1, [YELLOW_LV5])
        runner.set_phase("Main")

        # Digivolve triggers WD -> deletes target
        action = runner.find_action("Digivolve")
        runner.execute(action)
        runner.auto_resolve()

        # Place new target for EoOT
        runner.place_on_field(2, ["ST1-03"])

        # Now check if EoOT can activate
        perm = None
        for p in runner.game.player1.battle_area:
            if p.top_card and p.top_card.c_entity_base and p.top_card.c_entity_base.card_id == "EX11-023":
                perm = p
                break
        assert perm is not None, "Kaguyamon should be on field"

        effects = perm.top_card.effect_list(EffectTiming.OnEndTurn)
        eoot_effects = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(eoot_effects) >= 1

        # The effect's can_activate_this_turn should be False (OPT already used)
        can_activate = eoot_effects[0].can_activate_this_turn()
        assert can_activate is False, (
            "EoOT should be blocked by shared OPT after WD activated this turn"
        )


@pytest.mark.behavioral
class TestEX11023DeletionObserver:
    """C6: [All Turns][Once Per Turn] When other Digimon are deleted, play Puppet from trash."""

    def test_deletion_observer_uses_correct_pattern(self, debug_runner):
        """Effect should use _is_deletion_observer=True, not OnDestroyedAnyone timing.

        OnDestroyedAnyone fires on the deleted permanent's own card sources.
        _is_deletion_observer fires on field permanents observing the deletion.
        """
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        perm = runner.place_on_field(1, [YELLOW_LV5, "EX11-023"])
        all_effects = perm.top_card.effect_list(None)

        # Find the deletion observer effect
        observer_effects = [
            e for e in all_effects
            if getattr(e, '_is_deletion_observer', False)
        ]
        assert len(observer_effects) >= 1, (
            "EX11-023 should have _is_deletion_observer=True effect for 'when other Digimon are deleted'"
        )

    def test_deletion_observer_triggers_on_opponent_deletion(self, debug_runner):
        """When an opponent's Digimon is deleted, observer should fire and allow
        playing a Puppet from trash."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        kaguya_perm = runner.place_on_field(1, [YELLOW_LV5, "EX11-023"])
        opp_perm = runner.place_on_field(2, ["ST1-03"])  # opponent Digimon to delete

        # Put a Puppet Digimon in player 1's trash
        runner.inject_card(1, "BT6-082", "trash")  # Sistermon Blanc, Lv.3 Puppet

        # Delete opponent's Digimon
        runner.game.player2.delete_permanent(opp_perm)

        # Should enter SelectTarget for the trash play
        snap = runner.snapshot()
        assert snap.phase == "SelectTarget", (
            f"Should be in SelectTarget after deletion, got {snap.phase}"
        )

        # Select the Puppet card from trash (not Decline)
        ps = runner.game.pending_selection
        assert ps is not None, "Should have pending selection"
        from digimon_gym.engine.game.constants import SEL_TRASH_START
        trash_targets = [v for v in ps.valid_indices if v >= SEL_TRASH_START]
        assert len(trash_targets) >= 1, "Should have trash targets"
        runner.execute(trash_targets[0])
        runner.auto_resolve()

        snap = runner.snapshot()
        puppet_on_field = any(
            s.card_id == "BT6-082" for s in snap.p1_field
        )
        assert puppet_on_field, (
            f"Puppet Digimon (BT6-082) should be played from trash after opponent's Digimon deleted. "
            f"P1 field: {[(s.card_id, s.card_name) for s in snap.p1_field]}, "
            f"P1 trash: {snap.p1_trash}"
        )

    def test_deletion_observer_triggers_on_own_digimon_deletion(self, debug_runner):
        """When one of your own (other) Digimon is deleted, observer should fire."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        kaguya_perm = runner.place_on_field(1, [YELLOW_LV5, "EX11-023"])
        own_perm = runner.place_on_field(1, ["ST1-03"])  # own Digimon to delete

        # Put a Puppet Digimon in player 1's trash
        runner.inject_card(1, "BT6-082", "trash")

        # Delete own Digimon
        runner.game.player1.delete_permanent(own_perm)

        # Should enter SelectTarget for the trash play
        snap = runner.snapshot()
        assert snap.phase == "SelectTarget", (
            f"Should be in SelectTarget after own Digimon deletion, got {snap.phase}"
        )

        # Select the Puppet card from trash
        ps = runner.game.pending_selection
        from digimon_gym.engine.game.constants import SEL_TRASH_START
        trash_targets = [v for v in ps.valid_indices if v >= SEL_TRASH_START]
        assert len(trash_targets) >= 1, "Should have trash targets"
        runner.execute(trash_targets[0])
        runner.auto_resolve()

        snap = runner.snapshot()
        puppet_on_field = any(
            s.card_id == "BT6-082" for s in snap.p1_field
        )
        assert puppet_on_field, (
            f"Puppet should be played from trash when own Digimon is deleted. "
            f"P1 field: {[(s.card_id, s.card_name) for s in snap.p1_field]}, "
            f"P1 trash: {snap.p1_trash}"
        )

    def test_deletion_observer_does_not_trigger_on_self_deletion(self, debug_runner):
        """When Kaguyamon itself is deleted, observer should NOT fire."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        kaguya_perm = runner.place_on_field(1, [YELLOW_LV5, "EX11-023"])

        # Put a Puppet Digimon in player 1's trash
        runner.inject_card(1, "BT6-082", "trash")

        # Delete Kaguyamon itself
        runner.game.player1.delete_permanent(kaguya_perm)
        runner.auto_resolve()

        snap = runner.snapshot()
        puppet_on_field = any(
            s.card_id == "BT6-082" for s in snap.p1_field
        )
        assert not puppet_on_field, (
            "Puppet should NOT be played from trash when Kaguyamon itself is deleted "
            "(it says 'other Digimon')"
        )

    def test_deletion_observer_does_not_trigger_on_non_digimon(self, debug_runner):
        """Observer should NOT trigger when a Tamer is deleted (card says 'Digimon')."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        runner.place_on_field(1, [YELLOW_LV5, "EX11-023"])

        # Place a Tamer on opponent's field and delete it
        runner.place_on_field(2, ["BT1-087"])  # BT1-087 = T.K. Takaishi (Tamer)

        runner.inject_card(1, "BT6-082", "trash")

        # Delete the Tamer
        opp_tamer = runner.game.player2.battle_area[-1]
        runner.game.player2.delete_permanent(opp_tamer)
        runner.auto_resolve()

        snap = runner.snapshot()
        puppet_on_field = any(
            s.card_id == "BT6-082" for s in snap.p1_field
        )
        assert not puppet_on_field, (
            "Puppet should NOT be played when a non-Digimon (Tamer) is deleted"
        )

    def test_deletion_observer_once_per_turn(self, debug_runner):
        """Observer should only fire once per turn."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        kaguya_perm = runner.place_on_field(1, [YELLOW_LV5, "EX11-023"])
        opp1 = runner.place_on_field(2, ["ST1-03"])
        opp2 = runner.place_on_field(2, ["ST1-03"])

        # Put 2 Puppet Digimon in trash
        runner.inject_card(1, "BT6-082", "trash")
        runner.inject_card(1, "BT6-082", "trash")

        # Delete first opponent Digimon -> observer fires, select the trash card
        runner.game.player2.delete_permanent(opp1)
        snap = runner.snapshot()
        if snap.phase == "SelectTarget":
            ps = runner.game.pending_selection
            from digimon_gym.engine.game.constants import SEL_TRASH_START
            trash_targets = [v for v in ps.valid_indices if v >= SEL_TRASH_START]
            if trash_targets:
                runner.execute(trash_targets[0])
        runner.auto_resolve()

        # Delete second opponent Digimon -> observer should NOT fire (OPT)
        runner.game.player2.delete_permanent(opp2)
        runner.auto_resolve()

        snap = runner.snapshot()
        puppet_count = sum(1 for s in snap.p1_field if s.card_id == "BT6-082")
        assert puppet_count == 1, (
            f"Should play only 1 Puppet from trash (OPT), got {puppet_count}"
        )

    def test_deletion_observer_optional(self, debug_runner):
        """The play-from-trash should be optional (card says 'you may play')."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        kaguya_perm = runner.place_on_field(1, [YELLOW_LV5, "EX11-023"])

        all_effects = kaguya_perm.top_card.effect_list(None)
        observer_effects = [
            e for e in all_effects
            if getattr(e, '_is_deletion_observer', False)
        ]
        assert len(observer_effects) >= 1
        assert observer_effects[0].is_optional is True, (
            "Deletion observer effect should be optional ('you may play')"
        )

    def test_deletion_observer_no_puppet_in_trash_no_fire(self, debug_runner):
        """If no eligible Puppet Lv.4- in trash, observer should not produce selection."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        kaguya_perm = runner.place_on_field(1, [YELLOW_LV5, "EX11-023"])
        opp_perm = runner.place_on_field(2, ["ST1-03"])

        # Trash is empty (no Puppet cards)
        runner.clear_zone(1, "trash")

        runner.game.player2.delete_permanent(opp_perm)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Should return to normal phase without error
        assert snap.phase == "Main" or snap.phase == "End", (
            f"Should not hang in SelectTarget with no valid targets, got {snap.phase}"
        )

    def test_deletion_observer_only_level4_or_lower_puppet(self, debug_runner):
        """Should only allow playing Digimon that are Lv.4 or lower AND have Puppet trait."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=10)
        kaguya_perm = runner.place_on_field(1, [YELLOW_LV5, "EX11-023"])
        opp_perm = runner.place_on_field(2, ["ST1-03"])

        # Put non-Puppet in trash (ST1-03 = Agumon, Reptile, Lv.3)
        runner.inject_card(1, "ST1-03", "trash")
        # Put a Lv.5 Puppet in trash (won't qualify)
        runner.inject_card(1, "BT1-038", "trash")  # Monzaemon Lv.5 Puppet
        # Put a Lv.3 Puppet (DOES qualify)
        runner.inject_card(1, "BT6-082", "trash")

        runner.game.player2.delete_permanent(opp_perm)

        # If a selection phase appears, check that only BT6-082 is valid
        snap = runner.snapshot()
        if snap.phase == "SelectTarget":
            ps = runner.game.pending_selection
            assert ps is not None
            from digimon_gym.engine.game.constants import SEL_TRASH_START
            trash_targets = [v for v in ps.valid_indices if v >= SEL_TRASH_START]
            # Only BT6-082 should be valid (Lv.3 Puppet)
            assert len(trash_targets) == 1, (
                f"Should have exactly 1 valid trash target (Lv.3 Puppet only), got {len(trash_targets)}"
            )

        runner.auto_resolve()
