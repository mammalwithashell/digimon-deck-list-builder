from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_048(CardScript):
    """EX8-048 Landramon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX8-048 Play 1 [Close] from your hand")
        effect0.set_effect_description(
            "[When Digivolving] If you have 1 or fewer Tamers, you may play 1 [Close] from your hand without paying the cost."
        )
        effect0.is_optional = True
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner:
                tamer_count = sum(1 for p in card.owner.battle_area if p.is_tamer)
                if tamer_count > 1:
                    return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                return any('Close' in name for name in getattr(c, 'card_names', []))

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDigivolutionCardDiscarded)
        effect1.set_effect_name("EX8-048 Delete 4 cost or less Digimon")
        effect1.set_effect_description(
            "When effects trash this card from a [Mineral] or [Rock] trait Digimon's digivolution cards, delete 1 of your opponent's Digimon with a play cost of 4 or less."
        )
        effect1.is_inherited_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon and getattr(p.top_card, 'get_cost_itself', 0) <= 4

            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
