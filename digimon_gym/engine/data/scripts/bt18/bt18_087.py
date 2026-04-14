from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT18_087(CardScript):
    """BT18-087 Owen Dreadnought"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: set_memory_3
        # [Start of Your Turn] If you have 2 or less memory, set it to 3.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartTurn)
        effect0.set_effect_name("BT18-087 Set memory to 3")
        effect0.set_effect_description("[Start of Your Turn] If your memory is at 2 or less, it becomes 3.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Set memory to 3 if <= 2"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns] When a card is removed from your opponent's security stack,
        # by suspending this Tamer, delete 1 of your opponent's Digimon with 4000 DP or less.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnLoseSecurity)
        effect1.set_effect_name("BT18-087 Delete 1 opponent Digimon with 4000 DP or less")
        effect1.set_effect_description("[All Turns] When a card is removed from your opponent's security stack, by suspending this Tamer, delete 1 of your opponent's Digimon with 4000 DP or less.")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # [All Turns] Triggers when OPPONENT's security is removed
            event_player = context.get('event_player') or context.get('player')
            if event_player and card and card.owner:
                # The player who lost security must be the opponent
                if event_player is card.owner:
                    return False
            # Tamer must not already be suspended (suspend is cost)
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm and getattr(tamer_perm, 'is_suspended', False):
                return False
            # Must have at least 1 valid target (opponent Digimon with DP <= 4000)
            owner = card.owner
            if owner and owner.enemy:
                has_target = any(
                    p.is_digimon and p.dp is not None and p.dp <= 4000
                    for p in owner.enemy.battle_area
                )
                if not has_target:
                    return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend, Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return
            # Suspend this tamer as cost
            perm.suspend()
            # Delete 1 opponent Digimon with 4000 DP or less
            enemy = player.enemy if player else None
            if not enemy:
                return
            def target_filter(p):
                if not p.is_digimon:
                    return False
                if p.dp is not None and p.dp > 4000:
                    return False
                return True
            def on_delete(target_perm):
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # [Security] Play this card without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT18-087 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
