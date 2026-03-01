from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_092(CardScript):
    """BT16-092 Invincible Dragon-Insect Fusion"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] You may play 1 [ExVeemon] or [Stingmon] from your hand without paying the cost. Then, 2 of your Digimon may DNA digivolve into a Digimon card in your hand. Until the end of your opponent's turn, the Digimon this effect DNA digivolved can't be deleted in battle and gains <Blocker>.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT16-092 Play 1 [ExVeemon] or [Stingmon]")
        effect0.set_effect_description("[Main] You may play 1 [ExVeemon] or [Stingmon] from your hand without paying the cost. Then, 2 of your Digimon may DNA digivolve into a Digimon card in your hand. Until the end of your opponent's turn, the Digimon this effect DNA digivolved can't be deleted in battle and gains <Blocker>.")
        effect0._is_blocker = True
        effect0._is_cannot_be_deleted_by_battle = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card, Gain Keyword Blocker, Gain Keyword Cannot Be Deleted By Battle"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if perm:
                perm.grant_keyword('_is_blocker')
                perm.grant_keyword('_is_cannot_be_deleted_by_battle')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 [Veemon] or [Wormmon] from your hand or trash without paying the cost. Then, add this card to the hand.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT16-092 Play 1 [Veemon] or [Wormmon]")
        effect1.set_effect_description("[Security] You may play 1 [Veemon] or [Wormmon] from your hand or trash without paying the cost. Then, add this card to the hand.")
        effect1.is_security_effect = True
        effect1.is_security_effect = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
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
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
