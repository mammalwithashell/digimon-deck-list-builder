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
                if not (any('Dex' in _n or 'DeathX' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                if getattr(c, 'level', None) is None or c.level > 6:
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When any of your [DexDorugoramon] would leave the battle area, <Delay>.\r\n� By return 1 [Dorumon] from those Digimon's digivolution cards to the hand, you may play 1 [DeathXmon] from your trash without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-097 Play 1 [DeathXmon]")
        effect1.set_effect_description("[All Turns] When any of your [DexDorugoramon] would leave the battle area, <Delay>.\r\n� By return 1 [Dorumon] from those Digimon's digivolution cards to the hand, you may play 1 [DeathXmon] from your trash without paying the cost.")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('DexDorugoramon'))):
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
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 [Dorumon] from your hand or trash without paying the cost. Then, place this card in the battle area.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-097 Play Card, Trash From Hand, Add To Hand")
        effect2.set_effect_description("[Security] You may play 1 [Dorumon] from your hand or trash without paying the cost. Then, place this card in the battle area.")
        effect2.is_security_effect = True
        effect2.is_security_effect = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card, Trash From Hand, Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
