from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_095(CardScript):
    """BT20-095 Fellowship of Hope's Keepers"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Reveal the top 3 cards of your deck. Add 1 [Chronicle] trait card among them to the hand. Return the rest to the top or bottom of the deck. Then, place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-095 Add To Hand, Reveal And Select")
        effect0.set_effect_description("[Main] Reveal the top 3 cards of your deck. Add 1 [Chronicle] trait card among them to the hand. Return the rest to the top or bottom of the deck. Then, place this card in the battle area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Add To Hand, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if not (player and game):
                return
            def reveal_filter(c):
                return True
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [All Turns] When any of your [Chronicle] trait Digimon are deleted, <Delay>.\n・By moving your level 3 or higher Digimon from the breeding area to the battle area, it may digivolve into a [Chronicle] trait Digimon card in the hand or trash without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-095 Move 1 Digimon")
        effect1.set_effect_description("[All Turns] When any of your [Chronicle] trait Digimon are deleted, <Delay>.\n・By moving your level 3 or higher Digimon from the breeding area to the battle area, it may digivolve into a [Chronicle] trait Digimon card in the hand or trash without paying the cost.")
        effect1.is_optional = True
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if getattr(c, 'level', None) is None or c.level < 3:
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 [Chronicle] trait card with a play cost of 5 or less from your hand or trash without paying the cost. Then, place this card in the battle area.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-095 Play 1 [Chronicle] trait card from hand or trash")
        effect2.set_effect_description("[Security] You may play 1 [Chronicle] trait card with a play cost of 5 or less from your hand or trash without paying the cost. Then, place this card in the battle area.")
        effect2.is_security_effect = True
        effect2.is_security_effect = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card, Trash From Hand"""
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

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
