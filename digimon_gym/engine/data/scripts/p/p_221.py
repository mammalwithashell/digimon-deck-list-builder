from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_221(CardScript):
    """P-221 Chaosmon | Lv.7 (Yellow/Purple, Unique).

    Card text:
      <Security A. +1>
      <Partition (Yellow Lv.6 & Purple/Black Lv.6)>
      [When Digivolving] If DNA digivolving, until your opponent's turn
        ends, their effects don't affect this Digimon.
      [When Digivolving] [When Attacking] 1 of your opponent's Digimon gets
        -10000 DP until their turn ends.

    Clauses:
      C1 — <Security A. +1> factory keyword via `_security_attack_modifier = 1`.
      C2 — <Partition> documentary marker (engine gap: Partition recovery is
           not fully wired; BT23-102 pattern). `_is_partition = True` plus
           `_partition_conditions` metadata describing the two halves.
      C3 — [WD] DNA → CANNOT_BE_AFFECTED modifier on host perm, expiry
           'end_of_opponent_turn'. Only fires when ctx['is_dna_digivolve'].
      C4 — [WD] / [WA] → select 1 opp Digimon → register a CHANGE_DP
           modifier with value_fn using (cur, t, c) arity that returns
           `cur - 10000`, expiry 'end_of_opponent_turn' (card: "until their
           turn ends"), target-equality guard in `condition=`.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Timing: EffectTiming.None — DNA Jogress Condition ────────────
        effect0 = ICardEffect()
        effect0.set_effect_name("P-221 Jogress Condition")
        effect0.set_effect_description(
            "DNA Digivolve: Yellow Lv.6 + Purple/Black Lv.6 : Cost 0"
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ── C2: <Partition> documentary marker ──────────────────────────
        # Engine gap: Partition recovery not wired. We expose
        # `_is_partition` and `_partition_conditions` for RL observability
        # and future recovery logic.
        effect_partition = ICardEffect()
        effect_partition.set_effect_name("P-221 Partition")
        effect_partition.set_effect_description(
            "<Partition (Yellow Lv.6 & Purple/Black Lv.6)>"
        )
        effect_partition._is_partition = True
        # Two halves: (level, color_list)
        effect_partition._partition_conditions = [
            (6, ["Yellow"]),
            (6, ["Purple", "Black"]),
        ]

        def condition_partition(context: Dict[str, Any]) -> bool:
            return True
        effect_partition.set_can_use_condition(condition_partition)
        effects.append(effect_partition)

        # ── C1: <Security A. +1> factory keyword ────────────────────────
        effect_sa = ICardEffect()
        effect_sa.set_effect_name("P-221 Security Attack +1")
        effect_sa.set_effect_description(
            "<Security A. +1> (This Digimon checks 1 additional security card.)"
        )
        effect_sa._security_attack_modifier = 1

        def condition_sa(context: Dict[str, Any]) -> bool:
            return True
        effect_sa.set_can_use_condition(condition_sa)
        effects.append(effect_sa)

        # ── C3: [WD] If DNA digivolving → effects don't affect this ─────
        effect_immunity = ICardEffect()
        effect_immunity.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_immunity.set_effect_name(
            "P-221 [WD] DNA → opponent's effects don't affect this Digimon"
        )
        effect_immunity.set_effect_description(
            "[When Digivolving] If DNA digivolving, until your opponent's "
            "turn ends, their effects don't affect this Digimon."
        )
        effect_immunity.is_when_digivolving = True

        def condition_immunity(context: Dict[str, Any]) -> bool:
            # Field-presence gate
            if card is None or card.permanent_of_this_card() is None:
                return False
            return True
        effect_immunity.set_can_use_condition(condition_immunity)

        def process_immunity(ctx: Dict[str, Any]):
            game = ctx.get('game')
            perm = ctx.get('permanent') or (card.permanent_of_this_card() if card else None)
            if not (game and perm):
                return
            # Only on DNA digivolving
            if not ctx.get('is_dna_digivolve', False):
                return
            # Host target-equality guard on the modifier itself so queries
            # only affect Chaosmon, not other permanents.
            host_perm = perm
            from digimon_gym.engine.interfaces.modifiers import ModifierType
            game.register_modifier(
                host_perm,
                ModifierType.CANNOT_BE_AFFECTED,
                condition=lambda target, c, _p=host_perm: target is _p,
                value_fn=lambda cur, t, c: True,
                expiry='end_of_opponent_turn',
            )

        effect_immunity.set_on_process_callback(process_immunity)
        effects.append(effect_immunity)

        # ── C4a: [When Digivolving] -10000 DP to 1 opp Digimon ──────────
        effect_wd_dp = ICardEffect()
        effect_wd_dp.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_wd_dp.set_effect_name(
            "P-221 [WD] -10000 DP to 1 opponent's Digimon"
        )
        effect_wd_dp.set_effect_description(
            "[When Digivolving] 1 of your opponent's Digimon gets -10000 DP "
            "until their turn ends."
        )
        effect_wd_dp.is_when_digivolving = True

        def condition_wd_dp(context: Dict[str, Any]) -> bool:
            if card is None or card.permanent_of_this_card() is None:
                return False
            return True
        effect_wd_dp.set_can_use_condition(condition_wd_dp)
        effect_wd_dp.set_on_process_callback(
            lambda ctx: _process_minus_10000_dp(card, ctx)
        )
        effects.append(effect_wd_dp)

        # ── C4b: [When Attacking] -10000 DP to 1 opp Digimon ────────────
        effect_wa_dp = ICardEffect()
        effect_wa_dp.set_timing(EffectTiming.OnUseAttack)
        effect_wa_dp.set_effect_name(
            "P-221 [WA] -10000 DP to 1 opponent's Digimon"
        )
        effect_wa_dp.set_effect_description(
            "[When Attacking] 1 of your opponent's Digimon gets -10000 DP "
            "until their turn ends."
        )
        effect_wa_dp.is_on_attack = True

        def condition_wa_dp(context: Dict[str, Any]) -> bool:
            if card is None or card.permanent_of_this_card() is None:
                return False
            # Attacker gate: only fire when this perm is the attacker
            host = card.permanent_of_this_card()
            attacker = context.get('attacker')
            if attacker is not None and attacker is not host:
                return False
            return True
        effect_wa_dp.set_can_use_condition(condition_wa_dp)
        effect_wa_dp.set_on_process_callback(
            lambda ctx: _process_minus_10000_dp(card, ctx)
        )
        effects.append(effect_wa_dp)

        return effects


def _process_minus_10000_dp(card: 'CardSource', ctx: Dict[str, Any]):
    """Shared process: select 1 opponent's Digimon, register a CHANGE_DP
    modifier with value_fn `(cur, t, c) -> cur - 10000`, expiry
    'end_of_opponent_turn', target-equality guarded by condition.

    Per the No-Approximations policy, the target MUST be a player choice,
    not an auto-pick.
    """
    player = ctx.get('player')
    game = ctx.get('game')
    if not (player and game):
        return
    # Must have at least one opponent Digimon to do anything
    enemy = player.enemy
    if enemy is None:
        return
    has_target = any(p.is_digimon for p in enemy.battle_area)
    if not has_target:
        return

    from digimon_gym.engine.interfaces.modifiers import ModifierType

    def target_filter(p):
        return p.is_digimon

    def on_target(target_perm):
        # Target-equality guard: condition ensures the modifier only
        # applies to the selected target, not to every permanent queried.
        game.register_modifier(
            target_perm,
            ModifierType.CHANGE_DP,
            condition=lambda t, c, _tp=target_perm: t is _tp,
            value_fn=lambda cur, t, c: cur - 10000,
            expiry='end_of_opponent_turn',
        )

    game.effect_select_opponent_permanent(
        player,
        on_target,
        filter_fn=target_filter,
        is_optional=False,
        prompt="Select 1 of your opponent's Digimon for -10000 DP.",
    )
