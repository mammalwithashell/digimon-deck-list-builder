from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST9_05(CardScript):
    """ST9-05 Paildramon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] When DNA digivolving, return 1 of your opponent's
        # Digimon with 6000 DP or less to the bottom of its owner's deck.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("ST9-05 Return opp Digimon <=6000 DP to deck bottom")
        effect0.set_effect_description("[When Digivolving] When DNA digivolving, return 1 of your opponent's Digimon with 6000 DP or less to the bottom of its owner's deck.")
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not context.get('is_dna_digivolve'):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Return opp Digimon <=6000 DP to deck bottom"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy

            def target_filter(p):
                return p.is_digimon and p.dp is not None and p.dp <= 6000

            def on_select(target_perm):
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)

            game.effect_select_opponent_permanent(
                player, on_select, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] [Once Per Turn] Unsuspend this Digimon.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("ST9-05 Unsuspend this Digimon")
        effect1.set_effect_description("[When Attacking] [Once Per Turn] Unsuspend this Digimon.")
        effect1.is_on_attack = True
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Unsuspend_ST9_05")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Unsuspend this Digimon"""
            perm = ctx.get('permanent')
            if perm and perm.is_suspended:
                perm.unsuspend()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
