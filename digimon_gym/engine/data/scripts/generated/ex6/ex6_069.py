from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_069(CardScript):
    """EX6-069 Rise of the Seven Great Demon Lords"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] You may place 1 Digimon card with the [Seven Great Demon Lords] trait from your hand or trash as the bottom digivolution card of the [Gate of Deadly Sins] in your breeding area. Then, place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-069 Place 1 Digimon with [Seven Great Demon Lords] trait as bottom digivolution source in breeding area.")
        effect0.set_effect_description("[Main] You may place 1 Digimon card with the [Seven Great Demon Lords] trait from your hand or trash as the bottom digivolution card of the [Gate of Deadly Sins] in your breeding area. Then, place this card in the battle area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: delay
        # Delay
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-069 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [All Turns] When one of your [Seven Great Demon Lords] trait Digimon is deleted, <Delay>. \r\n • You may play 1 [Seven Great Demon Lords] trait Digimon from the digivolution cards of your [Gate of Deadly Sins] in the breeding area without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX6-069 Play 1 Digimon from breeding area")
        effect2.set_effect_description("[All Turns] When one of your [Seven Great Demon Lords] trait Digimon is deleted, <Delay>. \r\n • You may play 1 [Seven Great Demon Lords] trait Digimon from the digivolution cards of your [Gate of Deadly Sins] in the breeding area without paying the cost.")
        effect2.is_optional = True
        effect2.is_on_deletion = True

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
                if not (any('Gate of Deadly Sins' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
