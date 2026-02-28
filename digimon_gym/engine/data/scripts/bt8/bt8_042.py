from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_042(CardScript):
    """BT8-042 Shakkoumon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Jogress Condition
        effect0 = ICardEffect()
        effect0.set_effect_name("BT8-042 Jogress Condition")
        effect0.set_effect_description("Jogress Condition")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If you have 5 or fewer security cards, <Recovery +1 (Deck)>. (Place the top card of your deck on top of your security stack.) Then, when DNA digivolving, return 1 of your opponent's Digimon whose level is less than or equal to the number of cards in your security stack to its owner's hand.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT8-042 Recovery +1 (Deck) and return 1 Digimon to hand")
        effect1.set_effect_description("[When Digivolving] If you have 5 or fewer security cards, <Recovery +1 (Deck)>. (Place the top card of your deck on top of your security stack.) Then, when DNA digivolving, return 1 of your opponent's Digimon whose level is less than or equal to the number of cards in your security stack to its owner's hand.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Recovery +1, Bounce"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.recovery(1)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)
            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] 1 of your opponent's Digimon gets -3000 DP for the turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT8-042 DP -3000")
        effect2.set_effect_description("[When Attacking] 1 of your opponent's Digimon gets -3000 DP for the turn.")
        effect2.is_inherited_effect = True
        effect2.is_on_attack = True
        effect2.dp_modifier = -3000

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP -3000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-3000)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
