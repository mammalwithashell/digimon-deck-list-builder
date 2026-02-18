from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_091(CardScript):
    """BT13-091 Belphemon: Rage Mode | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] Delete all of your opponent's level 5 or lower Digimon. Then, if you have 6 or fewer cards in your hand, this Digimon gets +3000 DP and gains <Security Attack +1> for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-091 Delete opponent's all level 5 or lower Digimons and this Digimon gets effects")
        effect0.set_effect_description("[Start of Your Main Phase] Delete all of your opponent's level 5 or lower Digimon. Then, if you have 6 or fewer cards in your hand, this Digimon gets +3000 DP and gains <Security Attack +1> for the turn.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: DP +3000, Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(3000)
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 5:
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndAttack
        # [End of Attack][Once Per Turn] By deleting 1 of your other Digimon, unsuspend this Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-091 Delete your another Digimon to unsuspend this Digimon")
        effect1.set_effect_description("[End of Attack][Once Per Turn] By deleting 1 of your other Digimon, unsuspend this Digimon.")
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Unsuspend_BT13_091")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEndTurn
        # [End of Opponent�f Turn] If this Digimon is [Belphemon: Sleep Mode], trash the top card of this Digimon.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-091 Trash the top card of this Digimon")
        effect2.set_effect_description("[End of Opponent�f Turn] If this Digimon is [Belphemon: Sleep Mode], trash the top card of this Digimon. ")
        effect2.is_inherited_effect = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and len(permanent.digivolution_cards) >= 1):
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
