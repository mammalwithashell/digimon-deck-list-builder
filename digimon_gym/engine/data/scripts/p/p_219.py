from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_219(CardScript):
    """P-219 Flame Inferno"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.BeforePayCost
        # When this card would be used, if your opponent has 10 or more cards in their trash, reduce the use cost by 3
        effect0 = ICardEffect()
        effect0.set_effect_name("P-219 Reduce Use Cost -3")
        effect0.set_effect_description("When this card would be used, if your opponent has 10 or more cards in their trash, reduce the use cost by 3")
        effect0.cost_reduction = 3

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Cost -3"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 3 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] Delete 1 of your opponent's level 6 or lower Digimon. Then, by deleting 1 of your [Evil] or [Fallen Angel] trait Digimon, you may play 1 [Creepymon] from your trash without paying the cost. The Digimon this effect played gains <Rush> and <Blocker> until your opponent's turn ends.
        effect1 = ICardEffect()
        effect1.set_effect_name("P-219 Delete opponent's Digimon. Delete 1 of your own to play [Creepymon] from Trash")
        effect1.set_effect_description("[Main] Delete 1 of your opponent's level 6 or lower Digimon. Then, by deleting 1 of your [Evil] or [Fallen Angel] trait Digimon, you may play 1 [Creepymon] from your trash without paying the cost. The Digimon this effect played gains <Rush> and <Blocker> until your opponent's turn ends.")
        effect1._is_rush = True
        effect1._is_blocker = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete, Play Card, Gain Keyword Rush, Gain Keyword Blocker"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 6:
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            if not (player and game):
                return
            def play_filter(c):
                if not (any('Creepymon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if perm:
                perm.grant_keyword('_is_rush')
                perm.grant_keyword('_is_blocker')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("P-219 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
