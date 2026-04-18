"""Behavioral tests for EX8-030 Tapirmon (Lv.3 Yellow Digimon, DP 2000, Cost 3).

Card text:
    [All Turns] Your opponent can't gain memory other than by Tamer effects.

Key semantics (mirrors DCGO EX8_030.cs):
- Continuous [All Turns] static restriction registered as CANNOT_ADD_MEMORY.
- Applies to the *opponent* of Tapirmon's owner only.
- Tamer-sourced memory gains are explicitly exempted (DCGO: !IsTamerEffect).
- Modifier only active while Tapirmon is on the battle area (IsExistOnBattleArea).
- Owner of Tapirmon is completely unaffected.

Pattern reference: BT3-061 Chuumon (identical effect text, different color).
"""

import pytest

from digimon_gym.engine.data.enums import EffectTiming
from digimon_gym.engine.interfaces.card_effect import ICardEffect
from digimon_gym.engine.interfaces.modifiers import ModifierType


def _trigger_tapirmon_on_play(runner, perm):
    """Fire Tapirmon's [All Turns] on-play effect through the engine stack.

    Going through ``game._invoke_effect_callback`` ensures the effect is
    pushed onto the active-effect stack, which is the same code path used
    by real effect resolution.
    """
    game = runner.game
    top = perm.top_card
    for eff in top.effect_list(None):
        if eff.on_process_callback and (
            getattr(eff, 'is_on_play', False)
            or eff.timing == EffectTiming.OnEnterFieldAnyone
        ):
            # Mirror the engine's _collect_triggered_effects bookkeeping so
            # effect_source_permanent is populated for the condition.
            eff.set_effect_source_permanent(perm)
            ctx = {
                'game': game,
                'player': game.player1 if perm in game.player1.battle_area else game.player2,
                'permanent': perm,
                'card': eff.effect_source_card,
                'turn_player': game.turn_player,
                'opponent_player': game.opponent_player,
            }
            game._invoke_effect_callback(eff, ctx)


def _make_synthetic_effect(source_card=None, is_tamer_effect=False, source_permanent=None):
    """Build a bare ICardEffect used to impersonate a card-effect-sourced memory gain.

    The engine's ``Player.add_memory`` looks at ``game.current_effect``; by
    pushing one of these onto the stack via ``_invoke_effect_callback`` we
    can simulate Digimon-sourced vs Tamer-sourced gains in a controlled way.
    """
    eff = ICardEffect()
    eff.effect_source_card = source_card
    eff.is_tamer_effect = is_tamer_effect
    if source_permanent is not None:
        eff.set_effect_source_permanent(source_permanent)
    return eff


def _gain_as_effect(game, gaining_player, amount, effect):
    """Call ``gaining_player.add_memory`` while ``effect`` is on the active stack.

    Simulates a script that, inside its ``on_process_callback``, calls
    ``player.add_memory(amount)``. The CANNOT_ADD_MEMORY gate reads
    ``game.current_effect`` which is set via the effect stack.
    """
    captured = {}

    def _cb(_ctx):
        captured['before'] = game.memory
        gaining_player.add_memory(amount)
        captured['after'] = game.memory

    effect.on_process_callback = _cb
    game._invoke_effect_callback(effect, {'game': game})
    return captured['before'], captured['after']


@pytest.mark.behavioral
class TestEX8030Tapirmon:
    """Tests for EX8-030 Tapirmon."""

    # ─── Effect structure ────────────────────────────────────────────

    def test_has_memory_blocking_effect(self, debug_runner):
        """Tapirmon must expose a declared memory-blocking effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-030"])
        top = perm.top_card
        effects = top.effect_list(None)
        memory_effects = [
            e for e in effects
            if "memory" in (e.effect_description or "").lower()
            or "memory" in (e.effect_name or "").lower()
        ]
        assert len(memory_effects) >= 1, (
            f"Should have a memory-related effect. Effects: "
            f"{[(e.effect_name, e.effect_description) for e in effects]}"
        )

    def test_on_play_registers_cannot_add_memory(self, debug_runner):
        """Firing the on-play effect registers a CANNOT_ADD_MEMORY modifier."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-030"])
        game = runner.game

        before_count = len(game.modifiers._modifiers.get(ModifierType.CANNOT_ADD_MEMORY, []))
        _trigger_tapirmon_on_play(runner, perm)
        after_count = len(game.modifiers._modifiers.get(ModifierType.CANNOT_ADD_MEMORY, []))

        assert after_count == before_count + 1, (
            "Tapirmon on-play should register exactly one CANNOT_ADD_MEMORY modifier"
        )

    # ─── Clause: opponent blocked from Digimon effects ──────────────

    def test_opponent_blocked_from_digimon_sourced_gain(self, debug_runner):
        """A Digimon-sourced memory gain by the opponent must be fully blocked."""
        runner = debug_runner(initial_memory=5)
        tapir_perm = runner.place_on_field(1, ["EX8-030"])
        game = runner.game
        _trigger_tapirmon_on_play(runner, tapir_perm)

        # Simulate an opponent-side Digimon permanent with an effect that
        # calls add_memory. Use a real Digimon card as the source permanent.
        opp_digi_perm = runner.place_on_field(2, ["BT1-010"])  # Koromon (Lv.3 Digimon)
        digi_effect = _make_synthetic_effect(source_permanent=opp_digi_perm)

        before, after = _gain_as_effect(game, game.player2, 3, digi_effect)
        # Opponent (P2) is not turn_player by default; its gain subtracts from memory
        # in engine accounting. The block should leave memory untouched.
        assert before == after, (
            f"Opponent Digimon-sourced memory gain should be blocked. "
            f"before={before} after={after}"
        )

    # ─── Clause: Tamer exemption ────────────────────────────────────

    def test_opponent_tamer_sourced_gain_is_allowed(self, debug_runner):
        """Opponent memory gain sourced from a Tamer card effect must go through."""
        runner = debug_runner(initial_memory=5)
        tapir_perm = runner.place_on_field(1, ["EX8-030"])
        game = runner.game
        _trigger_tapirmon_on_play(runner, tapir_perm)

        # Build a synthetic Tamer effect. We detect "Tamer effect" via either
        # explicit is_tamer_effect flag OR effect_source_permanent.top_card.is_tamer.
        # Use a real Tamer card as the source permanent to exercise the
        # permanent-inspection fallback.
        opp_tamer_perm = runner.place_on_field(2, ["ST1-12"])  # Tai Kamiya (Tamer)
        assert opp_tamer_perm.top_card.is_tamer, "Test fixture must be a Tamer card"
        tamer_effect = _make_synthetic_effect(source_permanent=opp_tamer_perm)

        before, after = _gain_as_effect(game, game.player2, 2, tamer_effect)
        assert after == before - 2, (
            f"Tamer-sourced memory gain should NOT be blocked. "
            f"before={before} after={after}"
        )

    def test_opponent_tamer_effect_flag_is_respected(self, debug_runner):
        """The is_tamer_effect flag alone is sufficient to exempt a gain."""
        runner = debug_runner(initial_memory=5)
        tapir_perm = runner.place_on_field(1, ["EX8-030"])
        game = runner.game
        _trigger_tapirmon_on_play(runner, tapir_perm)

        # No source permanent/card — only the is_tamer_effect flag.
        tamer_effect = _make_synthetic_effect(is_tamer_effect=True)

        before, after = _gain_as_effect(game, game.player2, 1, tamer_effect)
        assert after == before - 1, (
            "is_tamer_effect=True must bypass the CANNOT_ADD_MEMORY block"
        )

    # ─── Clause: owner unaffected ────────────────────────────────────

    def test_owner_still_gains_from_digimon_effects(self, debug_runner):
        """Tapirmon's owner (P1) should be unaffected by the block."""
        runner = debug_runner(initial_memory=5)
        tapir_perm = runner.place_on_field(1, ["EX8-030"])
        game = runner.game
        _trigger_tapirmon_on_play(runner, tapir_perm)

        owner_digi_perm = runner.place_on_field(1, ["BT1-010"])
        digi_effect = _make_synthetic_effect(source_permanent=owner_digi_perm)

        before, after = _gain_as_effect(game, game.player1, 2, digi_effect)
        assert after == before + 2, (
            f"Owner should freely gain memory. before={before} after={after}"
        )

    def test_owner_still_gains_from_direct_engine_path(self, debug_runner):
        """Direct ``player.add_memory`` with no active effect is not a 'card effect' gain."""
        runner = debug_runner(initial_memory=5)
        tapir_perm = runner.place_on_field(1, ["EX8-030"])
        game = runner.game
        _trigger_tapirmon_on_play(runner, tapir_perm)

        before = game.memory
        game.player1.add_memory(1)
        assert game.memory == before + 1

    # ─── Clause: [All Turns] — works on both turns ───────────────────

    def test_blocks_opponent_on_opponent_turn(self, debug_runner):
        """The block still applies when it is the opponent's turn."""
        runner = debug_runner(initial_memory=5)
        tapir_perm = runner.place_on_field(1, ["EX8-030"])
        game = runner.game
        _trigger_tapirmon_on_play(runner, tapir_perm)

        # Swap turns so P2 is the turn player.
        game.turn_player, game.opponent_player = game.opponent_player, game.turn_player
        game.turn_player.is_my_turn = True
        game.opponent_player.is_my_turn = False

        opp_digi_perm = runner.place_on_field(2, ["BT1-010"])
        digi_effect = _make_synthetic_effect(source_permanent=opp_digi_perm)
        before, after = _gain_as_effect(game, game.player2, 3, digi_effect)
        assert before == after, "Block should hold on the opponent's own turn"

    # ─── Clause: field-presence gate ─────────────────────────────────

    def test_block_disabled_when_tapirmon_leaves_field(self, debug_runner):
        """Removing Tapirmon from the field lifts the CANNOT_ADD_MEMORY gate."""
        runner = debug_runner(initial_memory=5)
        tapir_perm = runner.place_on_field(1, ["EX8-030"])
        game = runner.game
        _trigger_tapirmon_on_play(runner, tapir_perm)

        # Confirm block is live first.
        opp_digi_perm = runner.place_on_field(2, ["BT1-010"])
        digi_effect = _make_synthetic_effect(source_permanent=opp_digi_perm)
        before, after = _gain_as_effect(game, game.player2, 1, digi_effect)
        assert before == after, "Precondition: block should be active"

        # Remove Tapirmon from the field.
        game.player1.battle_area.remove(tapir_perm)

        # Block condition checks battle_area membership — the modifier
        # should now be inert.
        before2, after2 = _gain_as_effect(game, game.player2, 2, digi_effect)
        assert after2 == before2 - 2, (
            f"With Tapirmon off-field the block must not apply. "
            f"before={before2} after={after2}"
        )

    # ─── Sanity: lose_memory path unaffected ─────────────────────────

    def test_opponent_can_still_lose_memory(self, debug_runner):
        """Negative memory changes (losses) are never blocked."""
        runner = debug_runner(initial_memory=5)
        tapir_perm = runner.place_on_field(1, ["EX8-030"])
        game = runner.game
        _trigger_tapirmon_on_play(runner, tapir_perm)

        before = game.memory
        game.player2.lose_memory(2)
        # player2 losing memory: self is not turn_player → game.memory -= (-2) → +2
        # (in engine accounting, opponent "losing" memory actually increases the
        # displayed memory towards turn_player). The important thing is that
        # the call goes through — no block.
        assert game.memory != before, "lose_memory should not be blocked"
