from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_024(CardScript):
    """BT19-024 MarineBullmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: decode
        # Decode
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-024 Decode")
        effect0.set_effect_description("Decode")
        effect0._is_decode = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may place 1 Digimon card with [Aqua]/[Sea Animal] in one of its traits from your hand as 1 of your Digimon's bottom digivolution card.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-024 Place a card from hand as the bottom digivolution card of 1 of our Digimon")
        effect1.set_effect_description("[On Play] You may place 1 Digimon card with [Aqua]/[Sea Animal] in one of its traits from your hand as 1 of your Digimon's bottom digivolution card.")
        effect1.is_optional = True
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may place 1 Digimon card with [Aqua]/[Sea Animal] in one of its traits from your hand as 1 of your Digimon's bottom digivolution card.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-024 Place a card from hand as the bottom digivolution card of 1 of our Digimon")
        effect2.set_effect_description("[When Digivolving] You may place 1 Digimon card with [Aqua]/[Sea Animal] in one of its traits from your hand as 1 of your Digimon's bottom digivolution card.")
        effect2.is_optional = True
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEndAttack
        # [End of Attack] (Once Per Turn) You may play 1 level 4 or lower Digimon card with [Aqua]/[Sea Animal] in one of its traits from this Digimon's digivolution cards without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT19-024 Play 1 Digimon from this Digimon's digivolution cards")
        effect3.set_effect_description("[End of Attack] (Once Per Turn) You may play 1 level 4 or lower Digimon card with [Aqua]/[Sea Animal] in one of its traits from this Digimon's digivolution cards without paying the cost.")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("PlayFromSources_BT19_024")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
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
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
