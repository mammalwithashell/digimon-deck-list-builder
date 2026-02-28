from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_080(CardScript):
    """BT10-080 SkullBaluchimon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDiscardHand
        # When one of your effects trashes this card in your hand, if it's your turn, 1 of your Digimon may digivolve into 1 purple Digimon card with [Undead] or [Dark Animal] in its traits from your trash for its digivolution cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-080 Your Digimon can digivolve to trash cards")
        effect0.set_effect_description("When one of your effects trashes this card in your hand, if it's your turn, 1 of your Digimon may digivolve into 1 purple Digimon card with [Undead] or [Dark Animal] in its traits from your trash for its digivolution cost.")
        effect0.is_optional = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not ('Purple' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT10-080 This Digimon gets effects")
        effect1.set_effect_description("Effect")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Deletion] Delete 1 of your opponent's Digimon.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT10-080 Delete 1 Digimon")
        effect2.set_effect_description("[On Deletion] Delete 1 of your opponent's Digimon.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete, Add Temp Effect"""
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
            # Grant temporary effect to target permanent
            pass  # descriptive-tagged: add_temp_effect

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
