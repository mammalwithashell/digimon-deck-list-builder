from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_084(CardScript):
    """BT20-084 Sistermon Ciel (Awakened) | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-084 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Sistermon Ciel] for cost 1
        effect0._alt_digi_cost = 1
        effect0._alt_digi_name = "Sistermon Ciel"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Sistermon Ciel'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Trash] [All Turns] When any of your Digimon are played, 1 of your [Sistermon Ciel] may digivolve into this card without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-084 Digivolve 1 [Sistermon Ciel (Awakened)] from trash")
        effect1.set_effect_description("[Trash] [All Turns] When any of your Digimon are played, 1 of your [Sistermon Ciel] may digivolve into this card without paying the cost.")
        effect1.is_optional = True
        effect1.is_on_play = True

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
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your opponent's Digimon or Tamers can't suspend until the end of their turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-084 1 Digimon/Tamer can't suspend")
        effect2.set_effect_description("[On Play] 1 of your opponent's Digimon or Tamers can't suspend until the end of their turn.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your opponent's Digimon or Tamers can't suspend until the end of their turn.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-084 1 Digimon/Tamer can't suspend")
        effect3.set_effect_description("[When Digivolving] 1 of your opponent's Digimon or Tamers can't suspend until the end of their turn.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEndTurn
        # [End of All Turns] Place this Digimon's top stacked card as the top security card.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT20-084 Place top card as top security")
        effect4.set_effect_description("[End of All Turns] Place this Digimon's top stacked card as the top security card.")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and len(permanent.digivolution_cards) >= 0):
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Add To Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add top card of deck to security
            if player:
                player.recovery(1)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
