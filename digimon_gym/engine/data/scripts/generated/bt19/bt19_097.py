from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_097(CardScript):
    """BT19-097 Bonds of True Love"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDiscardLibrary
        # When this card is trashed from your deck, you may place it in the battle area.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-097 Place this card in the battle Area")
        effect0.set_effect_description("When this card is trashed from your deck, you may place it in the battle area.")
        effect0.is_optional = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] Trash the top 2 cards of your deck. Then, place this card in the battle area.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-097 Trash 2 cards of your deck and place this card in the battle area.")
        effect1.set_effect_description("[Main] Trash the top 2 cards of your deck. Then, place this card in the battle area.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Mill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Mill 2 cards from own deck
            if player and player.library_cards:
                mill_count = min(2, len(player.library_cards))
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: delay
        # Delay
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-097 Delay")
        effect2.set_effect_description("Delay")
        effect2._is_delay = True

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnStartTurn
        # [Start of your Turn] If you don't have a Digimon, <Delay>.\r\n� You may play 1 [Impmon] from your trash without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT19-097 Play an [Impmon] from trash.")
        effect3.set_effect_description("[Start of your Turn] If you don't have a Digimon, <Delay>.\r\n� You may play 1 [Impmon] from your trash without paying the cost.")
        effect3.is_optional = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('Impmon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
