"""Behavioral tests for ST19-14 Arisa Kinosaki (Tamer, Yellow).

Card text:
[Start of Your Turn] If you have 2 or less memory, set it to 3.

[Your Turn] When effects play one of your Tokens or a Digimon with the
[Puppet] trait, by suspending this Tamer, 1 of those Digimon gains <Rush>
for the turn. (This Digimon can attack the turn it comes into play.)

Security Effect:
[Security] Play this card without paying the cost.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestST19_14Effect0_StartOfTurnMemory:
    """[Start of Your Turn] If you have 2 or less memory, set it to 3."""

    def _get_sot_effect(self, perm):
        """Find the OnStartTurn memory effect from ST19-14."""
        for cs in perm.card_sources:
            if cs.c_entity_base and cs.c_entity_base.card_id == "ST19-14":
                effects = cs.effect_list(None)
                sot_effs = [
                    e for e in effects if e.timing == EffectTiming.OnStartTurn
                ]
                assert len(sot_effs) == 1, (
                    "ST19-14 should have exactly 1 OnStartTurn effect"
                )
                return sot_effs[0]
        raise AssertionError("ST19-14 not found in permanent stack")

    def test_timing_is_on_start_turn(self, debug_runner):
        """C# uses EffectTiming.OnStartTurn, NOT OnStartMainPhase."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, ["ST19-14"])
        eff = self._get_sot_effect(perm)
        assert eff.timing == EffectTiming.OnStartTurn

    def test_set_memory_to_3_when_at_0(self, debug_runner):
        """When memory is 0 (<= 2), set it to 3."""
        runner = debug_runner(initial_memory=0)
        game = runner.game
        perm = runner.place_on_field(1, ["ST19-14"])
        eff = self._get_sot_effect(perm)

        assert eff.can_use_condition({})
        eff.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        assert game.memory == 3

    def test_set_memory_to_3_when_at_2(self, debug_runner):
        """When memory is exactly 2 (<= 2), set it to 3."""
        runner = debug_runner(initial_memory=2)
        game = runner.game
        perm = runner.place_on_field(1, ["ST19-14"])
        eff = self._get_sot_effect(perm)

        eff.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        assert game.memory == 3

    def test_no_change_when_memory_is_3(self, debug_runner):
        """When memory is 3 (> 2), do not change it."""
        runner = debug_runner(initial_memory=3)
        game = runner.game
        perm = runner.place_on_field(1, ["ST19-14"])
        eff = self._get_sot_effect(perm)

        eff.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        assert game.memory == 3, "Memory should stay at 3, not be modified"

    def test_no_change_when_memory_is_5(self, debug_runner):
        """When memory is 5 (> 2), do not change it."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        perm = runner.place_on_field(1, ["ST19-14"])
        eff = self._get_sot_effect(perm)

        eff.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        assert game.memory == 5

    def test_condition_fails_when_not_on_field(self, debug_runner):
        """Condition should fail if tamer is not on the field."""
        runner = debug_runner(initial_memory=0)
        game = runner.game
        card = runner.inject_card(1, "ST19-14", "hand")
        effects = card.effect_list(None)
        sot_effs = [e for e in effects if e.timing == EffectTiming.OnStartTurn]
        assert len(sot_effs) == 1
        assert not sot_effs[0].can_use_condition({})


@pytest.mark.behavioral
class TestST19_14Effect1_GrantRush:
    """[Your Turn] When effects play Token/Puppet, suspend to grant Rush."""

    def _get_rush_effect(self, perm):
        """Find the OnEnterFieldAnyone rush-grant effect from ST19-14."""
        for cs in perm.card_sources:
            if cs.c_entity_base and cs.c_entity_base.card_id == "ST19-14":
                effects = cs.effect_list(None)
                enter_effs = [
                    e for e in effects
                    if e.timing == EffectTiming.OnEnterFieldAnyone
                ]
                assert len(enter_effs) == 1, (
                    "ST19-14 should have exactly 1 OnEnterFieldAnyone effect"
                )
                return enter_effs[0]
        raise AssertionError("ST19-14 not found in permanent stack")

    def test_effect_is_optional(self, debug_runner):
        """The rush-grant effect is optional (player chooses to suspend)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST19-14"])
        eff = self._get_rush_effect(perm)
        assert eff.is_optional, "Effect must be optional"

    def test_grant_rush_to_token_via_engine(self, debug_runner):
        """When a token is played, the engine auto-activates and grants Rush.

        effect_play_token fires OnEnterFieldAnyone which triggers the tamer
        effect. We verify the end state: tamer suspended + token has Rush.
        """
        runner = debug_runner(initial_memory=5)
        game = runner.game
        tamer_perm = runner.place_on_field(1, ["ST19-14"])

        game.effect_play_token(game.player1, 'diaboromon')
        token_perm = game.player1.battle_area[-1]

        assert tamer_perm.is_suspended, "Tamer should be suspended after activation"
        assert token_perm.has_keyword('_is_rush'), "Token should have Rush"

    def test_grant_rush_to_puppet_digimon(self, debug_runner):
        """When a Puppet-trait Digimon is played, grant it Rush via direct call."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        tamer_perm = runner.place_on_field(1, ["ST19-14"])
        eff = self._get_rush_effect(tamer_perm)

        # Place a Puppet Digimon (simulating effect play)
        puppet_perm = runner.place_on_field(1, ["BT2-055"])  # ToyAgumon, Puppet trait

        ctx = {
            'played_permanent': puppet_perm,
            'event_permanent': puppet_perm,
            'event_player': game.player1,
        }
        assert eff.can_use_condition(ctx), "Should trigger on Puppet Digimon"

        eff.on_process_callback({
            'player': game.player1,
            'game': game,
            **ctx,
        })
        assert tamer_perm.is_suspended
        assert puppet_perm.has_keyword('_is_rush')

    def test_rush_expires_at_end_of_turn(self, debug_runner):
        """Rush granted should be 'for the turn' (duration = turn_count), not permanent."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        tamer_perm = runner.place_on_field(1, ["ST19-14"])
        eff = self._get_rush_effect(tamer_perm)

        puppet_perm = runner.place_on_field(1, ["BT2-055"])

        ctx = {
            'played_permanent': puppet_perm,
            'event_permanent': puppet_perm,
            'event_player': game.player1,
        }
        eff.on_process_callback({
            'player': game.player1,
            'game': game,
            **ctx,
        })
        assert puppet_perm.has_keyword('_is_rush'), "Should have Rush now"

        # Check the grant has a turn-limited duration, not -1 (permanent)
        rush_duration = puppet_perm._granted_keywords.get('_is_rush')
        assert rush_duration != -1, (
            f"Rush duration should be turn-limited, not permanent (-1). Got: {rush_duration}"
        )

        # Simulate end of turn: clear expired grants
        puppet_perm.clear_expired_grants(game.turn_count + 1)
        assert not puppet_perm.has_keyword('_is_rush'), (
            "Rush should expire after the turn"
        )

    def test_no_trigger_when_tamer_already_suspended(self, debug_runner):
        """If the tamer is already suspended, cannot activate."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        tamer_perm = runner.place_on_field(1, ["ST19-14"], is_suspended=True)
        eff = self._get_rush_effect(tamer_perm)

        puppet_perm = runner.place_on_field(1, ["BT2-055"])

        ctx = {
            'played_permanent': puppet_perm,
            'event_permanent': puppet_perm,
            'event_player': game.player1,
        }
        assert not eff.can_use_condition(ctx), (
            "Should not trigger when tamer is already suspended"
        )

    def test_no_trigger_on_opponent_turn(self, debug_runner):
        """Should not trigger during opponent's turn."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        tamer_perm = runner.place_on_field(1, ["ST19-14"])
        eff = self._get_rush_effect(tamer_perm)

        game.player1.is_my_turn = False

        puppet_perm = runner.place_on_field(1, ["BT2-055"])

        ctx = {
            'played_permanent': puppet_perm,
            'event_permanent': puppet_perm,
            'event_player': game.player1,
        }
        assert not eff.can_use_condition(ctx)

    def test_no_trigger_on_non_puppet_digimon(self, debug_runner):
        """Should not trigger on a non-Puppet, non-Token Digimon."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        tamer_perm = runner.place_on_field(1, ["ST19-14"])
        eff = self._get_rush_effect(tamer_perm)

        # ST1-02 is Koromon — not Puppet trait
        non_puppet = runner.place_on_field(1, ["ST1-02"])

        ctx = {
            'played_permanent': non_puppet,
            'event_permanent': non_puppet,
            'event_player': game.player1,
        }
        assert not eff.can_use_condition(ctx), (
            "Should not trigger on non-Puppet, non-Token Digimon"
        )

    def test_no_trigger_on_opponent_token(self, debug_runner):
        """Should not trigger when opponent plays a token."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        tamer_perm = runner.place_on_field(1, ["ST19-14"])
        eff = self._get_rush_effect(tamer_perm)

        # Play token on opponent's field
        game.effect_play_token(game.player2, 'diaboromon')
        opp_token = game.player2.battle_area[-1]

        ctx = {
            'played_permanent': opp_token,
            'event_permanent': opp_token,
            'event_player': game.player2,
        }
        assert not eff.can_use_condition(ctx), (
            "Should not trigger on opponent's token"
        )


@pytest.mark.behavioral
class TestST19_14Effect2_Security:
    """[Security] Play this card without paying the cost."""

    def _get_security_effect(self, card):
        """Find the security effect from ST19-14."""
        effects = card.effect_list(None)
        sec_effs = [
            e for e in effects
            if e.timing == EffectTiming.SecuritySkill
        ]
        assert len(sec_effs) == 1, (
            "ST19-14 should have exactly 1 SecuritySkill effect"
        )
        return sec_effs[0]

    def test_security_timing(self, debug_runner):
        """Security effect must use SecuritySkill timing."""
        runner = debug_runner(initial_memory=0)
        card = runner.inject_card(1, "ST19-14", "hand")
        effects = card.effect_list(None)
        sec_effs = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_effs) == 1, (
            "Must have exactly 1 SecuritySkill-timed effect"
        )

    def test_security_is_security_effect(self, debug_runner):
        """Security effect must have is_security_effect = True."""
        runner = debug_runner(initial_memory=0)
        card = runner.inject_card(1, "ST19-14", "hand")
        eff = self._get_security_effect(card)
        assert eff.is_security_effect, "Must be marked as security effect"
