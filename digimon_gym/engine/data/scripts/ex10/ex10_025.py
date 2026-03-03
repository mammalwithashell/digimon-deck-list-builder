from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_025(CardScript):
    """EX10-025 Sunarizamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX10-025 Place 2 as bottom digivolution sources")
        effect0.set_effect_description(
            "[On Play] You may place 2 cards with the [Mineral] or [Rock] trait from your trash as 1 of your [Mineral] or [Rock] trait Digimon's bottom digivolution cards."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def source_filter(source_card) -> bool:
                traits = getattr(source_card, 'card_traits', [])
                return 'Mineral' in traits or 'Rock' in traits

            if not any(source_filter(c) for c in player.trash_cards):
                return

            def target_filter(p):
                return p.is_digimon and (p.has_trait('Mineral') or p.has_trait('Rock'))

            def on_target(target_perm):
                placed = 0
                for source_card in list(player.trash_cards):
                    if placed >= 2:
                        break
                    if not source_filter(source_card):
                        continue
                    player.trash_cards.remove(source_card)
                    target_perm.add_card_source_bottom(source_card)
                    placed += 1

            game.effect_select_own_permanent(
                player, on_target, filter_fn=target_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDigivolutionCardDiscarded)
        effect1.set_effect_name("EX10-025 Delete 4 cost or less Digimon")
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
