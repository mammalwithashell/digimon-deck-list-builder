from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_099(CardScript):
    """BT23-099 The Sistermon Sisters Training Gym"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Bypass Option color requirement check in action mask
        card._match_color_requirement = False

        # Timing: EffectTiming.OptionSkill
        # [Main] <Draw 1> Then, place this card in the battle area.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("BT23-099 Draw 1")
        effect1.set_effect_description("[Main] <Draw 1> Then, place this card in the battle area.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: delay
        # Delay
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-099 Delay")
        effect2.set_effect_description("Delay")
        effect2.is_on_play = True
        effect2._is_delay = True

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # [Your Turn] When any of your Digimon digivolve into a Digimon with
        # [Huckmon] or [Jesmon] in its name, play 1 [Sistermon] from hand/trash.
        # Uses _is_digivolve_observer pattern (fires from _fire_digivolve_observers)
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-099 Play Sistermon on Huckmon/Jesmon digivolve")
        effect3.set_effect_description("[Your Turn] When any of your Digimon digivolve into a Digimon with [Huckmon] or [Jesmon] in its name, you may play 1 card with [Sistermon] in its name from your hand or trash without paying the cost.")
        effect3.is_optional = True
        effect3._is_digivolve_observer = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check that the digivolved permanent has Huckmon or Jesmon in name
            digi_perm = context.get('digivolved_permanent')
            if digi_perm and digi_perm.top_card:
                names = getattr(digi_perm.top_card, 'card_names', []) or []
                if any('Huckmon' in n or 'Jesmon' in n for n in names):
                    return True
            return False

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Sistermon from hand or trash"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return any('Sistermon' in _n for _n in getattr(c, 'card_names', []))
            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 card with [Sistermon] in its name from your hand or trash without paying the cost. Then, place this card in the battle area.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.SecuritySkill)
        effect4.set_effect_name("BT23-099 Play Card")
        effect4.set_effect_description("[Security] You may play 1 card with [Sistermon] in its name from your hand or trash without paying the cost. Then, place this card in the battle area.")
        effect4.is_security_effect = True
        effect4.is_security_effect = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not (any('Sistermon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
