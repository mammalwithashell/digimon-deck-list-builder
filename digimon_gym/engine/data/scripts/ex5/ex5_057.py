from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_057(CardScript):
    """EX5-057 Labramon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By trashing 1 card in your hand, you may return 1 Digimon card with the [Dark Animal]/[Shaman] trait from your trash to the hand.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX5-057 Trash 1 card from hand to return 1 card from trash to hand")
        effect0.set_effect_description("[On Play] By trashing 1 card in your hand, you may return 1 Digimon card with the [Dark Animal]/[Shaman] trait from your trash to the hand.")
        effect0.is_optional = True
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash From Hand, Add To Hand"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)
            for card_to_add in list(player.trash_cards):
                traits = getattr(card_to_add, 'card_traits', []) or []
                if any('Dark Animal' in trait or 'DarkAnimal' in trait or 'Shaman' in trait for trait in traits):
                    player.trash_cards.remove(card_to_add)
                    player.hand_cards.append(card_to_add)
                    break

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] [Once Per Turn] When an effect plays one of your Digimon, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX5-057 Memory +1")
        effect1.set_effect_description("[Your Turn] [Once Per Turn] When an effect plays one of your Digimon, gain 1 memory.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Memory1_EX5_065")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if context.get('event_player') is not card.owner:
                return False
            played_card = context.get('played_card')
            return bool(played_card and getattr(played_card, 'is_digimon', False))

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
