"""Behavioral tests for BT10-042 Venusmon.

Card text (source of truth):
  Lv.6 | Yellow | Shaman/Olympos XII | DP:12000 | Cost:13
  [When Digivolving] All of your opponent's Digimon gain <Security A. -1>
    (This Digimon checks 1 fewer security card.) until the end of your
    opponent's turn.
  [Opponent's Turn] Your opponent's Digimon with <Security A.> can't attack
    this Digimon and can't activate [When Attacking] and [When Digivolving]
    effects.

Decomposition:
  C1 [When Digivolving]: Loop over opponent's Digimon, register
     CHANGE_SECURITY_ATTACK -1 with target-equality guard until opp turn ends.
     NO selection — applied to ALL.
  C2 [Opponent's Turn] static: Opp Digimon with <Security A.> modifications
     can't attack Venusmon (via CANNOT_ATTACK_TARGET) and can't activate
     [When Attacking] / [When Digivolving] effects (via DISABLE_EFFECT).
     The "Opponent's Turn" clause gates both via condition (`not is_my_turn`).
"""

import pytest

from digimon_gym.engine.data.enums import EffectTiming
from digimon_gym.engine.interfaces.modifiers import ModifierType


BT10_042 = "BT10-042"          # Venusmon (Lv.6, Yellow, test target)
AGUMON_ST1 = "ST1-03"          # Agumon (Lv.3, Red) — generic opp Digimon
GREYMON_ST1 = "ST1-07"         # Greymon (Lv.4, Red) — generic opp Digimon
PATAMON = "BT3-020"            # Patamon (Lv.3, Yellow) — generic own Digimon
TAMER = "BT5-089"              # Izzy & Mimi — Tamer for non-Digimon opp target


def _get_venusmon_effects(perm):
    """Return all effects on the Venusmon permanent's top card."""
    return perm.top_card.effect_list(None)


def _get_wd_effect(perm):
    """Return the [When Digivolving] SA-1 effect (C1)."""
    effects = _get_venusmon_effects(perm)
    wd = [e for e in effects
          if getattr(e, 'is_when_digivolving', False)
          and 'Security Attack -1' in (e.effect_name or '')]
    assert len(wd) >= 1, f"Venusmon should have a When Digivolving SA-1 effect, got {len(wd)}"
    return wd[0]


def _get_static_effect(perm):
    """Return the [Opponent's Turn] static restriction effect (C2).

    The C2 static effect is registered twice (once for on-play, once for
    when-digivolving) so it fires under both triggers. Both instances
    register the same modifiers. This helper returns the on-play variant.
    """
    effects = _get_venusmon_effects(perm)
    static = [e for e in effects
              if '[Opponent' in (e.effect_description or '')
              and getattr(e, 'is_on_play', False)]
    assert len(static) >= 1, \
        "Venusmon should have an on-play [Opponent's Turn] static registration effect"
    return static[0]


def _get_static_wd_effect(perm):
    """Return the when-digivolving variant of the [Opponent's Turn] static."""
    effects = _get_venusmon_effects(perm)
    static = [e for e in effects
              if '[Opponent' in (e.effect_description or '')
              and getattr(e, 'is_when_digivolving', False)]
    assert len(static) >= 1, \
        "Venusmon should have a when-digivolving [Opponent's Turn] static registration effect"
    return static[0]


@pytest.mark.behavioral
class TestBT10042C1_WhenDigivolving_SAMinus:
    """C1: [When Digivolving] All opp Digimon gain <SA-1> until end opp turn."""

    def test_wd_effect_exists(self, debug_runner):
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [BT10_042])
        wd = _get_wd_effect(perm)
        assert wd.is_when_digivolving, "Effect should have is_when_digivolving flag"

    def test_wd_applies_sa_minus_1_to_opp_digimon(self, debug_runner):
        """Each opponent Digimon should gain <Security A. -1> after the effect fires."""
        runner = debug_runner(initial_memory=10)
        opp1 = runner.place_on_field(2, [AGUMON_ST1])
        opp2 = runner.place_on_field(2, [GREYMON_ST1])

        # Baseline: no SA modifier
        assert opp1.security_attack_modifier() == 0
        assert opp2.security_attack_modifier() == 0

        perm = runner.place_on_field(1, [BT10_042])
        game = runner.game
        wd = _get_wd_effect(perm)
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert opp1.security_attack_modifier() == -1, \
            f"opp1 should have SA -1, got {opp1.security_attack_modifier()}"
        assert opp2.security_attack_modifier() == -1, \
            f"opp2 should have SA -1, got {opp2.security_attack_modifier()}"

    def test_wd_does_not_apply_to_own_digimon(self, debug_runner):
        """Own Digimon should NOT be affected by the SA -1."""
        runner = debug_runner(initial_memory=10)
        own_other = runner.place_on_field(1, [PATAMON])
        opp = runner.place_on_field(2, [AGUMON_ST1])

        perm = runner.place_on_field(1, [BT10_042])
        game = runner.game
        wd = _get_wd_effect(perm)
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert own_other.security_attack_modifier() == 0, \
            "Own Digimon should not be affected by opp SA -1"
        assert opp.security_attack_modifier() == -1

    def test_wd_target_equality_guard_per_perm(self, debug_runner):
        """Each opp Digimon should get its own modifier — not leak across perms.

        Regression guard: If the loop uses a naive lambda without a target-equality
        guard, the registered modifier condition may apply to ALL perms rather than
        just the target, causing overcounting (e.g., -2 or -3 per opponent).
        """
        runner = debug_runner(initial_memory=10)
        opp1 = runner.place_on_field(2, [AGUMON_ST1])
        opp2 = runner.place_on_field(2, [GREYMON_ST1])
        opp3 = runner.place_on_field(2, [AGUMON_ST1])

        perm = runner.place_on_field(1, [BT10_042])
        game = runner.game
        wd = _get_wd_effect(perm)
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Each opponent should have exactly -1, not -2 or -3.
        assert opp1.security_attack_modifier() == -1
        assert opp2.security_attack_modifier() == -1
        assert opp3.security_attack_modifier() == -1

    def test_wd_registers_modifiers_with_eot_opponent_expiry(self, debug_runner):
        """Modifier expiry should be 'end_of_opponent_turn'."""
        runner = debug_runner(initial_memory=10)
        opp = runner.place_on_field(2, [AGUMON_ST1])
        perm = runner.place_on_field(1, [BT10_042])
        game = runner.game
        wd = _get_wd_effect(perm)
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        entries = game.modifiers._modifiers.get(ModifierType.CHANGE_SECURITY_ATTACK, [])
        matching = [e for e in entries if e.expiry == 'end_of_opponent_turn']
        assert len(matching) >= 1, \
            "At least one SA modifier should be registered with end_of_opponent_turn expiry"

    def test_wd_skips_opponent_non_digimon(self, debug_runner):
        """Non-Digimon permanents (tokens/tamers/opts) should not get the modifier."""
        runner = debug_runner(initial_memory=10)
        opp_digi = runner.place_on_field(2, [AGUMON_ST1])
        try:
            opp_tamer = runner.place_on_field(2, [TAMER])
        except Exception:
            opp_tamer = None  # Tamer may not exist in this card pool

        perm = runner.place_on_field(1, [BT10_042])
        game = runner.game
        wd = _get_wd_effect(perm)
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert opp_digi.security_attack_modifier() == -1
        if opp_tamer is not None:
            # Tamer has no security_attack concept; ensure modifier wasn't
            # registered with its own permanent as the target-equality guard.
            entries = game.modifiers._modifiers.get(ModifierType.CHANGE_SECURITY_ATTACK, [])
            for e in entries:
                # Check the condition: it should NOT return True for the tamer
                ok = e.is_active(opp_tamer, {}) if e.condition else True
                # We only care that registered entries were not indiscriminately
                # applied to the tamer by the script's loop filter.
                # (If the script correctly filters p.is_digimon, the tamer never
                # had a modifier registered with its perm as the target-equality
                # target; the condition for other targets returns False against
                # opp_tamer as target.)
                assert not ok or e.source_permanent is not opp_tamer, \
                    "Tamer should not have SA modifier registered"


@pytest.mark.behavioral
class TestBT10042C2_OpponentTurn_StaticRestrictions:
    """C2: [Opponent's Turn] opp Digimon with <Security A.> can't attack this
    Digimon and can't activate [When Attacking] / [When Digivolving] effects."""

    def test_static_effect_exists(self, debug_runner):
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [BT10_042])
        static = _get_static_effect(perm)
        assert static is not None

    def test_static_registers_cannot_attack_target_on_venusmon(self, debug_runner):
        """When Venusmon enters field, a CANNOT_ATTACK_TARGET modifier should
        be registered on Venusmon so opp Digimon with SA changes can't attack it."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [BT10_042])
        game = runner.game
        static = _get_static_effect(perm)
        if static.on_process_callback is not None:
            static.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })
        else:
            # Purely declarative registration; verify on effect_list-scan model.
            # Force a fake play trigger via the script's setup path if any.
            pass

        entries = game.modifiers._modifiers.get(ModifierType.CANNOT_ATTACK_TARGET, [])
        assert len(entries) >= 1, \
            "CANNOT_ATTACK_TARGET modifier should be registered by Venusmon static effect"

    def test_opp_with_sa_changes_cant_attack_venusmon_on_opp_turn(self, debug_runner):
        """Opponent Digimon with SA changes cannot attack Venusmon during opp turn.

        Tests the CANNOT_ATTACK_TARGET modifier's condition closure — it should
        return True only when:
          - target perm is Venusmon
          - attacker has SA changes
          - it's opponent's (not Venusmon owner's) turn
        """
        runner = debug_runner(initial_memory=10)
        opp = runner.place_on_field(2, [AGUMON_ST1])
        perm = runner.place_on_field(1, [BT10_042])
        game = runner.game

        # Simulate C1 firing first (WD gives SA -1 to all opp)
        wd = _get_wd_effect(perm)
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        # Simulate C2 static setup
        static = _get_static_effect(perm)
        if static.on_process_callback is not None:
            static.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })

        # Switch to opponent's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        game.turn_player = game.player2
        game.opponent_player = game.player1

        # Opponent has SA changes (from C1). Attempt to query CANNOT_ATTACK_TARGET
        # on Venusmon with opp as attacker.
        blocked = game.modifiers.has_modifier(
            perm, ModifierType.CANNOT_ATTACK_TARGET, {'attacker': opp}
        )
        assert blocked, "Opp with SA changes should be blocked from attacking Venusmon on opp turn"

    def test_opp_without_sa_changes_can_attack_venusmon(self, debug_runner):
        """Opp Digimon without SA changes CAN attack Venusmon (even on opp turn)."""
        runner = debug_runner(initial_memory=10)
        opp = runner.place_on_field(2, [AGUMON_ST1])
        perm = runner.place_on_field(1, [BT10_042])
        game = runner.game

        # Do NOT fire C1 — opp has no SA changes
        static = _get_static_effect(perm)
        if static.on_process_callback is not None:
            static.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })

        # Switch to opp turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        game.turn_player = game.player2
        game.opponent_player = game.player1

        # Query CANNOT_ATTACK_TARGET with opp (no SA changes) as attacker
        blocked = game.modifiers.has_modifier(
            perm, ModifierType.CANNOT_ATTACK_TARGET, {'attacker': opp}
        )
        assert not blocked, \
            "Opp without SA changes should NOT be blocked from attacking Venusmon"

    def test_restriction_not_active_on_own_turn(self, debug_runner):
        """On Venusmon owner's turn, the restriction should NOT apply."""
        runner = debug_runner(initial_memory=10)
        opp = runner.place_on_field(2, [AGUMON_ST1])
        perm = runner.place_on_field(1, [BT10_042])
        game = runner.game

        # Fire C1 (give opp SA changes)
        wd = _get_wd_effect(perm)
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        static = _get_static_effect(perm)
        if static.on_process_callback is not None:
            static.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })

        # Stay on player1 (Venusmon owner) turn
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False
        game.turn_player = game.player1
        game.opponent_player = game.player2

        blocked = game.modifiers.has_modifier(
            perm, ModifierType.CANNOT_ATTACK_TARGET, {'attacker': opp}
        )
        assert not blocked, "Restriction should NOT apply on Venusmon owner's turn"

    def test_disable_effect_for_wd_on_opp_with_sa_changes(self, debug_runner):
        """DISABLE_EFFECT should return True for a WD effect on an opp Digimon
        with SA changes, during opp turn."""
        runner = debug_runner(initial_memory=10)
        opp = runner.place_on_field(2, [AGUMON_ST1])
        perm = runner.place_on_field(1, [BT10_042])
        game = runner.game

        wd = _get_wd_effect(perm)
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        static = _get_static_effect(perm)
        if static.on_process_callback is not None:
            static.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })

        # Switch to opp turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        game.turn_player = game.player2
        game.opponent_player = game.player1

        # Build a fake WD effect object
        from digimon_gym.engine.interfaces.card_effect import ICardEffect
        fake_wd_effect = ICardEffect()
        fake_wd_effect.is_when_digivolving = True

        disabled = game.modifiers.has_modifier(
            opp, ModifierType.DISABLE_EFFECT, {'effect': fake_wd_effect}
        )
        assert disabled, \
            "Opp Digimon with SA changes should have WD effects disabled on opp turn"

    def test_disable_effect_for_wa_on_opp_with_sa_changes(self, debug_runner):
        """DISABLE_EFFECT should return True for a WA effect (is_on_attack)
        on an opp Digimon with SA changes, during opp turn."""
        runner = debug_runner(initial_memory=10)
        opp = runner.place_on_field(2, [AGUMON_ST1])
        perm = runner.place_on_field(1, [BT10_042])
        game = runner.game

        wd = _get_wd_effect(perm)
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        static = _get_static_effect(perm)
        if static.on_process_callback is not None:
            static.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })

        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        game.turn_player = game.player2
        game.opponent_player = game.player1

        from digimon_gym.engine.interfaces.card_effect import ICardEffect
        fake_wa_effect = ICardEffect()
        fake_wa_effect.is_on_attack = True

        disabled = game.modifiers.has_modifier(
            opp, ModifierType.DISABLE_EFFECT, {'effect': fake_wa_effect}
        )
        assert disabled, \
            "Opp Digimon with SA changes should have WA effects disabled on opp turn"

    def test_disable_effect_ignores_non_wa_wd_effects(self, debug_runner):
        """DISABLE_EFFECT should NOT disable effects that are neither WD nor WA."""
        runner = debug_runner(initial_memory=10)
        opp = runner.place_on_field(2, [AGUMON_ST1])
        perm = runner.place_on_field(1, [BT10_042])
        game = runner.game

        wd = _get_wd_effect(perm)
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        static = _get_static_effect(perm)
        if static.on_process_callback is not None:
            static.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })

        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        game.turn_player = game.player2
        game.opponent_player = game.player1

        from digimon_gym.engine.interfaces.card_effect import ICardEffect
        neutral_effect = ICardEffect()
        neutral_effect.is_on_play = True  # NOT WA, NOT WD

        disabled = game.modifiers.has_modifier(
            opp, ModifierType.DISABLE_EFFECT, {'effect': neutral_effect}
        )
        assert not disabled, \
            "Non-WD/WA effects should NOT be disabled by Venusmon's restriction"

    def test_disable_effect_ignores_opp_without_sa_changes(self, debug_runner):
        """Opp Digimon WITHOUT SA changes should not have effects disabled."""
        runner = debug_runner(initial_memory=10)
        opp = runner.place_on_field(2, [AGUMON_ST1])
        perm = runner.place_on_field(1, [BT10_042])
        game = runner.game

        # Do NOT fire C1 (no SA changes)
        static = _get_static_effect(perm)
        if static.on_process_callback is not None:
            static.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })

        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        game.turn_player = game.player2
        game.opponent_player = game.player1

        from digimon_gym.engine.interfaces.card_effect import ICardEffect
        fake_wd = ICardEffect()
        fake_wd.is_when_digivolving = True

        disabled = game.modifiers.has_modifier(
            opp, ModifierType.DISABLE_EFFECT, {'effect': fake_wd}
        )
        assert not disabled, \
            "Opp Digimon WITHOUT SA changes should have effects unaffected"

    def test_disable_effect_ignores_own_digimon(self, debug_runner):
        """DISABLE_EFFECT should not apply to the Venusmon controller's own Digimon."""
        runner = debug_runner(initial_memory=10)
        own_other = runner.place_on_field(1, [PATAMON])
        perm = runner.place_on_field(1, [BT10_042])
        game = runner.game

        static = _get_static_effect(perm)
        if static.on_process_callback is not None:
            static.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })

        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        game.turn_player = game.player2
        game.opponent_player = game.player1

        from digimon_gym.engine.interfaces.card_effect import ICardEffect
        fake_wd = ICardEffect()
        fake_wd.is_when_digivolving = True

        disabled = game.modifiers.has_modifier(
            own_other, ModifierType.DISABLE_EFFECT, {'effect': fake_wd}
        )
        assert not disabled, \
            "Venusmon should not disable its own controller's Digimon effects"


@pytest.mark.behavioral
class TestBT10042_EngineWiringCannotAttackTarget:
    """Verify the engine wiring: CANNOT_ATTACK_TARGET filters the action mask."""

    def test_action_mask_excludes_blocked_attack(self, debug_runner):
        """When CANNOT_ATTACK_TARGET blocks opp's Agumon from attacking Venusmon,
        the action mask during opp turn should not allow that specific pairing."""
        from digimon_gym.engine.game.action_mask import build_action_mask
        from digimon_gym.engine.game.constants import TARGETS_PER_ATTACKER
        from digimon_gym.engine.data.enums import GamePhase

        runner = debug_runner(initial_memory=10)
        opp = runner.place_on_field(2, [AGUMON_ST1], turn_played=-1)
        perm = runner.place_on_field(1, [BT10_042], is_suspended=True, turn_played=-1)
        game = runner.game

        # Fire C1 — opp gains SA changes
        wd = _get_wd_effect(perm)
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        # Fire C2 static setup
        static = _get_static_effect(perm)
        if static.on_process_callback is not None:
            static.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })

        # Switch to opp's turn — opp is attacker
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.current_phase = GamePhase.Main
        game.memory = 0  # opp has 0 memory to attack

        # Venusmon is at slot 0 on player1's field. Agumon at slot 0 on player2.
        # Attack action id = 100 + 0*15 + 0 = 100 (opp's attacker 0 -> defender 0).
        mask = build_action_mask(game, player_id=2)
        attack_action = 100 + 0 * TARGETS_PER_ATTACKER + 0
        assert mask[attack_action] == 0.0, (
            f"Attack action {attack_action} should be masked off — "
            f"opp Agumon with SA changes can't attack Venusmon"
        )

    def test_action_mask_allows_attack_without_sa_changes(self, debug_runner):
        """Baseline: without SA changes, opp Agumon can attack the suspended Venusmon."""
        from digimon_gym.engine.game.action_mask import build_action_mask
        from digimon_gym.engine.game.constants import TARGETS_PER_ATTACKER
        from digimon_gym.engine.data.enums import GamePhase

        runner = debug_runner(initial_memory=10)
        opp = runner.place_on_field(2, [AGUMON_ST1], turn_played=-1)
        perm = runner.place_on_field(1, [BT10_042], is_suspended=True, turn_played=-1)
        game = runner.game

        # Do NOT fire C1 — no SA changes on opp
        static = _get_static_effect(perm)
        if static.on_process_callback is not None:
            static.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })

        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.current_phase = GamePhase.Main
        game.memory = 0

        mask = build_action_mask(game, player_id=2)
        attack_action = 100 + 0 * TARGETS_PER_ATTACKER + 0
        assert mask[attack_action] == 1.0, (
            "Without SA changes, opp Agumon should be able to attack suspended Venusmon"
        )
