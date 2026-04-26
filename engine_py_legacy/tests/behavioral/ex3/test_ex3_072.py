"""Behavioral tests for EX3-072 Megiddo Flame (Option, Purple, Cost 4).

Card text:
  [Main] Delete 1 of your opponent's level 4 or lower Digimon. By
      deleting 1 of your Digimon, delete 1 of your opponent's level 6
      or lower Digimon instead.
  [Security] You may play 1 [Guilmon] from your trash without paying
      the cost.

C# reference behavior:
  - OptionSkill: first optionally select own Digimon to delete (canNoSelect: true)
  - If own Digimon is deleted, maxLevel upgrades from 4 to 6
  - Then mandatory delete of opponent's Digimon with level <= maxLevel
  - SecuritySkill: optional play of [Guilmon] from trash (free)
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase
from digimon_gym.engine.game.constants import (
    SEL_MY_FIELD_START, SEL_OPP_FIELD_START,
)


FILLER_DECK = ["BT2-067"] * 50  # Purple Lv.3


@pytest.mark.behavioral
class TestEX3072MegiddoFlameEffectStructure:
    """Structural tests for EX3-072 effect definitions."""

    def _get_effects(self, runner):
        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source("EX3-072", runner.game.player1)
        return cs.effect_list(None)

    def test_has_option_skill_effect(self, debug_runner):
        """Should have exactly 1 OptionSkill timing effect."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        effects = self._get_effects(runner)
        option_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(option_effects) == 1, "Should have exactly 1 OptionSkill effect"

    def test_has_security_skill_effect(self, debug_runner):
        """Should have exactly 1 SecuritySkill timing effect."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        effects = self._get_effects(runner)
        security_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(security_effects) == 1, "Should have exactly 1 SecuritySkill effect"
        assert security_effects[0].is_security_effect, "Should be marked as security effect"


@pytest.mark.behavioral
class TestEX3072MegiddoFlameMainEffect:
    """Tests for the [Main] deletion effect."""

    def _trigger_option_skill(self, runner):
        """Directly invoke the OptionSkill process callback."""
        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()
        player = runner.game.player1
        cs = db.create_card_source("EX3-072", player)
        effects = cs.effect_list(None)
        option_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(option_effects) == 1
        option_effects[0].on_process_callback({
            'player': player,
            'game': runner.game,
        })

    def test_base_delete_lv4_or_lower_no_own_digimon(self, debug_runner):
        """Without own Digimon, should directly delete opponent's lv4 or lower."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        # Place opponent's lv3 Digimon (should be deletable)
        opp_target = runner.place_on_field(2, ["ST1-03"])  # Agumon Lv3

        # Player 1 has NO Digimon on field
        self._trigger_option_skill(runner)

        # Should go directly to selecting opponent's Digimon (no own selection)
        assert game.current_phase == GamePhase.SelectTarget, (
            f"Should be in SelectTarget for opponent's Digimon, got {game.current_phase}"
        )

        # Select opponent's Agumon (slot 0)
        opp_action = SEL_OPP_FIELD_START + 0
        assert opp_action in runner.action_mask(), (
            f"Should be able to select opponent's Agumon. Legal: {runner.actions()}"
        )
        runner.execute(opp_action)

        # Agumon should be deleted
        snap = runner.snapshot()
        assert len(snap.p2_field) == 0, "Opponent's Agumon should be deleted"
        assert "ST1-03" in snap.p2_trash, "Agumon should be in opponent's trash"

    def test_base_delete_lv4_target_is_valid(self, debug_runner):
        """Level 4 Digimon should be a valid target at base threshold."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        runner.place_on_field(2, ["ST1-07"])  # Greymon Lv4

        self._trigger_option_skill(runner)

        # Should be in SelectTarget for opponent
        assert game.current_phase == GamePhase.SelectTarget

        opp_action = SEL_OPP_FIELD_START + 0
        assert opp_action in runner.action_mask(), (
            "Lv4 Greymon should be a valid target"
        )

    def test_base_delete_lv5_not_targetable(self, debug_runner):
        """Level 5 Digimon should NOT be targetable at base lv4 threshold."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        runner.place_on_field(2, ["ST1-08"])  # Garudamon Lv5

        self._trigger_option_skill(runner)

        # With no valid targets at lv4, the effect should not create a selection
        # (or should skip past it). Either way, we should not be in SelectTarget
        # for opponent, OR if we are, lv5 should not be valid.
        if game.current_phase == GamePhase.SelectTarget:
            opp_action = SEL_OPP_FIELD_START + 0
            assert opp_action not in runner.action_mask(), (
                "Lv5 Garudamon should NOT be a valid target at base lv4 threshold"
            )

    def test_upgrade_delete_own_then_lv6(self, debug_runner):
        """Deleting own Digimon should upgrade threshold to lv6."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        # P1 has a Digimon to sacrifice
        own_mon = runner.place_on_field(1, ["ST1-03"])  # Agumon Lv3 (slot 0)
        # P2 has a Lv5 Digimon (only deletable with upgrade)
        opp_target = runner.place_on_field(2, ["ST1-08"])  # Garudamon Lv5

        self._trigger_option_skill(runner)

        # First: optional selection of own Digimon to delete
        assert game.current_phase == GamePhase.SelectTarget, (
            f"Should be in SelectTarget for own Digimon, got {game.current_phase}"
        )

        # Select own Agumon to sacrifice (slot 0)
        own_action = SEL_MY_FIELD_START + 0
        assert own_action in runner.action_mask(), (
            f"Should be able to select own Agumon. Legal: {runner.actions()}"
        )
        runner.execute(own_action)

        # Now should be selecting opponent's Digimon with upgraded lv6 threshold
        assert game.current_phase == GamePhase.SelectTarget, (
            f"Should be in SelectTarget for opponent's Digimon, got {game.current_phase}"
        )

        # Garudamon (Lv5) should now be valid
        opp_action = SEL_OPP_FIELD_START + 0
        assert opp_action in runner.action_mask(), (
            f"Lv5 Garudamon should be valid with upgraded lv6 threshold. Legal: {runner.actions()}"
        )
        runner.execute(opp_action)

        snap = runner.snapshot()
        # Own Agumon deleted
        assert "ST1-03" in snap.p1_trash, "Own Agumon should be in trash"
        # Opponent Garudamon deleted
        assert len(snap.p2_field) == 0, "Opponent's Garudamon should be deleted"
        assert "ST1-08" in snap.p2_trash, "Garudamon should be in opponent's trash"

    def test_upgrade_allows_lv6_target(self, debug_runner):
        """Lv6 Digimon should be targetable after upgrade."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        own_mon = runner.place_on_field(1, ["ST1-03"])  # sacrifice
        runner.place_on_field(2, ["ST1-10"])  # Phoenixmon Lv6

        self._trigger_option_skill(runner)

        # Select own Digimon to sacrifice
        own_action = SEL_MY_FIELD_START + 0
        runner.execute(own_action)

        # Lv6 Phoenixmon should be targetable after upgrade
        opp_action = SEL_OPP_FIELD_START + 0
        assert opp_action in runner.action_mask(), (
            "Lv6 Phoenixmon should be valid after upgrade"
        )

    def test_decline_own_deletion_uses_base_lv4(self, debug_runner):
        """Declining to delete own Digimon should use base lv4 threshold."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        # P1 has a Digimon (could sacrifice but won't)
        own_mon = runner.place_on_field(1, ["ST1-03"])  # Agumon Lv3
        # P2 has both a lv3 and a lv5 Digimon
        runner.place_on_field(2, ["ST1-03"])   # Agumon Lv3 (slot 0)
        runner.place_on_field(2, ["ST1-08"])   # Garudamon Lv5 (slot 1)

        self._trigger_option_skill(runner)

        # First: optional own Digimon selection - DECLINE (action 62)
        assert game.current_phase == GamePhase.SelectTarget, (
            f"Should be in SelectTarget for own Digimon, got {game.current_phase}"
        )
        # Verify decline is available (optional selection)
        assert 62 in runner.action_mask(), (
            f"Should be able to decline own Digimon deletion. Legal: {runner.actions()}"
        )
        runner.execute(62)  # Decline

        # Now should select opponent's Digimon at base lv4 threshold
        assert game.current_phase == GamePhase.SelectTarget, (
            f"Should be in SelectTarget for opponent's Digimon, got {game.current_phase}"
        )

        # Lv3 Agumon should be valid
        opp_agumon = SEL_OPP_FIELD_START + 0
        assert opp_agumon in runner.action_mask(), (
            "Lv3 Agumon should be valid at base lv4 threshold"
        )

        # Lv5 Garudamon should NOT be valid (base lv4 threshold)
        opp_garudamon = SEL_OPP_FIELD_START + 1
        assert opp_garudamon not in runner.action_mask(), (
            "Lv5 Garudamon should NOT be valid at base lv4 threshold"
        )

        # Select Agumon
        runner.execute(opp_agumon)

        snap = runner.snapshot()
        # Own Agumon should still be on field (not deleted)
        p1_digimon = [f for f in snap.p1_field if f.is_digimon]
        assert any(f.card_id == "ST1-03" for f in p1_digimon), (
            "Own Agumon should still be on field (was not sacrificed)"
        )
        # Opponent's Agumon deleted, Garudamon remains
        assert any(f.card_id == "ST1-08" for f in snap.p2_field), (
            "Opponent's Garudamon should still be on field"
        )

    def test_own_selection_is_optional_not_branch(self, debug_runner):
        """Own Digimon selection should be optional (decline available), not a branch choice.

        C# uses canNoSelect=true for own Digimon selection, NOT a branch choice.
        The Python should use effect_select_own_permanent with is_optional=True.
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        own_mon = runner.place_on_field(1, ["ST1-03"])
        runner.place_on_field(2, ["ST1-03"])

        self._trigger_option_skill(runner)

        assert game.current_phase == GamePhase.SelectTarget, (
            f"Should be in SelectTarget (not branch choice), got {game.current_phase}"
        )

        # In a proper implementation, the valid actions should include
        # own field permanents AND action 62 (decline)
        legal = runner.action_mask()
        own_slot = SEL_MY_FIELD_START + 0
        assert own_slot in legal, (
            f"Own Digimon should be selectable. Legal: {runner.actions()}"
        )
        assert 62 in legal, (
            f"Decline should be available (optional). Legal: {runner.actions()}"
        )

        # Should NOT have branch choice actions (1000-1009 range)
        branch_actions = [a for a in legal if 1000 <= a <= 1009]
        assert len(branch_actions) == 0, (
            f"Should NOT use branch choice, found branch actions: {branch_actions}"
        )


@pytest.mark.behavioral
class TestEX3072MegiddoFlameSecurityEffect:
    """Tests for the [Security] play Guilmon effect."""

    def _trigger_security_skill(self, runner):
        """Directly invoke the SecuritySkill process callback."""
        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()
        player = runner.game.player1
        cs = db.create_card_source("EX3-072", player)
        effects = cs.effect_list(None)
        security_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(security_effects) == 1
        security_effects[0].on_process_callback({
            'player': player,
            'game': runner.game,
        })

    def test_security_plays_guilmon_from_trash(self, debug_runner):
        """Should allow playing a Guilmon from trash for free."""
        from digimon_gym.engine.game.constants import SEL_TRASH_START
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        # Put a Guilmon in P1's trash
        runner.inject_card(1, "BT2-009", zone="trash")

        self._trigger_security_skill(runner)

        # Should be in selection phase to pick from trash
        assert game.current_phase == GamePhase.SelectTarget, (
            f"Should be in SelectTarget for trash Guilmon, got {game.current_phase}"
        )

        # Find the Guilmon in trash and select it
        guilmon_idx = None
        for i, c in enumerate(game.player1.trash_cards):
            names = getattr(c, 'card_names', [])
            if any('Guilmon' in n for n in names):
                guilmon_idx = i
                break
        assert guilmon_idx is not None, "Guilmon should be in trash"
        trash_action = SEL_TRASH_START + guilmon_idx
        assert trash_action in runner.action_mask(), (
            f"Guilmon trash action {trash_action} should be legal. Legal: {runner.actions()}"
        )
        runner.execute(trash_action)
        runner.auto_resolve()  # resolve any follow-up phases

        snap = runner.snapshot()
        # Guilmon should be on field now
        p1_digimon = [f for f in snap.p1_field if f.card_id == "BT2-009"]
        assert len(p1_digimon) == 1, (
            f"Guilmon should be played to field from trash. Field: {snap.p1_field}"
        )

    def test_security_is_optional(self, debug_runner):
        """Security effect should be optional (can decline)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        runner.inject_card(1, "BT2-009", zone="trash")

        self._trigger_security_skill(runner)

        if game.current_phase == GamePhase.SelectTarget:
            # Should be able to decline
            assert 62 in runner.action_mask(), (
                "Security Guilmon play should be optional (decline available)"
            )

    def test_security_no_guilmon_in_trash_skips(self, debug_runner):
        """If no Guilmon in trash, security effect should do nothing."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        # No Guilmon in trash
        before_phase = game.current_phase

        self._trigger_security_skill(runner)

        # Should not enter selection if no valid targets
        # (either stays in same phase or returns to Main)
        snap = runner.snapshot()
        assert len(snap.p1_field) == 0, "Nothing should be played with no Guilmon in trash"
