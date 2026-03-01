from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_069(CardScript):
    """BT11-069 MetalGreymon (X Antibody) | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-069 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 1
        effect0._alt_digi_cost = 1

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Until the end of your opponent's turn, this Digimon can't have its DP reduced by your opponent's effects, and isn't affected by <De-Digivolve> effects. Then, if [MetalGreymon] or [X Antibody] is in this Digimon's digivolution cards, delete 1 of your opponent's Digimon with 6000 DP or less.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT11-069 This Digimon gets effects and delete 1 Digimon with 6000 DP or less")
        effect1.set_effect_description("[When Digivolving] Until the end of your opponent's turn, this Digimon can't have its DP reduced by your opponent's effects, and isn't affected by <De-Digivolve> effects. Then, if [MetalGreymon] or [X Antibody] is in this Digimon's digivolution cards, delete 1 of your opponent's Digimon with 6000 DP or less.")
        effect1.is_when_digivolving = True
        effect1._is_immune_dp_minus = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete, Gain Keyword Immune Dp Minus"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.dp is None or p.dp > 6000:
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            if perm:
                perm.grant_keyword('_is_immune_dp_minus')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnUnTappedAnyone
        # [Opponent's Turn][Once Per Turn] When a Digimon becomes unsuspended, if this Digimon has [Greymon] or [Omnimon] in its name, trash the top card of your opponent's security stack.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUnTappedAnyone)
        effect2.set_effect_name("BT11-069 Trash the top card of opponent's security")
        effect2.set_effect_description("[Opponent's Turn][Once Per Turn] When a Digimon becomes unsuspended, if this Digimon has [Greymon] or [Omnimon] in its name, trash the top card of your opponent's security stack.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("TrashSecurity_BT11_069")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
