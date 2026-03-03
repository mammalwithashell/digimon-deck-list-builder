from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_056(CardScript):
    """BT22-056 Guardromon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-056 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.3 for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3
        effect0._alt_digi_color = CardColor.Black

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your opponent's Digimon gets -3000 DP for the turn. Then, if this Digimon's stack has 2 or more same-level cards, <De-Digivolve 1> 1 of your opponent's Digimon. (Trash the top card. You can't trash past level 3 cards.)
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT22-056 Give 1 digimon -3k DP. if 2 or more same level in sources, De-Digivolve 1")
        effect1.set_effect_description("[On Play] 1 of your opponent's Digimon gets -3000 DP for the turn. Then, if this Digimon's stack has 2 or more same-level cards, <De-Digivolve 1> 1 of your opponent's Digimon. (Trash the top card. You can't trash past level 3 cards.)")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your opponent's Digimon gets -3000 DP for the turn. Then, if this Digimon's stack has 2 or more same-level cards, <De-Digivolve 1> 1 of your opponent's Digimon. (Trash the top card. You can't trash past level 3 cards.)
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT22-056 Give 1 digimon -3k DP. if 2 or more same level in sources, De-Digivolve 1")
        effect2.set_effect_description("[When Digivolving] 1 of your opponent's Digimon gets -3000 DP for the turn. Then, if this Digimon's stack has 2 or more same-level cards, <De-Digivolve 1> 1 of your opponent's Digimon. (Trash the top card. You can't trash past level 3 cards.)")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: dp_modifier
        # DP modifier
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-056 DP modifier")
        effect3.set_effect_description("DP modifier")
        effect3.is_inherited_effect = True
        effect3.dp_modifier = 2000

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
