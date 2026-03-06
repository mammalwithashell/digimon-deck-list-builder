from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_016(CardScript):
    """BT10-016 Jesmon (X Antibody) | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-016 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Jesmon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 Digimon card with [Sistermon] in its name from your hand or trash without paying its memory cost. Then, if [Jesmon] is in this Digimon's digivolution cards or you have a Digimon with [Sistermon] in its name in play, until the end of your opponent's turn, all of your Digimon may also attack unsuspended Digimon and get +2000 DP.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT10-016 Play 1 Digimon from hand or trash and your Digimon get effects")
        effect1.set_effect_description("[When Digivolving] You may play 1 Digimon card with [Sistermon] in its name from your hand or trash without paying its memory cost. Then, if [Jesmon] is in this Digimon's digivolution cards or you have a Digimon with [Sistermon] in its name in play, until the end of your opponent's turn, all of your Digimon may also attack unsuspended Digimon and get +2000 DP.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card, Attack Unsuspended"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('Sistermon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Can attack unsuspended Digimon via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CAN_ATTACK_UNSUSPENDED, perm,
                    value_fn=lambda: True, expiry='persistent')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
