from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST18_10(CardScript):
    """ST18-10 GrandGalemon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] [When Digivolving] Suspend 1 Digimon. If this effect
        # suspends your Digimon, you may play 1 Digimon card with the [Bird]
        # or [Avian] in any of its traits with 3000 DP or less from your hand
        # without paying the cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("ST18-10 Suspend 1 Digimon, may play Bird/Avian 3000 DP or less")
        effect0.set_effect_description(
            "[On Play] [When Digivolving] Suspend 1 Digimon. If this effect "
            "suspends your Digimon, you may play 1 Digimon card with [Bird] "
            "or [Avian] in any of its traits with 3000 DP or less "
            "from your hand without paying the cost."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon

            def on_suspend(target_perm):
                target_perm.suspend()
                # Check if we suspended our own Digimon
                if target_perm in player.battle_area:
                    def play_filter(c):
                        if not getattr(c, 'is_digimon', False):
                            return False
                        dp = getattr(c, 'dp', 0) or 0
                        if dp > 3000:
                            return False
                        traits = getattr(c, 'card_traits', []) or []
                        return any('Bird' in t or 'Avian' in t for t in traits)

                    game.effect_play_from_zone(
                        player, 'hand', play_filter, free=True, is_optional=True)

            game.effect_select_any_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Same effect for When Digivolving
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("ST18-10 When Digivolving: Suspend 1 Digimon, may play Bird/Avian")
        effect1.set_effect_description(
            "[When Digivolving] Suspend 1 Digimon. If this effect suspends your "
            "Digimon, you may play 1 Digimon card with [Bird] or [Avian] in any "
            "of its traits with 3000 DP or less from your hand without "
            "paying the cost."
        )
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon

            def on_suspend(target_perm):
                target_perm.suspend()
                if target_perm in player.battle_area:
                    def play_filter(c):
                        if not getattr(c, 'is_digimon', False):
                            return False
                        dp = getattr(c, 'dp', 0) or 0
                        if dp > 3000:
                            return False
                        traits = getattr(c, 'card_traits', []) or []
                        return any('Bird' in t or 'Avian' in t for t in traits)

                    game.effect_play_from_zone(
                        player, 'hand', play_filter, free=True, is_optional=True)

            game.effect_select_any_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Inherited: [Your Turn] [Once Per Turn] When this Digimon attacks your
        # opponent's Digimon, you may unsuspend this Digimon.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnAllyAttack)
        effect2.set_effect_name("ST18-10 May unsuspend when attacking opponent's Digimon")
        effect2.set_effect_description(
            "[Your Turn] [Once Per Turn] When this Digimon attacks your "
            "opponent's Digimon, you may unsuspend this Digimon."
        )
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.is_on_attack = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("ST18_10_ESS_Unsuspend")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            perm = ctx.get('permanent')
            if perm:
                perm.unsuspend()

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
