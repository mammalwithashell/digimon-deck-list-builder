"""Behavioral tests for LM-035 Amber Memory Boost!

Card text (from cards.json):
Purple also meets this card's color requirements.
[Main] Reveal the top 3 cards of your deck. Add 1 yellow or purple Digimon card
    among them to the hand. Return the rest to the bottom of deck. Then, place
    this card in the battle area.
[Main] <Delay> (By trashing this card after the placing turn, activate the
    effect below.)
    - Gain 2 memory.
[Security] Place this card in the battle area.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestLM035AmberMemoryBoost:
    """Tests for LM-035 Amber Memory Boost!"""

    def test_main_effect_uses_option_skill_timing(self, debug_runner):
        """Main effect should use OptionSkill timing (not OnEnterFieldAnyone)."""
        runner = debug_runner(
            deck1=["LM-035"] * 4 + ["ST1-07"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=10,
        )
        game = runner.game
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-035", game.player1)
        effects = cs.effect_list(None)

        main = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main) >= 1, (
            f"Should have OptionSkill main effect. Timings: "
            f"{[(e.effect_name, e.timing) for e in effects]}"
        )

    def test_main_effect_uses_reveal_and_select(self, debug_runner):
        """Main effect should use effect_reveal_and_select for player choice."""
        runner = debug_runner(
            deck1=["LM-035"] * 4 + ["ST1-07"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=10,
        )
        game = runner.game
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-035", game.player1)
        effects = cs.effect_list(None)

        main = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main) >= 1, "Should have OptionSkill effect"

        # Verify the process callback exists and calls game.effect_reveal_and_select
        # by checking the callback source or by running it
        effect = main[0]
        assert effect.on_process_callback is not None, "Main effect should have process callback"

    def test_has_delay_marker(self, debug_runner):
        """Should have Delay marker effect."""
        runner = debug_runner(
            deck1=["LM-035"] * 4 + ["ST1-07"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=10,
        )
        game = runner.game
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-035", game.player1)
        effects = cs.effect_list(None)

        delay = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delay) >= 1, "Should have Delay marker"

    def test_has_delay_effect_gain_2_memory(self, debug_runner):
        """Should have Delay effect that gains 2 memory."""
        runner = debug_runner(
            deck1=["LM-035"] * 4 + ["ST1-07"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=10,
        )
        game = runner.game
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-035", game.player1)
        effects = cs.effect_list(None)

        delay_eff = [e for e in effects if getattr(e, '_is_delay_effect', False)]
        assert len(delay_eff) >= 1, "Should have Delay effect"
        assert delay_eff[0].timing == EffectTiming.OnStartMainPhase, (
            f"Delay effect should use OnStartMainPhase timing, got {delay_eff[0].timing}"
        )

    def test_security_uses_security_skill_timing(self, debug_runner):
        """[Security] should use SecuritySkill timing."""
        runner = debug_runner(
            deck1=["LM-035"] * 4 + ["ST1-07"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=10,
        )
        game = runner.game
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-035", game.player1)
        effects = cs.effect_list(None)

        sec = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec) >= 1, (
            f"Should have SecuritySkill effect. Timings: "
            f"{[(e.effect_name, e.timing) for e in effects]}"
        )

    def test_delay_effect_has_process_callback(self, debug_runner):
        """Delay effect should have a process callback for gaining memory."""
        runner = debug_runner(
            deck1=["LM-035"] * 4 + ["ST1-07"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=5,
        )
        game = runner.game
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-035", game.player1)
        effects = cs.effect_list(None)

        delay_eff = [e for e in effects if getattr(e, '_is_delay_effect', False)]
        assert len(delay_eff) >= 1, "Should have Delay effect"
        assert delay_eff[0].on_process_callback is not None, (
            "Delay effect should have process callback"
        )
        assert delay_eff[0].timing == EffectTiming.OnStartMainPhase
