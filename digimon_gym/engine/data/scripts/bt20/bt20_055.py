from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_055(CardScript):
    """BT20-055 Invisimon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEndTurn)
        effect0.set_effect_name("BT20-055 Play this card")
        effect0.set_effect_description("(Security) [End of Opponent's Turn] Play this card without paying the cost.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                return True

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        def build_main_effect(is_on_play: bool = False, is_when_digivolving: bool = False):
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect.set_effect_name(
                "BT20-055 <De-Digivolve 2> 1 opponent's Digimon, flip their security card faceup, then delete 1 Digimon"
            )
            effect.set_effect_description(
                "[On Play] <De-Digivolve 2> 1 of your opponent's Digimon and flip your opponent's top face-down security card face up. Then, delete 1 of their Digimon with 1 or fewer digivolution cards."
                if is_on_play else
                "[When Digivolving] <De-Digivolve 2> 1 of your opponent's Digimon and flip your opponent's top face-down security card face up. Then, delete 1 of their Digimon with 1 or fewer digivolution cards."
            )
            effect.is_on_play = is_on_play
            effect.is_when_digivolving = is_when_digivolving

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                def on_de_digivolve(target_perm):
                    removed = target_perm.de_digivolve(2)
                    enemy = player.enemy if player else None
                    if enemy:
                        enemy.trash_cards.extend(removed)

                game.effect_select_opponent_permanent(
                    player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

                enemy = player.enemy if player else None
                if enemy and enemy.security_cards:
                    pass

                def delete_filter(p):
                    return p.is_digimon and len(p.digivolution_cards) <= 1

                def on_delete(target_perm):
                    enemy = player.enemy if player else None
                    if enemy:
                        enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=delete_filter, is_optional=False)

            effect.set_on_process_callback(process)
            return effect

        effects.append(build_main_effect(is_on_play=True))
        effects.append(build_main_effect(is_when_digivolving=True))

        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnSecurityCheck)
        effect3.set_effect_name("BT20-055 Place top card face up as bottom security")
        effect3.set_effect_description(
            "[Your Turn] When your Digimon check face-up security cards, you may place this Digimon's top stacked card face up as the bottom security card."
        )
        effect3.is_optional = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect3.effect_source_permanent if hasattr(effect3, 'effect_source_permanent') else None
            if not (permanent and len(permanent.digivolution_cards) >= 0):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.recovery(1)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
