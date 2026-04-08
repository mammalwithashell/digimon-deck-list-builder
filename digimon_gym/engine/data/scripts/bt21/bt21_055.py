from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_055(CardScript):
    """BT21-055 Sunarizamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Your Turn] When this Digimon would digivolve into a card with the [Mineral] or
        # [Rock] trait, reduce the digivolution cost by 1.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.WhenWouldDigivolve)
        effect0.set_effect_name("BT21-055 Digi cost -1 for [Mineral] or [Rock]")
        effect0.set_effect_description(
            "[Your Turn] When this Digimon would digivolve into a card with the [Mineral] or "
            "[Rock] trait, reduce the digivolution cost by 1."
        )
        effect0.cost_reduction = 1

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            # Only reduce cost when digivolving into a [Mineral] or [Rock] trait Digimon
            target_card = context.get('card_source')
            if target_card:
                traits = getattr(target_card, 'card_traits', []) or []
                if not ('Mineral' in traits or 'Rock' in traits):
                    return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Inherited Effect: When effects trash this card from a [Mineral] or [Rock] trait
        # Digimon's digivolution cards, delete 1 of your opponent's Digimon with a play cost
        # of 4 or less.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDigivolutionCardDiscarded)
        effect1.set_effect_name("BT21-055 Delete opponent's Digimon cost 4 or less")
        effect1.set_effect_description(
            "When effects trash this card from a [Mineral] or [Rock] trait Digimon's "
            "digivolution cards, delete 1 of your opponent's Digimon with a play cost of 4 or less."
        )
        effect1.is_inherited_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Check this card was the one trashed
            trashed_cards = context.get('trashed_cards', [])
            if card not in trashed_cards:
                return False
            # Check the permanent has [Mineral] or [Rock] trait
            permanent = context.get('permanent')
            if permanent is None:
                return False
            if not (permanent.has_trait('Mineral') or permanent.has_trait('Rock')):
                return False
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
