from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_097(CardScript):
    """BT20-097 The Apostle of Doom Descends!"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] 1 of your Digimon may digivolve into a level 6 or lower Digimon card with [Dex] or [DeathX] in its name in the trash with the digivolution cost reduced by 4. Then, place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-097 May Digivolve into level 6 or lower, then place in battle area")
        effect0.set_effect_description("[Main] 1 of your Digimon may digivolve into a level 6 or lower Digimon card with [Dex] or [DeathX] in its name in the trash with the digivolution cost reduced by 4. Then, place this card in the battle area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 6:
                    return False
                if not (any('Dex' in _n or 'DeathX' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        # Delay
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-097 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('DexDorugoramon'))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When any of your [DexDorugoramon] would leave the battle area, <Delay>.\r\n� By return 1 [Dorumon] from those Digimon's digivolution cards to the hand, you may play 1 [DeathXmon] from your trash without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-097 Play 1 [DeathXmon]")
        effect2.set_effect_description("[All Turns] When any of your [DexDorugoramon] would leave the battle area, <Delay>.\\r\\n� By return 1 [Dorumon] from those Digimon's digivolution cards to the hand, you may play 1 [DeathXmon] from your trash without paying the cost.")
        effect2.is_optional = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('DexDorugoramon'))):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card, Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 6:
                    return False
                if not (any('Dex' in _n or 'DeathX' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 [Dorumon] from your hand or trash without paying the cost. Then, place this card in the battle area.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-097 Play Card, Add To Hand")
        effect3.set_effect_description("[Security] You may play 1 [Dorumon] from your hand or trash without paying the cost. Then, place this card in the battle area.")
        effect3.is_security_effect = True
        effect3.is_security_effect = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card, Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not (any('Dorumon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
