from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_013(CardScript):
    """BT19-013 Shoutmon X5 | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When this Digimon would leave the battle area, you may place up to 3 Digimon cards with the [Xros Heart] trait from this Digimon's digivolution cards under 1 of your Tamers.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.WhenRemoveField)
        effect0.set_effect_name("BT19-013 Save up to 3 Digimon cards with the [Xros Heart] trait.")
        effect0.set_effect_description("[All Turns] When this Digimon would leave the battle area, you may place up to 3 Digimon cards with the [Xros Heart] trait from this Digimon's digivolution cards under 1 of your Tamers.")
        effect0.is_optional = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may play 1 play cost 4 or lower Digimon card with the [Xros Heart] trait from under your Tamers without paying the cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("BT19-013 Play 1 play cost 4 or lower [Xros Heart] Digimon")
        effect1.set_effect_description("[On Deletion] You may play 1 play cost 4 or lower Digimon card with the [Xros Heart] trait from under your Tamers without paying the cost.")
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('Xros Heart' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.None
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-013 Effect")
        effect2.set_effect_description("Effect")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
