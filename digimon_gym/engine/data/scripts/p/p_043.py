from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_043(CardScript):
    """P-043 Kudamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may return 1 [Kentaurosmon] from your trash to the bottom of your deck to trigger <Recovery +1 (Deck)>. (Place the top card of your deck on top of your security stack.)
        effect0 = ICardEffect()
        effect0.set_effect_name("P-043 Return 1 [Kentaurosmon] from trash to the bottom of deck to trigger Recovery +1 (Deck)")
        effect0.set_effect_description("[On Play] You may return 1 [Kentaurosmon] from your trash to the bottom of your deck to trigger <Recovery +1 (Deck)>. (Place the top card of your deck on top of your security stack.)")
        effect0.is_optional = True
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Recovery +1, Return To Deck"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.recovery(1)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_return(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)
            game.effect_select_opponent_permanent(
                player, on_return, filter_fn=target_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] 1 of your opponent's Digimon gets -1000 DP for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("P-043 DP -1000")
        effect1.set_effect_description("[On Deletion] 1 of your opponent's Digimon gets -1000 DP for the turn.")
        effect1.is_inherited_effect = True
        effect1.is_on_deletion = True
        effect1.dp_modifier = -1000

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP -1000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-1000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
