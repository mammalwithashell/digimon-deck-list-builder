from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_066(CardScript):
    """BT10-066 DarkKnightmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] <De-Digivolve 1> 1 of your opponent's Digimon. Then, if DigiXrosing with 2 cards, delete 1 of your opponent's Digimon with a play cost of 5 or less.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT10-066 De-Digivolve 1 to 1 Digimon and delete 1 Digimon with 5 or less Cost")
        effect0.set_effect_description("[On Play] <De-Digivolve 1> 1 of your opponent's Digimon. Then, if DigiXrosing with 2 cards, delete 1 of your opponent's Digimon with a play cost of 5 or less.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete, De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.WhenPermanentWouldBeDeleted
        # [All Turns] When this Digimon would be deleted, by returning 1 black level 4 or lower Digimon card from this Digimon's digivolution cards to its owner's hand, you may play 1 [SkullKnightmon] or [DeadlyAxemon] from this Digimon's digivolution cards without paying its memory cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.WhenPermanentWouldBeDeleted)
        effect1.set_effect_name("BT10-066 Return 1 digivolution card to play a Digimon from digivolution cards")
        effect1.set_effect_description("[All Turns] When this Digimon would be deleted, by returning 1 black level 4 or lower Digimon card from this Digimon's digivolution cards to its owner's hand, you may play 1 [SkullKnightmon] or [DeadlyAxemon] from this Digimon's digivolution cards without paying its memory cost.")
        effect1.is_optional = True
        effect1.set_hash_string("PlayDigimon_BT10_066")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card, Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not ('Black' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.None
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT10-066 Effect")
        effect2.set_effect_description("Effect")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
