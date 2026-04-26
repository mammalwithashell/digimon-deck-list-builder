"""Behavioral tests for BT15-101 MetalGarurumon (Lv.6 Blue, Cost 12).

Card text (per cards.json / C# source of truth):
Alt digivolve: Lv.5 w/[Garurumon] in name: Cost 4
While you have a Tamer with [Matt Ishida] in its name and your opponent has a
    10000 DP or higher Digimon, one of your [Gabumon] may digivolve into this
    card in your hand for a digivolution cost of 4, ignoring its digivolution
    requirements.
Evade.
[When Digivolving] 3 of your opponent's Digimon and Tamers can't suspend until
    the end of their turn.
[All Turns] [Once Per Turn] When this Digimon becomes suspended, you may
    unsuspend it.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase


@pytest.mark.behavioral
class TestBT15101MetalGarurumon:
    """Tests for BT15-101 MetalGarurumon."""

    # ---- Alt digi from Lv.5 Garurumon for 4 ----

    def test_alt_digi_from_garurumon_lv5(self, debug_runner):
        """Standard alt-digi: can digivolve from Lv.5 with [Garurumon] in name for cost 4."""
        runner = debug_runner(initial_memory=5)

        # Inject BT15-101 into hand
        runner.inject_card(1, "BT15-101", "hand")
        bt15_card = runner.game.player1.hand_cards[-1]

        effects = bt15_card.effect_list(None)
        alt_digi_effects = [e for e in effects
                            if getattr(e, '_alt_digi_cost', None) is not None
                            and getattr(e, '_alt_digi_name', None) == 'Garurumon']
        assert len(alt_digi_effects) >= 1, (
            "Should have alt-digi effect for Garurumon"
        )
        assert alt_digi_effects[0]._alt_digi_cost == 4
        assert alt_digi_effects[0]._alt_digi_level == 5

    # ---- Evade keyword ----

    def test_has_evade(self, debug_runner):
        """BT15-101 should have the Evade keyword."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-101"])

        effects = perm.top_card.effect_list(None)
        evade_effects = [e for e in effects if getattr(e, '_is_evade', False)]
        assert len(evade_effects) >= 1, "BT15-101 should have Evade keyword"

    # ---- Warp digivolve from Gabumon ----

    def test_warp_digi_condition_needs_matt_ishida(self, debug_runner):
        """Warp digivolve condition requires a Matt Ishida tamer on field."""
        runner = debug_runner(initial_memory=10)

        runner.inject_card(1, "BT15-101", "hand")
        bt15_card = runner.game.player1.hand_cards[-1]

        # Place Gabumon on field
        runner.place_on_field(1, ["BT15-020"])
        # Place opponent high-DP Digimon
        runner.place_on_field(2, ["ST1-10"], is_suspended=False)  # Phoenixmon Lv.6 12000 DP

        effects = bt15_card.effect_list(None)
        hand_main_effects = [e for e in effects if getattr(e, '_is_hand_main', False)]
        assert len(hand_main_effects) == 1, "Should have 1 hand-main warp digivolve effect"

        warp_eff = hand_main_effects[0]
        ctx = {'game': runner.game, 'player': runner.game.player1}
        # Without Matt Ishida, condition should fail
        assert not warp_eff.can_use_condition(ctx), (
            "Warp digivolve should require Matt Ishida tamer"
        )

    def test_warp_digi_condition_needs_high_dp_opponent(self, debug_runner):
        """Warp digivolve condition requires opponent to have 10000+ DP Digimon."""
        runner = debug_runner(initial_memory=10)

        runner.inject_card(1, "BT15-101", "hand")
        bt15_card = runner.game.player1.hand_cards[-1]

        # Place Gabumon + Matt Ishida on field
        runner.place_on_field(1, ["BT15-020"])
        runner.place_on_field(1, ["BT15-083"])  # Matt Ishida

        effects = bt15_card.effect_list(None)
        warp_eff = [e for e in effects if getattr(e, '_is_hand_main', False)][0]
        ctx = {'game': runner.game, 'player': runner.game.player1}

        # Without high-DP opponent, condition should fail
        assert not warp_eff.can_use_condition(ctx), (
            "Warp digivolve should require opponent 10000+ DP Digimon"
        )

    def test_warp_digi_condition_needs_gabumon(self, debug_runner):
        """Warp digivolve condition requires a [Gabumon] on your field."""
        runner = debug_runner(initial_memory=10)

        runner.inject_card(1, "BT15-101", "hand")
        bt15_card = runner.game.player1.hand_cards[-1]

        # Place Matt Ishida but NO Gabumon
        runner.place_on_field(1, ["BT15-086"])
        # Place a non-Gabumon Digimon
        runner.place_on_field(1, ["ST2-08"])
        # Opponent has high DP
        runner.place_on_field(2, ["ST1-10"])

        effects = bt15_card.effect_list(None)
        warp_eff = [e for e in effects if getattr(e, '_is_hand_main', False)][0]
        ctx = {'game': runner.game, 'player': runner.game.player1}

        assert not warp_eff.can_use_condition(ctx), (
            "Warp digivolve should require Gabumon on field"
        )

    def test_warp_digi_all_conditions_met(self, debug_runner):
        """With Matt Ishida, Gabumon on field, and 10000+ DP opponent, warp should be available."""
        runner = debug_runner(initial_memory=10)

        runner.inject_card(1, "BT15-101", "hand")
        bt15_card = runner.game.player1.hand_cards[-1]

        runner.place_on_field(1, ["BT15-020"])   # Gabumon
        runner.place_on_field(1, ["BT15-083"])   # Matt Ishida
        runner.place_on_field(2, ["ST1-10"])     # Opponent high DP

        effects = bt15_card.effect_list(None)
        warp_eff = [e for e in effects if getattr(e, '_is_hand_main', False)][0]
        ctx = {'game': runner.game, 'player': runner.game.player1}

        assert warp_eff.can_use_condition(ctx), (
            "Warp digivolve should be available with all conditions met"
        )

    # ---- [When Digivolving] can't suspend ----

    def test_when_digivolving_is_mandatory(self, debug_runner):
        """The when-digivolving effect is mandatory per C# (canNoSelect=false)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-101"])

        effects = perm.top_card.effect_list(None)
        digi_effects = [e for e in effects if e.timing == EffectTiming.OnEnterFieldAnyone
                        and getattr(e, 'is_when_digivolving', False)]
        assert len(digi_effects) == 1
        assert not digi_effects[0].is_optional, (
            "When-digivolving effect should be mandatory"
        )

    # ---- [All Turns][OPT] unsuspend self ----

    def test_unsuspend_self_on_suspend(self, debug_runner):
        """When this Digimon becomes suspended, it may unsuspend itself."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-101"])

        effects = perm.top_card.effect_list(None)
        unsuspend_effects = [e for e in effects if e.timing == EffectTiming.OnTappedAnyone]
        assert len(unsuspend_effects) == 1

        unsuspend_eff = unsuspend_effects[0]
        assert unsuspend_eff.is_optional, "Unsuspend effect should be optional"
        assert unsuspend_eff.max_count_per_turn == 1, "Should be Once Per Turn"

    def test_unsuspend_self_only_for_this_digimon(self, debug_runner):
        """The unsuspend trigger should only fire when THIS Digimon is suspended."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-101"])
        other_perm = runner.place_on_field(1, ["ST2-08"])

        effects = perm.top_card.effect_list(None)
        unsuspend_eff = [e for e in effects if e.timing == EffectTiming.OnTappedAnyone][0]

        # Should trigger when this perm is the event_permanent
        assert unsuspend_eff.can_use_condition({'event_permanent': perm}), (
            "Should trigger when this Digimon is suspended"
        )

        # Should NOT trigger when a different perm is the event_permanent
        assert not unsuspend_eff.can_use_condition({'event_permanent': other_perm}), (
            "Should NOT trigger when another Digimon is suspended"
        )

    def test_unsuspend_process_actually_unsuspends(self, debug_runner):
        """After processing the unsuspend effect, the Digimon should be unsuspended."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-101"], is_suspended=True)

        effects = perm.top_card.effect_list(None)
        unsuspend_eff = [e for e in effects if e.timing == EffectTiming.OnTappedAnyone][0]

        unsuspend_eff.on_process_callback({
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': perm,
        })

        assert not perm.is_suspended, "Digimon should be unsuspended after effect"
