from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_099(CardScript):
    """BT19-099 The Wicked God Descends!"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] You may play 1 [Composite] trait Digimon card from your trash with the play cost reduced by 4. Then, place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT19-099 Play 1 [Composite] trait Digimon card from your trash with the play cost reduced by 4.")
        effect0.set_effect_description("[Main] You may play 1 [Composite] trait Digimon card from your trash with the play cost reduced by 4. Then, place this card in the battle area.")
        effect0.cost_reduction = 4

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Cost -4, Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return any(
                    'Composite' in trait for trait in (getattr(c, 'card_traits', []) or [])
                )
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=False,
                manual_reduction=4, is_optional=True)
            # Cost reduction by 4 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        # Delay
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-099 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When any of your Digimon with [Millenniummon] in its name would leave the battle area, <Delay>.\r\n• You may play 1 [Wicked God] trait Digimon card with a play cost 1 greater than that Digimon from your hand or trash without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenRemoveField)
        effect2.set_effect_name("BT19-099 Play 1 [Wicked God] trait Digimon card from hand or trash")
        effect2.set_effect_description("[All Turns] When any of your Digimon with [Millenniummon] in its name would leave the battle area, <Delay>.\r\n• You may play 1 [Wicked God] trait Digimon card with a play cost 1 greater than that Digimon from your hand or trash without paying the cost.")
        effect2.is_optional = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return any(
                    'Wicked God' in trait or 'WickedGod' in trait
                    for trait in (getattr(c, 'card_traits', []) or [])
                )
            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
