from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_036(CardScript):
    """BT13-036 Liollmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ---------------------------------------------------------------
        # [Your Turn][Once Per Turn] When a card is removed from your
        # security stack, gain 1 memory.
        # ---------------------------------------------------------------
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnLoseSecurity)
        effect0.set_effect_name("BT13-036 Memory +1")
        effect0.set_effect_description("[Your Turn][Once Per Turn] When a card is removed from your security stack, gain 1 memory.")
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Memory+1_BT13_036")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            if player:
                player.add_memory(1)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ---------------------------------------------------------------
        # ESS [When Attacking][Once Per Turn]
        # If there are 6 or fewer total cards in both players' security
        # stacks, 1 of your opponent's Digimon gets -2000 DP for the turn.
        # Uses player choice (effect_select_opponent_permanent), NOT auto-select.
        # ---------------------------------------------------------------
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("BT13-036 DP -2000")
        effect1.set_effect_description("[When Attacking][Once per Turn] If there are 6 or fewer total cards in both players' security stacks, 1 of your opponent's Digimon gets -2000 DP for the turn.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("DP-2000_BT13_036")
        effect1.is_on_attack = True
        effect1.dp_modifier = -2000

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Check total security cards <= 6
            owner = card.owner if card else None
            if owner:
                enemy = owner.enemy if owner else None
                own_sec = len(owner.security_cards) if owner.security_cards else 0
                opp_sec = len(enemy.security_cards) if enemy and enemy.security_cards else 0
                if own_sec + opp_sec > 6:
                    return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Select 1 opponent Digimon, apply DP -2000 for turn."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....interfaces.modifiers import ModifierType

            def opp_digi_filter(p):
                return p.is_digimon

            def on_selected(target_perm):
                game.register_modifier(
                    target_perm,
                    ModifierType.CHANGE_DP,
                    value_fn=lambda: -2000,
                    expiry='end_of_turn',
                )

            game.effect_select_opponent_permanent(
                player, on_selected, filter_fn=opp_digi_filter, is_optional=False,
                prompt="Select 1 of your opponent's Digimon to give -2000 DP.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
