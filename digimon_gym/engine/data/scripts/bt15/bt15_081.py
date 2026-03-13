from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_081(CardScript):
    """BT15-081 Leviamon (X Antibody) | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: security_attack_plus
        # Security Attack +1
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-081 Security Attack +1")
        effect0.set_effect_description("Security Attack +1")
        effect0._security_attack_modifier = 1

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT15-081 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 2
        effect1._alt_digi_cost = 2
        effect1._alt_digi_name = "Leviamon"

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Trash] [All Turns] When an effect play an opponent's Digimon or Tamer, 1 of your [Leviamon] or 1 of your Digimon with [X Antibody] in its digivolution cards may digivolve into this card without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT15-081 Digivolve 1 [Leviamon (X Antibody)] from trash")
        effect2.set_effect_description("[Trash] [All Turns] When an effect play an opponent's Digimon or Tamer, 1 of your [Leviamon] or 1 of your Digimon with [X Antibody] in its digivolution cards may digivolve into this card without paying the cost.")
        effect2.is_optional = True
        effect2.is_on_play = True

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
                if not ('Purple' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If your opponent has as many or more total Digimon and Tamers as you, delete 1 of your opponent's Tamers. Then, delete 1 of your opponent's level 3, 1 of their level 5 and 1 of their level 7 Digimon.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT15-081 Delete 1 Tamer and 1 level 3, 5, and 7  Digimon")
        effect3.set_effect_description("[When Digivolving] If your opponent has as many or more total Digimon and Tamers as you, delete 1 of your opponent's Tamers. Then, delete 1 of your opponent's level 3, 1 of their level 5 and 1 of their level 7 Digimon.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
