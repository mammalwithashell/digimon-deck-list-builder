from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_037(CardScript):
    """BT22-037 Chirinmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-037 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.4 for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDiscardSecurity
        # When effects trash this card from the security stack, 1 of your opponent's Digimon gets -8000 DP for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-037 -8K DP")
        effect1.set_effect_description("When effects trash this card from the security stack, 1 of your opponent's Digimon gets -8000 DP for the turn.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By trashing your top security card, this Digimon may digivolve into a Digimon card with [Kentaurosmon] or [Mitamamon] in its name or the [CS] trait in the hand with the digivolution cost reduced by 2.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-037 By trashing top security, digivolve into card with [Kentaurosmon]/[Mitamamon] in name or [CS] trait ")
        effect2.set_effect_description("[When Digivolving] By trashing your top security card, this Digimon may digivolve into a Digimon card with [Kentaurosmon] or [Mitamamon] in its name or the [CS] trait in the hand with the digivolution cost reduced by 2.")
        effect2.is_optional = True
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Digivolve, Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not (any('Kentaurosmon' in _n or 'Mitamamon' in _n for _n in getattr(c, 'card_names', [])) or any('CS' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -4000 DP for the turn.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-037 -4K DP")
        effect3.set_effect_description("[When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -4000 DP for the turn.")
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT22_037_WA")
        effect3.is_on_attack = True
        effect3.dp_modifier = -4000

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: DP -4000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-4000)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
