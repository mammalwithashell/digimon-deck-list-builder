"""Behavioral tests for BT21-081 Owen Dreadnought (Tamer).

Card text:
[Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
[End of Your Turn] By suspending this Tamer, 1 of your Digimon with the
[Reptile] or [Dragonkin] trait gains <Piercing> for the turn. Then, that
Digimon attacks.
Security Effect [Security] Play this card without paying the cost.
"""

import pytest

# ST1-03 Agumon: Lv.3, Reptile trait, DP 2000
# ST1-06 Coredramon: Lv.4, Dragon trait (NOT Reptile/Dragonkin), DP 6000
# BT1-010 Agumon: Lv.3, Reptile trait
OWEN_DECK = ["BT21-081"] * 4 + ["ST1-03"] * 46 + ["BT1-001"] * 4


@pytest.mark.behavioral
class TestBT21081OwenDreadnought:
    """Tests for BT21-081 Owen Dreadnought."""

    # -- Effect 0: [Start of Your Main Phase] memory gain --

    def test_start_main_phase_gain_memory_opponent_has_digimon(self, debug_runner):
        """If opponent has a Digimon, gain 1 memory at start of main phase."""
        runner = debug_runner(deck1=OWEN_DECK, deck2=OWEN_DECK, initial_memory=3)
        runner.place_on_field(1, ["BT21-081"])
        # Opponent has a Digimon
        runner.place_on_field(2, ["ST1-03"])
        runner.set_phase("Main")

        from digimon_gym.engine.data.enums import EffectTiming
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        snap = runner.snapshot()
        assert snap.memory == 4, f"Expected memory 4 (3+1), got {snap.memory}"

    def test_start_main_phase_no_gain_when_opponent_has_no_digimon(self, debug_runner):
        """If opponent has no Digimon, do NOT gain memory."""
        runner = debug_runner(deck1=OWEN_DECK, deck2=OWEN_DECK, initial_memory=3)
        runner.place_on_field(1, ["BT21-081"])
        # Opponent has NO Digimon (only tamer)
        runner.place_on_field(2, ["BT21-081"])  # Tamer, not Digimon
        runner.set_phase("Main")

        from digimon_gym.engine.data.enums import EffectTiming
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        snap = runner.snapshot()
        assert snap.memory == 3, f"Expected memory 3 (unchanged), got {snap.memory}"

    def test_start_main_phase_no_gain_on_opponent_turn(self, debug_runner):
        """Memory gain should not fire on opponent's turn."""
        runner = debug_runner(deck1=OWEN_DECK, deck2=OWEN_DECK,
                              initial_memory=3, first_player=2)
        runner.place_on_field(1, ["BT21-081"])
        runner.place_on_field(2, ["ST1-03"])
        runner.set_phase("Main")

        from digimon_gym.engine.data.enums import EffectTiming
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        snap = runner.snapshot()
        # Memory is from P2's perspective; P1's tamer shouldn't fire
        assert snap.memory == 3, f"Expected memory 3 (unchanged), got {snap.memory}"

    # -- Effect 1: [End of Your Turn] Suspend + Piercing + Attack --

    def test_eot_condition_blocks_when_already_suspended(self, debug_runner):
        """If tamer is already suspended, the EOT effect should not be available."""
        runner = debug_runner(deck1=OWEN_DECK, deck2=OWEN_DECK, initial_memory=-3)
        # Place Owen already suspended
        runner.place_on_field(1, ["BT21-081"], is_suspended=True)
        runner.place_on_field(1, ["ST1-03"])  # Reptile Digimon
        runner.set_phase("Main")

        game = runner.game
        player = game.player1
        owen_card = None
        for p in player.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT21-081":
                owen_card = p.top_card
                break
        assert owen_card is not None

        effects = owen_card.effect_list(None)
        from digimon_gym.engine.data.enums import EffectTiming
        eot_effect = None
        for eff in effects:
            if eff.timing == EffectTiming.OnEndTurn:
                eot_effect = eff
                break
        assert eot_effect is not None, "Should have End of Turn effect"
        result = eot_effect.can_use_condition({})
        assert result is False, (
            "End of Turn condition should be False when tamer is already suspended"
        )

    def test_eot_condition_passes_when_unsuspended(self, debug_runner):
        """If tamer is unsuspended and it is our turn, the EOT effect should be available."""
        runner = debug_runner(deck1=OWEN_DECK, deck2=OWEN_DECK, initial_memory=-3)
        runner.place_on_field(1, ["BT21-081"])  # Unsuspended
        runner.place_on_field(1, ["ST1-03"])  # Reptile Digimon
        runner.set_phase("Main")

        game = runner.game
        player = game.player1
        owen_card = None
        for p in player.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT21-081":
                owen_card = p.top_card
                break
        assert owen_card is not None

        effects = owen_card.effect_list(None)
        from digimon_gym.engine.data.enums import EffectTiming
        eot_effect = None
        for eff in effects:
            if eff.timing == EffectTiming.OnEndTurn:
                eot_effect = eff
                break
        assert eot_effect is not None
        result = eot_effect.can_use_condition({})
        assert result is True, (
            "End of Turn condition should be True when tamer is unsuspended on our turn"
        )

    def test_eot_suspends_tamer_and_creates_selection(self, debug_runner):
        """EOT effect should suspend tamer and offer Reptile/Dragonkin target selection."""
        runner = debug_runner(deck1=OWEN_DECK, deck2=OWEN_DECK, initial_memory=-3)
        runner.place_on_field(1, ["BT21-081"])
        runner.place_on_field(1, ["ST1-03"])  # Reptile Digimon

        runner.set_phase("End")
        runner.game.phase_end()

        # Tamer should be suspended
        snap = runner.snapshot()
        tamer = next((s for s in snap.p1_field if s.card_id == "BT21-081"), None)
        assert tamer is not None
        assert tamer.is_suspended, (
            f"Tamer should be suspended after EOT effect. "
            f"Phase: {snap.phase}, actions: {runner.actions()}"
        )

        # Should have a pending selection for the Reptile/Dragonkin target
        ps = runner.game.pending_selection
        assert ps is not None, "Should have pending selection for Reptile/Dragonkin Digimon"

    def test_eot_selection_mandatory_not_optional(self, debug_runner):
        """Target selection should be mandatory (not optional) per C# canNoSelect:false."""
        runner = debug_runner(deck1=OWEN_DECK, deck2=OWEN_DECK, initial_memory=-3)
        runner.place_on_field(1, ["BT21-081"])
        runner.place_on_field(1, ["ST1-03"])  # Reptile Digimon

        runner.set_phase("End")
        runner.game.phase_end()

        ps = runner.game.pending_selection
        assert ps is not None, "Should have pending selection"
        # Check that Pass (action 62) is NOT in the legal actions — selection is mandatory
        legal = runner.action_mask()
        assert 62 not in legal, (
            f"Selection should be mandatory (no pass/decline). Legal actions: {legal}"
        )

    def test_eot_grants_piercing_to_selected_digimon(self, debug_runner):
        """Selected Reptile/Dragonkin Digimon should gain Piercing for the turn."""
        runner = debug_runner(deck1=OWEN_DECK, deck2=OWEN_DECK, initial_memory=-3)
        runner.place_on_field(1, ["BT21-081"])
        reptile_perm = runner.place_on_field(1, ["ST1-03"])

        runner.set_phase("End")
        runner.game.phase_end()

        # Select the Reptile Digimon
        ps = runner.game.pending_selection
        assert ps is not None
        legal = runner.action_mask()
        non_pass = [a for a in legal if a != 62]
        assert non_pass, f"Should have non-pass actions. Legal: {legal}"
        runner.execute(non_pass[0])

        # The Reptile Digimon should now have Piercing
        assert reptile_perm.has_keyword('_is_piercing'), (
            "Selected Digimon should have Piercing keyword"
        )

    def test_eot_piercing_not_on_tamer(self, debug_runner):
        """Piercing should NOT be granted to the tamer itself (bug: _is_piercing on effect)."""
        runner = debug_runner(deck1=OWEN_DECK, deck2=OWEN_DECK, initial_memory=-3)
        runner.place_on_field(1, ["BT21-081"])
        runner.place_on_field(1, ["ST1-03"])

        # Check the tamer permanent does NOT have Piercing
        tamer_perm = None
        for p in runner.game.player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT21-081":
                tamer_perm = p
                break
        assert tamer_perm is not None
        assert not tamer_perm.has_keyword('_is_piercing'), (
            "Tamer should NOT have Piercing — only the selected Digimon gets it"
        )

    def test_eot_only_reptile_dragonkin_selectable(self, debug_runner):
        """Only Digimon with Reptile or Dragonkin trait should be selectable."""
        runner = debug_runner(deck1=OWEN_DECK, deck2=OWEN_DECK, initial_memory=-3)
        runner.place_on_field(1, ["BT21-081"])
        # ST1-06 Coredramon has "Dragon" trait (NOT Reptile or Dragonkin)
        runner.place_on_field(1, ["ST1-06"])

        runner.set_phase("End")
        runner.game.phase_end()

        # Tamer should be suspended (cost was paid)
        snap = runner.snapshot()
        tamer = next((s for s in snap.p1_field if s.card_id == "BT21-081"), None)
        assert tamer is not None
        assert tamer.is_suspended, "Tamer should be suspended"

        # No valid target => no pending selection (filter finds no match)
        # The selection should not appear since no eligible Digimon
        ps = runner.game.pending_selection
        # If there's a pending selection, it should have no valid actions
        if ps is not None:
            legal = runner.action_mask()
            non_pass = [a for a in legal if a != 62]
            assert not non_pass, (
                f"Dragon-trait Digimon should NOT be selectable. Non-pass actions: {non_pass}"
            )

    def test_eot_forced_attack_creates_eot_action_phase(self, debug_runner):
        """After granting Piercing, the Digimon should be forced to attack (EndOfTurnAction)."""
        runner = debug_runner(deck1=OWEN_DECK, deck2=OWEN_DECK, initial_memory=-3)
        runner.place_on_field(1, ["BT21-081"])
        reptile_perm = runner.place_on_field(1, ["ST1-03"])

        runner.set_phase("End")
        runner.game.phase_end()

        # Select the Reptile Digimon
        ps = runner.game.pending_selection
        assert ps is not None
        legal = runner.action_mask()
        non_pass = [a for a in legal if a != 62]
        assert non_pass, f"Should have non-pass actions. Legal: {legal}"
        runner.execute(non_pass[0])

        # After selection resolves, should be in EndOfTurnAction phase
        snap = runner.snapshot()
        assert snap.phase == "EndOfTurnAction", (
            f"Expected EndOfTurnAction phase for forced attack, got {snap.phase}. "
            f"Actions: {runner.actions()}"
        )

    def test_eot_forced_attack_must_attack(self, debug_runner):
        """The forced attack should not allow passing — Digimon MUST attack."""
        runner = debug_runner(deck1=OWEN_DECK, deck2=OWEN_DECK, initial_memory=-3)
        runner.place_on_field(1, ["BT21-081"])
        runner.place_on_field(1, ["ST1-03"])
        # Place an opponent Digimon so there's a target
        runner.place_on_field(2, ["ST1-03"])

        runner.set_phase("End")
        runner.game.phase_end()

        # Select the Reptile Digimon
        ps = runner.game.pending_selection
        assert ps is not None
        legal = runner.action_mask()
        non_pass = [a for a in legal if a != 62]
        runner.execute(non_pass[0])

        # Now in EndOfTurnAction — check that attack actions exist
        snap = runner.snapshot()
        assert snap.phase == "EndOfTurnAction"
        legal = runner.action_mask()
        attack_actions = [a for a in legal if 100 <= a < 300]
        assert attack_actions, (
            f"Should have attack actions in EndOfTurnAction. Legal: {runner.actions()}"
        )
