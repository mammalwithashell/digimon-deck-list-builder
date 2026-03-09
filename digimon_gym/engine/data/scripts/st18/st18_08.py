from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST18_08(CardScript):
    """ST18-08 Galemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Security] You may play 1 card with the [LIBERATOR] trait and a play
        # cost of 4 or less from your hand or trash without paying the cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("ST18-08 Security: Play 1 LIBERATOR cost 4 or less")
        effect0.set_effect_description(
            "[Security] You may play 1 card with the [LIBERATOR] trait and a play "
            "cost of 4 or less from your hand or trash without paying the cost."
        )
        effect0.is_security_effect = True
        effect0.is_optional = True

        def condition_sec(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition_sec)

        def process_sec(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                cost = getattr(c, 'play_cost', 99) or 99
                return any('LIBERATOR' in t for t in traits) and cost <= 4

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process_sec)
        effects.append(effect0)

        # <Vortex>
        effect1 = ICardEffect()
        effect1.set_effect_name("ST18-08 Vortex")
        effect1.set_effect_description("Vortex")
        effect1._is_vortex = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # [When Attacking] [Once Per Turn] You may suspend 1 of your Digimon.
        # If this effect suspended your Digimon, 1 of your Digimon gets +4000 DP
        # for the turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnAllyAttack)
        effect2.set_effect_name("ST18-08 Suspend own Digimon for +4000 DP")
        effect2.set_effect_description(
            "[When Attacking] [Once Per Turn] You may suspend 1 of your Digimon. "
            "If this effect suspended your Digimon, 1 of your Digimon gets +4000 DP "
            "for the turn."
        )
        effect2.is_on_attack = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("ST18_08_WA_Suspend")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon and not p.is_suspended

            def on_suspend(target_perm):
                target_perm.suspend()
                # Grant +4000 DP to 1 of your Digimon
                def dp_filter(p):
                    return p.is_digimon
                def on_dp_grant(dp_target):
                    dp_target.change_dp(4000)
                game.effect_select_own_permanent(
                    player, on_dp_grant, filter_fn=dp_filter, is_optional=False)

            game.effect_select_own_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Inherited: [Your Turn] This Digimon gets +2000 DP.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.StatModifier)
        effect3.set_effect_name("ST18-08 +2000 DP")
        effect3.set_effect_description("[Your Turn] This Digimon gets +2000 DP.")
        effect3.is_inherited_effect = True
        effect3.dp_modifier = 2000

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
