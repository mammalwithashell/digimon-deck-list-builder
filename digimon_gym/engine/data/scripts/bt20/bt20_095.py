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
                if getattr(c, 'is_digi_egg', False):
                    return False
                if not getattr(c, 'has_play_cost', False):
                    return False
                if getattr(c, 'get_cost_itself', 0) > 5:
                    return False
                if not (any('Chronicle' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        # Delay
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-095 Delay")
        effect1.set_effect_description("Delay")
        effect1.is_on_deletion = True
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [All Turns] When any of your [Chronicle] trait Digimon are deleted, <Delay>.\n・By moving your level 3 or higher Digimon from the breeding area to the battle area, it may digivolve into a [Chronicle] trait Digimon card in the hand or trash without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-095 Move 1 Digimon")
        effect2.set_effect_description("[All Turns] When any of your [Chronicle] trait Digimon are deleted, <Delay>.\\n・By moving your level 3 or higher Digimon from the breeding area to the battle area, it may digivolve into a [Chronicle] trait Digimon card in the hand or trash without paying the cost.")
        effect2.is_optional = True
        effect2.is_on_deletion = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if getattr(c, 'is_digi_egg', False):
                    return False
                if not getattr(c, 'has_play_cost', False):
                    return False
                if getattr(c, 'get_cost_itself', 0) > 5:
                    return False
                if not (any('Chronicle' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 [Chronicle] trait card with a play cost of 5 or less from your hand or trash without paying the cost. Then, place this card in the battle area.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-095 Play 1 [Chronicle] trait card from hand or trash")
        effect3.set_effect_description("[Security] You may play 1 [Chronicle] trait card with a play cost of 5 or less from your hand or trash without paying the cost. Then, place this card in the battle area.")
        effect3.is_security_effect = True
        effect3.is_security_effect = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
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
                if getattr(c, 'is_digi_egg', False):
                    return False
                if not getattr(c, 'has_play_cost', False):
                    return False
                if getattr(c, 'get_cost_itself', 0) > 5:
                    return False
                if not (any('Chronicle' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
