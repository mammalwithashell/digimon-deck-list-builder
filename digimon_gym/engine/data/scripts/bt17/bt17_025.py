from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_025(CardScript):
    """BT17-025 Cerberusmon: Werewolf Mode | Lv.5 Blue/Purple Digimon"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT17-025 When Digivolving: play Lv.3 from trash")
        effect0.set_effect_description(
            "[When Digivolving] You may play 1 level 3 blue or purple Digimon card from your trash or from one of your Digimon's digivolution cards without paying the cost."
        )
        effect0.is_when_digivolving = True
        effect0.is_optional = True

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

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) != 3:
                    return False
                colors = [col.name for col in (getattr(c, 'card_colors', []) or [])]
                return 'Blue' in colors or 'Purple' in colors

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT17-025 Inherited: bounce opponent Lv.3")
        effect1.set_effect_description(
            "[All Turns] [Once Per Turn] When an effect plays one of your Digimon, return 1 of your opponent's level 3 Digimon to the hand."
        )
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("AllTurns_BT17-025_BounceL3")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if context.get('event_player') is not card.owner:
                return False
            played_card = context.get('played_card')
            if not (played_card and getattr(played_card, 'is_digimon', False)):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon and p.level == 3

            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)

            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
