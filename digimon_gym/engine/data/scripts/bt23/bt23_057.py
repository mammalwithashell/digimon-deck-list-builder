from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any

from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_057(CardScript):
    """BT23-057 Gankoomon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-057 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT23-057 play cost -5")
        effect1.set_effect_description(
            "When this card would be played, by returning 3 cards with [Huckmon], [Sistermon] or [Jesmon] in their names from your trash to the top or bottom of the deck, reduce the play cost by 5."
        )
        effect1.set_hash_string("PlayCost-5_BT23_057")
        effect1.cost_reduction = 5

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            return

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        def make_token_delete_effect(name: str, when_digivolving: bool) -> ICardEffect:
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect.set_effect_name(name)
            effect.set_effect_description(
                "Play 1 [Hinukamuy] Token. Then, delete 1 of your opponent's Digimon within the effect's play-cost cap."
            )
            if when_digivolving:
                effect.is_when_digivolving = True
            else:
                effect.is_on_play = True

            def condition(context: Dict[str, Any]) -> bool:
                return bool(card and card.permanent_of_this_card() is not None)

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get("player")
                game = ctx.get("game")
                owner_perm = ctx.get("permanent")
                if not (player and game and owner_perm):
                    return

                game.effect_play_token(player, "hinukamuy")
                max_play_cost = 6 + (3 * sum(1 for p in player.battle_area if p is not owner_perm and p.is_digimon))

                def target_filter(p):
                    if not p.is_digimon or not p.top_card:
                        return False
                    return p.top_card.get_cost_itself <= max_play_cost

                def on_delete(target_perm):
                    enemy = player.enemy if player else None
                    if enemy:
                        enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(player, on_delete, filter_fn=target_filter, is_optional=False)

            effect.set_on_process_callback(process)
            return effect

        effects.append(make_token_delete_effect("BT23-057 play Hinukamuy Token", when_digivolving=False))
        effects.append(make_token_delete_effect("BT23-057 play Hinukamuy Token", when_digivolving=True))

        return effects
