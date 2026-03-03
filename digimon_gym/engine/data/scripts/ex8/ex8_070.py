from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_070(CardScript):
    """EX8-070 Zofr Kabus"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name(
            "EX8-070 By trashing 1 source, gain collision, piercing, reboot, +3000 DP, and can't be returned to hand or deck by opponent"
        )
        effect0.set_effect_description(
            "[Main] By trashing any 1 digivolution card of 1 of your [Mineral] or [Rock] trait Digimon, until the end of your opponent's turn, it gains <Collision>, <Piercing> and <Reboot>, gets +3000 DP, and can't be returned to the hand or deck by your opponent's effects."
        )
        effect0._is_collision = True
        effect0._is_piercing = True
        effect0._is_reboot = True
        effect0._is_cannot_return_to_hand = True
        effect0._is_cannot_return_to_deck = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            expiry_turn = game.turn_count + 1

            def target_filter(p):
                if not p.is_digimon:
                    return False
                if not (p.has_trait('Mineral') or p.has_trait('Rock')):
                    return False
                return not p.has_no_digivolution_cards

            def on_target(target_perm):
                trashed = target_perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
                target_perm.change_dp(3000)
                target_perm.grant_keyword('_is_collision', expiry_turn)
                target_perm.grant_keyword('_is_piercing', expiry_turn)
                target_perm.grant_keyword('_is_reboot', expiry_turn)
                target_perm.grant_keyword('_is_cannot_return_to_hand', expiry_turn)
                target_perm.grant_keyword('_is_cannot_return_to_deck', expiry_turn)

            game.effect_select_own_permanent(
                player, on_target, filter_fn=target_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("EX8-070 Delete")
        effect1.set_effect_description("[Security] Delete 1 of your opponent's Digimon with the lowest play cost.")
        effect1.is_security_effect = True
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            digimon = [p for p in enemy.battle_area if p.is_digimon]
            if not digimon:
                return
            min_cost = min(getattr(p.top_card, 'get_cost_itself', 0) for p in digimon)

            def target_filter(p):
                return p.is_digimon and getattr(p.top_card, 'get_cost_itself', 0) == min_cost

            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
