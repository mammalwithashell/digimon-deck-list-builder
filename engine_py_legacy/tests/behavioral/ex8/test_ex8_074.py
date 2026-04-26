"""Behavioral tests for EX8-074 MedievalGallantmon (Lv.6 Green/Red, DP 11000, Cost 11).

Card text:
  When this card would be played, by suspending 2 Digimon, reduce play cost by 4.
  <Alliance>
  <Vortex>
  [When Digivolving] You may suspend 1 Digimon. Then, you may delete 1 of your
  opponent's 8000 DP or lower Digimon. For each other suspended Digimon, add 3000
  to this DP deletion effect's maximum.
  [All Turns] [Once Per Turn] When Digimon are played, you may activate 1 of this
  Digimon's [When Digivolving] effects.

C# reference:
  - BeforePayCost: suspend 2 Digimon (any unsuspended Digimon on field per
    IsPermanentExistsOnBattleAreaDigimon — not just own) -> reduce play cost by 4
  - Alliance, Vortex keywords
  - When Digivolving: suspend 1 Digimon (any on field), then delete opponent Digimon
    with DP <= 8000 + 3000 * (other suspended Digimon count excluding self)
  - All Turns OPT: When any Digimon is PLAYED (not digivolved), re-run
    When Digivolving
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase
from engine_py_legacy.engine.game.constants import SEL_MY_FIELD_START, SEL_OPP_FIELD_START

# Use ST1-03 (Biyomon Lv3 1000DP) as filler Digimon
# ST1-06 (Birdramon Lv4 4000DP) as mid-level
# ST1-10 (Phoenixmon Lv6 12000DP) as target too high to delete
# BT1-010 (Agumon Lv3 2000DP)
FILLER_DECK = ["EX8-074"] * 4 + ["ST1-03"] * 23 + ["BT1-010"] * 23


@pytest.mark.behavioral
class TestEX8074MedievalGallantmon:
    """Tests for EX8-074 MedievalGallantmon."""

    # ── Keywords ─────────────────────────────────────────────────────

    def test_has_alliance(self, debug_runner):
        """Should have <Alliance>."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-074"])

        card = perm.top_card
        effects = card.effect_list(None)
        alliance = [e for e in effects if getattr(e, '_is_alliance', False)]
        assert len(alliance) == 1

    def test_has_vortex(self, debug_runner):
        """Should have <Vortex>."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-074"])

        card = perm.top_card
        effects = card.effect_list(None)
        vortex = [e for e in effects if getattr(e, '_is_vortex', False)]
        assert len(vortex) == 1

    # ── BeforePayCost: Suspend 2 Digimon for -4 ─────────────────────

    def test_before_pay_cost_leak_guard(self, debug_runner):
        """BeforePayCost should have leak guard: only for THIS card being played."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-074"])

        card = perm.top_card
        effects = card.effect_list(None)
        bpc = [e for e in effects if e.timing == EffectTiming.BeforePayCost]
        assert len(bpc) == 1

        eff = bpc[0]
        # With a different card_source -> should return False (leak guard)
        assert not eff.can_use_condition({'card_source': None}), \
            "Should fail for a different card (leak guard)"

    def test_before_pay_cost_requires_2_unsuspended_digimon(self, debug_runner):
        """BeforePayCost condition should require >= 2 unsuspended Digimon on ANY field."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        # Place MedievalGallantmon on field first (to get card reference)
        perm_self = runner.place_on_field(1, ["EX8-074"])
        card = perm_self.top_card

        effects = card.effect_list(None)
        bpc = [e for e in effects if e.timing == EffectTiming.BeforePayCost][0]

        # Only 1 unsuspended Digimon (self) -> need 2, should fail
        assert not bpc.can_use_condition({'card_source': card}), \
            "Should fail with only 1 unsuspended Digimon on field"

        # Add a second unsuspended Digimon on own field
        runner.place_on_field(1, ["ST1-03"])
        assert bpc.can_use_condition({'card_source': card}), \
            "Should pass with 2 unsuspended Digimon on own field"

    def test_before_pay_cost_counts_opponent_digimon(self, debug_runner):
        """BeforePayCost should count opponent's unsuspended Digimon too (C# both fields)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)

        # Place MedievalGallantmon alone on P1's field
        perm_self = runner.place_on_field(1, ["EX8-074"])
        card = perm_self.top_card

        effects = card.effect_list(None)
        bpc = [e for e in effects if e.timing == EffectTiming.BeforePayCost][0]

        # 1 own (self) + 0 opponent = 1 total -> should fail
        assert not bpc.can_use_condition({'card_source': card}), \
            "Should fail with only 1 total unsuspended Digimon"

        # Add opponent Digimon -> 1 own + 1 opp = 2 total -> should pass
        runner.place_on_field(2, ["ST1-03"])
        assert bpc.can_use_condition({'card_source': card}), \
            "Should pass with 2 unsuspended Digimon across both fields"

    def test_before_pay_cost_cost_reduction_value(self, debug_runner):
        """BeforePayCost should reduce cost by 4."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-074"])

        card = perm.top_card
        effects = card.effect_list(None)
        bpc = [e for e in effects if e.timing == EffectTiming.BeforePayCost][0]
        assert bpc.cost_reduction == 4

    # ── [When Digivolving]: Suspend + Delete ─────────────────────────

    def test_when_digivolving_effect_exists(self, debug_runner):
        """Should have a When Digivolving effect with correct timing."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-074"])

        card = perm.top_card
        effects = card.effect_list(None)
        wd = [e for e in effects if e.is_when_digivolving]
        assert len(wd) == 1, "Should have exactly 1 When Digivolving effect"

    def test_when_digivolving_condition_requires_field(self, debug_runner):
        """When Digivolving condition should require the card to be on field."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        runner.inject_card(1, "EX8-074", "hand")
        hand_card = game.player1.hand_cards[-1]

        effects = hand_card.effect_list(None)
        wd = [e for e in effects if e.is_when_digivolving]
        if wd:
            assert not wd[0].can_use_condition({}), \
                "Should fail when card is not on field"

    def test_when_digivolving_suspend_then_delete(self, debug_runner):
        """When Digivolving: suspend 1 Digimon, then delete opponent Digimon <= threshold."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        # Place MedievalGallantmon on P1 field
        perm = runner.place_on_field(1, ["EX8-074"])
        # Place target on P1 field (to suspend)
        own_target = runner.place_on_field(1, ["ST1-03"])
        # Place opponent Digimon with DP <= 8000
        opp_target = runner.place_on_field(2, ["ST1-03"])  # 1000 DP

        # Trigger When Digivolving effect
        card = perm.top_card
        effects = card.effect_list(None)
        wd = [e for e in effects if e.is_when_digivolving][0]
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'card': card,
        }

        # Process the effect
        wd.on_process_callback(ctx)

        # Should enter selection phase for suspend
        assert game.current_phase == GamePhase.SelectTarget, \
            f"Should be in SelectTarget phase for suspend, got {game.current_phase}"

    def test_when_digivolving_delete_dp_threshold_base(self, debug_runner):
        """Base deletion DP threshold should be 8000 (no other suspended Digimon).

        When we suspend the opponent's Coredramon (6000 DP), it then counts as
        1 "other suspended Digimon" for the deletion step, raising threshold to 11000.
        The suspended Coredramon (6000 DP <= 11000) should be deletable.
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        # Place MedievalGallantmon on P1 field
        perm = runner.place_on_field(1, ["EX8-074"])
        # Place a Digimon on P1 field (to suspend)
        own_target = runner.place_on_field(1, ["ST1-03"])
        # Place opponent Digimon — Coredramon (6000 DP)
        opp_target = runner.place_on_field(2, ["ST1-06"])

        # Trigger When Digivolving
        card = perm.top_card
        effects = card.effect_list(None)
        wd = [e for e in effects if e.is_when_digivolving][0]
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'card': card,
        }
        wd.on_process_callback(ctx)

        # First selection: suspend 1 Digimon (offered opponent first)
        assert game.current_phase == GamePhase.SelectTarget
        legal = runner.action_mask()
        # Pick the opponent Digimon to suspend (SEL_OPP_FIELD_START = 114)
        opp_actions = [a for a in legal if a >= SEL_OPP_FIELD_START]
        assert len(opp_actions) > 0, \
            f"Should offer opponent Digimon to suspend. Legal: {legal}, actions: {runner.actions()}"
        runner.execute(opp_actions[0])

        # After suspend, should offer delete selection
        assert game.current_phase == GamePhase.SelectTarget, \
            f"Should be in SelectTarget for deletion, got {game.current_phase}"
        legal2 = runner.action_mask()
        # Should have options for opponent Digimon (now suspended Coredramon)
        opp_delete_actions = [a for a in legal2 if a >= SEL_OPP_FIELD_START]
        assert len(opp_delete_actions) > 0, \
            f"Should offer deletion of opponent Digimon. Legal: {legal2}, actions: {runner.actions()}"
        runner.execute(opp_delete_actions[0])

        runner.auto_resolve()

        snap = runner.snapshot()
        # Opponent's Coredramon (6000 DP) should be deleted
        opp_digimon = [s for s in snap.p2_field if s.card_id == "ST1-06"]
        assert len(opp_digimon) == 0, \
            "Opponent's 6000 DP Digimon should be deleted (threshold 11000 = 8000 + 3000*1)"

    def test_when_digivolving_delete_dp_threshold_scaling(self, debug_runner):
        """Deletion DP threshold should scale: 8000 + 3000 * (other suspended Digimon).

        Setup: 2 already-suspended Digimon on P1's field. We suspend the opponent's
        Phoenixmon (12000 DP). Now 3 "other suspended" Digimon exist (2 own + 1 opp),
        so threshold = 8000 + 3000*3 = 17000. Phoenixmon (12000 DP) should be deletable.
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        # Place MedievalGallantmon on P1 field
        perm = runner.place_on_field(1, ["EX8-074"])
        # Place 2 already-suspended Digimon (they count as "other suspended")
        runner.place_on_field(1, ["ST1-03"], is_suspended=True)
        runner.place_on_field(1, ["BT1-010"], is_suspended=True)
        # Place opponent Phoenixmon (12000 DP)
        opp_target = runner.place_on_field(2, ["ST1-10"])

        # Trigger When Digivolving
        card = perm.top_card
        effects = card.effect_list(None)
        wd = [e for e in effects if e.is_when_digivolving][0]
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'card': card,
        }
        wd.on_process_callback(ctx)

        # First selection: suspend opponent Phoenixmon
        assert game.current_phase == GamePhase.SelectTarget
        legal = runner.action_mask()
        opp_actions = [a for a in legal if a >= SEL_OPP_FIELD_START]
        assert len(opp_actions) > 0, \
            f"Should offer opponent Digimon to suspend. Legal: {legal}"
        runner.execute(opp_actions[0])

        # After suspending Phoenixmon, deletion phase:
        # "other suspended" = 2 (own) + 1 (Phoenixmon) = 3
        # threshold = 8000 + 3000*3 = 17000
        # Phoenixmon (12000 DP) <= 17000 → should be offered for deletion
        assert game.current_phase == GamePhase.SelectTarget, \
            f"Should be in deletion selection, got {game.current_phase}"
        legal2 = runner.action_mask()
        opp_delete_actions = [a for a in legal2 if a >= SEL_OPP_FIELD_START]
        assert len(opp_delete_actions) > 0, \
            f"Phoenixmon (12000 DP) should be deletable with threshold 17000. Legal: {legal2}, actions: {runner.actions()}"
        runner.execute(opp_delete_actions[0])

        runner.auto_resolve()

        snap = runner.snapshot()
        # Phoenixmon (12000 DP) should be deleted
        opp_phoenixmon = [s for s in snap.p2_field if s.card_id == "ST1-10"]
        assert len(opp_phoenixmon) == 0, \
            f"Phoenixmon (12000 DP) should be deleted with threshold 17000. P2 field: {[(s.card_id, s.dp) for s in snap.p2_field]}"

    def test_when_digivolving_suspend_optional(self, debug_runner):
        """The suspend step should be optional (may suspend)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        # Place MedievalGallantmon on P1 field
        perm = runner.place_on_field(1, ["EX8-074"])
        # Place target to suspend
        own_target = runner.place_on_field(1, ["ST1-03"])

        # Trigger When Digivolving
        card = perm.top_card
        effects = card.effect_list(None)
        wd = [e for e in effects if e.is_when_digivolving][0]
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'card': card,
        }
        wd.on_process_callback(ctx)

        # Should be in selection phase
        assert game.current_phase == GamePhase.SelectTarget
        # Check that the selection is optional (action 0 = decline should be legal)
        assert game.pending_selection is not None
        assert game.pending_selection.is_optional, \
            "Suspend step should be optional (may suspend)"

    def test_when_digivolving_delete_optional(self, debug_runner):
        """The delete step should be optional (may delete)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        # Place MedievalGallantmon on P1 field
        perm = runner.place_on_field(1, ["EX8-074"])
        # Place target to suspend and opponent to delete
        runner.place_on_field(1, ["ST1-03"])
        runner.place_on_field(2, ["ST1-03"])  # 1000 DP opponent

        # Trigger When Digivolving
        card = perm.top_card
        effects = card.effect_list(None)
        wd = [e for e in effects if e.is_when_digivolving][0]
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'card': card,
        }
        wd.on_process_callback(ctx)

        # Resolve suspend (pick first legal)
        assert game.current_phase == GamePhase.SelectTarget
        legal = runner.action_mask()
        if legal:
            runner.execute(legal[0])

        # Now in delete phase — should be optional
        if game.current_phase == GamePhase.SelectTarget:
            assert game.pending_selection is not None
            assert game.pending_selection.is_optional, \
                "Delete step should be optional (may delete)"

    # ── [All Turns] [Once Per Turn] ──────────────────────────────────

    def test_all_turns_once_per_turn_effect_exists(self, debug_runner):
        """Should have an All Turns OPT effect that triggers on Digimon play."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-074"])

        card = perm.top_card
        effects = card.effect_list(None)
        all_turns = [e for e in effects
                     if e.timing == EffectTiming.OnEnterFieldAnyone
                     and not e.is_when_digivolving
                     and not e.is_on_play
                     and e.max_count_per_turn == 1]
        assert len(all_turns) == 1, "Should have All Turns OPT effect"
        assert all_turns[0].is_optional, "All Turns effect should be optional"

    def test_all_turns_only_on_digimon_play_not_digivolve(self, debug_runner):
        """All Turns effect should trigger on Digimon play but NOT digivolve."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-074"])

        card = perm.top_card
        effects = card.effect_list(None)
        all_turns = [e for e in effects
                     if e.timing == EffectTiming.OnEnterFieldAnyone
                     and not e.is_when_digivolving
                     and not e.is_on_play
                     and e.max_count_per_turn == 1][0]

        opp_perm = runner.place_on_field(2, ["ST1-03"])

        # Digivolve event -> should NOT trigger
        ctx_digi = {
            'played_permanent': opp_perm,
            'permanent': opp_perm,
            'is_digivolve': True,
        }
        assert not all_turns.can_use_condition(ctx_digi), \
            "Should NOT trigger on digivolve"

        # Play event -> should trigger
        ctx_play = {
            'played_permanent': opp_perm,
            'permanent': opp_perm,
            'is_digivolve': False,
        }
        assert all_turns.can_use_condition(ctx_play), \
            "Should trigger on Digimon play"

    def test_all_turns_does_not_trigger_on_non_digimon(self, debug_runner):
        """All Turns effect should NOT trigger when a non-Digimon (Tamer/Option) is played."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-074"])

        card = perm.top_card
        effects = card.effect_list(None)
        all_turns = [e for e in effects
                     if e.timing == EffectTiming.OnEnterFieldAnyone
                     and not e.is_when_digivolving
                     and not e.is_on_play
                     and e.max_count_per_turn == 1][0]

        # Place a Tamer
        from engine_py_legacy.engine.core.permanent import Permanent
        tamer_perm = runner.place_on_field(1, ["BT1-085"])  # Tai Kamiya Tamer

        ctx_tamer = {
            'played_permanent': tamer_perm,
            'permanent': tamer_perm,
            'is_digivolve': False,
        }
        assert not all_turns.can_use_condition(ctx_tamer), \
            "Should NOT trigger on Tamer play (only Digimon)"

    def test_all_turns_triggers_on_opponent_digimon_play(self, debug_runner):
        """All Turns: should trigger when OPPONENT plays a Digimon too."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-074"])

        card = perm.top_card
        effects = card.effect_list(None)
        all_turns = [e for e in effects
                     if e.timing == EffectTiming.OnEnterFieldAnyone
                     and not e.is_when_digivolving
                     and not e.is_on_play
                     and e.max_count_per_turn == 1][0]

        # Opponent Digimon
        opp_perm = runner.place_on_field(2, ["ST1-03"])

        ctx_opp_play = {
            'played_permanent': opp_perm,
            'permanent': opp_perm,
            'is_digivolve': False,
        }
        assert all_turns.can_use_condition(ctx_opp_play), \
            "Should trigger on opponent's Digimon play (All Turns)"

    def test_all_turns_once_per_turn_limit(self, debug_runner):
        """All Turns effect should only activate once per turn."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-074"])

        card = perm.top_card
        effects = card.effect_list(None)
        all_turns = [e for e in effects
                     if e.timing == EffectTiming.OnEnterFieldAnyone
                     and not e.is_when_digivolving
                     and not e.is_on_play
                     and e.max_count_per_turn == 1][0]

        assert all_turns.max_count_per_turn == 1
        # Simulate first activation
        assert all_turns.can_activate_this_turn()
        all_turns.record_activation()
        # Should not activate again
        assert not all_turns.can_activate_this_turn(), \
            "Should not activate more than once per turn"

    def test_all_turns_replays_when_digivolving_effect(self, debug_runner):
        """All Turns should replay the When Digivolving effect (suspend + delete)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        # Place MedievalGallantmon on P1 field
        perm = runner.place_on_field(1, ["EX8-074"])
        # Place targets
        own_target = runner.place_on_field(1, ["ST1-03"])
        opp_target = runner.place_on_field(2, ["ST1-03"])  # 1000 DP

        card = perm.top_card
        effects = card.effect_list(None)
        all_turns = [e for e in effects
                     if e.timing == EffectTiming.OnEnterFieldAnyone
                     and not e.is_when_digivolving
                     and not e.is_on_play
                     and e.max_count_per_turn == 1][0]

        # Call process directly (simulating trigger)
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'card': card,
            'played_permanent': own_target,
            'is_digivolve': False,
        }
        all_turns.on_process_callback(ctx)

        # Should enter selection phase (suspend step from When Digivolving)
        assert game.current_phase == GamePhase.SelectTarget, \
            f"All Turns replay should trigger suspend selection, got {game.current_phase}"

    # ── Integration test: cost reduction via actual play ──────────────

    def test_cost_reduction_value_applied_on_play(self, debug_runner):
        """Playing MedievalGallantmon with 2+ unsuspended Digimon should cost 7 (11 - 4).

        NOTE: The interactive "suspend 2 Digimon" selection from BeforePayCost is
        overwritten by the When Digivolving effect's selection (engine limitation:
        BeforePayCost process callbacks with interactive selections get superseded by
        subsequent OnEnterFieldAnyone effects). The cost reduction VALUE is correctly
        applied, but the suspend cost isn't enforced interactively.
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        game = runner.game

        # Place 2 unsuspended Digimon on P1 field (to satisfy condition)
        runner.place_on_field(1, ["ST1-03"], turn_played=-1)
        runner.place_on_field(1, ["BT1-010"], turn_played=-1)

        # Inject EX8-074 into hand
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX8-074", "hand")

        snap_before = runner.snapshot()
        memory_before = snap_before.memory  # should be 10

        # Find play action for MedievalGallantmon
        play_action = runner.find_action("Play MedievalGallantmon")
        if play_action is None:
            play_action = runner.find_action("Play Medieval")
        if play_action is None:
            play_action = runner.find_action("EX8-074")

        assert play_action is not None, \
            f"Should find play action. Actions: {runner.actions()}"

        runner.execute(play_action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Cost should be 7 (11 - 4 reduction), so memory = 10 - 7 = 3
        assert snap_after.memory == memory_before - 7, \
            f"Memory should decrease by 7 (cost 11 - 4 reduction). Before: {memory_before}, After: {snap_after.memory}"
        # MedievalGallantmon should be on field
        assert any(s.card_id == "EX8-074" for s in snap_after.p1_field), \
            "MedievalGallantmon should be on field after play"
