from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_100(CardScript):
    """BT20-100 The Last Guardian"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Reveal the top 3 cards of your deck. Add 1 [Cool Boy] and 1 card with the [Royal Knight] or [X Antibody] trait among them to the hand. Return the rest to the bottom of the deck. Then, place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-100 Reveal the top 3 cards, add 1 [Cool Boy] and 1 [Royal Knight]/[X Antibody] trait")
        effect0.set_effect_description("[Main] Reveal the top 3 cards of your deck. Add 1 [Cool Boy] and 1 card with the [Royal Knight] or [X Antibody] trait among them to the hand. Return the rest to the bottom of the deck. Then, place this card in the battle area.")

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
                if not (any('Omekamon' in _n or 'Cool Boy' in _n for _n in getattr(c, 'card_names', []))):
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
        effect1.set_effect_name("BT20-100 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When any of your Digimon with [Omnimon] in their names would leave the battle area, <Delay>.\r\n� 1 of those Digimon doesn't leave.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-100 Prevent Removal")
        effect2.set_effect_description("[All Turns] When any of your Digimon with [Omnimon] in their names would leave the battle area, <Delay>.\r\n� 1 of those Digimon doesn't leave.")
        effect2.is_optional = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 [Omekamon] or [Cool Boy] from your hand or trash without paying the cost. Then, place this card in the battle area.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-100 Play Card")
        effect3.set_effect_description("[Security] You may play 1 [Omekamon] or [Cool Boy] from your hand or trash without paying the cost. Then, place this card in the battle area.")
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
                if not (any('Omekamon' in _n or 'Cool Boy' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
