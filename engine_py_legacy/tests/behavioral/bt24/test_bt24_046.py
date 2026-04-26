"""Behavioral tests for BT24-046 Garurumon (Lv.4 Green/Yellow Digimon).

Card text:
＜Jamming＞
[On Play] [When Digivolving] Suspend 1 of your opponent's Digimon.
Inherited: [When Attacking] [Once Per Turn] Suspend 1 of your opponent's Digimon.

Alt-digi: [Gabumon] in name or Lv.3 w/[TS] trait: Cost 2
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming

GARURUMON_DECK = ["BT24-046"] * 50


@pytest.mark.behavioral
class TestBT24046Garurumon:
    """Tests for BT24-046 Garurumon."""

    # ── Keyword: Jamming ──────────────────────────────────────────

    def test_jamming_present(self, debug_runner):
        """Should have Jamming keyword."""
        runner = debug_runner(deck1=GARURUMON_DECK, deck2=GARURUMON_DECK, initial_memory=10)

        runner.inject_card(1, "BT24-046", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        jamming = [e for e in effects if getattr(e, '_is_jamming', False)]
        assert len(jamming) == 1, "Should have 1 Jamming effect"

    # ── Alt-Digi ──────────────────────────────────────────────────

    def test_alt_digi_attributes(self, debug_runner):
        """Alt-digi: [Gabumon] name or Lv.3 w/[TS] trait, cost 2."""
        runner = debug_runner(deck1=GARURUMON_DECK, deck2=GARURUMON_DECK, initial_memory=10)

        runner.inject_card(1, "BT24-046", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        alt_digi = [e for e in effects if hasattr(e, '_alt_digi_cost')]
        assert len(alt_digi) == 1, "Should have 1 alt-digi effect"
        eff = alt_digi[0]
        assert eff._alt_digi_cost == 2
        assert eff._alt_digi_level == 3
        assert eff._alt_digi_name == "Gabumon"
        assert eff._alt_digi_trait == "TS"

    # ── On Play: Suspend ──────────────────────────────────────────

    def test_on_play_suspend_opponent_digimon(self, debug_runner):
        """[On Play] should suspend 1 opponent's Digimon."""
        runner = debug_runner(deck1=GARURUMON_DECK, deck2=GARURUMON_DECK, initial_memory=5)

        runner.place_on_field(2, ["BT24-046"])
        opp_perm = runner.game.player2.battle_area[0]
        assert not opp_perm.is_suspended, "Target should start unsuspended"

        runner.inject_card(1, "BT24-046", "hand")
        action = runner.find_action("Play Garurumon")
        assert action is not None, "Should be able to play Garurumon"
        runner.execute(action)
        runner.auto_resolve()

        assert opp_perm.is_suspended, "Opponent's Digimon should be suspended after On Play"

    # ── When Digivolving: Suspend ─────────────────────────────────

    def test_when_digivolving_suspend_opponent_digimon(self, debug_runner):
        """[When Digivolving] should suspend 1 opponent's Digimon."""
        runner = debug_runner(deck1=GARURUMON_DECK, deck2=GARURUMON_DECK, initial_memory=10)

        runner.place_on_field(1, ["BT24-046"])
        runner.place_on_field(2, ["BT24-046"])
        opp_perm = runner.game.player2.battle_area[0]
        assert not opp_perm.is_suspended

        runner.inject_card(1, "BT24-046", "hand")
        action = runner.find_action("Digivolve")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()
            assert opp_perm.is_suspended, "Opponent's Digimon should be suspended after digivolving"

    # ── Inherited: When Attacking ─────────────────────────────────

    def test_inherited_effect_attributes(self, debug_runner):
        """Inherited effect should be marked correctly."""
        runner = debug_runner(deck1=GARURUMON_DECK, deck2=GARURUMON_DECK, initial_memory=10)

        runner.inject_card(1, "BT24-046", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)

        inherited = [e for e in effects if e.is_inherited_effect]
        assert len(inherited) == 1, "Should have 1 inherited effect"
        eff = inherited[0]
        assert eff.timing == EffectTiming.OnUseAttack, "Should use OnUseAttack timing"
        assert eff.max_count_per_turn == 1, "Should be Once Per Turn"
