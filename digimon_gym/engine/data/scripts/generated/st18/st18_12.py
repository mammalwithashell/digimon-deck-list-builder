from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from .....core.card_script import CardScript
from .....interfaces.card_effect import ICardEffect
from .....data.enums import EffectTiming

if TYPE_CHECKING:
    from .....core.card_source import CardSource


class ST18_12(CardScript):
    """ST18-12 Zephagamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Vortex>
        effect0 = ICardEffect()
        effect0.set_effect_name("ST18-12 Vortex")
        effect0.set_effect_description("Vortex")
        effect0._is_vortex = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [When Digivolving] You may unsuspend this Digimon. Then, suspend 1 of
        # your opponent's Digimon.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("ST18-12 May unsuspend self, then suspend 1 opponent's Digimon")
        effect1.set_effect_description(
            "[When Digivolving] You may unsuspend this Digimon. Then, suspend 1 "
            "of your opponent's Digimon."
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
            if not (player and perm and game):
                return
            # May unsuspend self
            if perm.is_suspended:
                perm.unsuspend()
            # Then, suspend 1 opponent's Digimon
            def target_filter(p):
                return p.is_digimon

            def on_suspend(target_perm):
                target_perm.suspend()

            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [All Turns] [Once Per Turn] When any Digimon suspend, you may play 1
        # green Digimon card with [Avian] or [Bird] in any of its traits and a
        # play cost of 5 or less from your hand without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnTappedAnyone)
        effect2.set_effect_name("ST18-12 When any Digimon suspend, play 1 green Bird/Avian cost 5 or less")
        effect2.set_effect_description(
            "[All Turns] [Once Per Turn] When any Digimon suspend, you may play "
            "1 green Digimon card with [Avian] or [Bird] in any of its traits "
            "and a play cost of 5 or less from your hand without paying the cost."
        )
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("ST18_12_AT_Play")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                cost = getattr(c, 'play_cost', 99) or 99
                if cost > 5:
                    return False
                colors = [col.name for col in getattr(c, 'card_colors', [])]
                if 'Green' not in colors:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Avian' in t or 'Bird' in t for t in traits)

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
