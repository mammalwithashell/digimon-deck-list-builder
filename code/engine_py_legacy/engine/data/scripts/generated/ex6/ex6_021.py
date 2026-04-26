from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_021(CardScript):
    """EX6-021 ArkhaiAngemon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By adding the top or bottom card of your security stack to the hand, 1 of your opponent's Digimon gets -4000 DP for the turn. Then, you may place 1 Digimon card with the [Angel]/[Archangel]/[Three Great Angels] trait from your hand at the bottom of your security stack.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-021 Add 1 card from top or bottom security to hand to -4000 DP an opponent's Digimon, then place 1 card from hand at the bottom of security")
        effect0.set_effect_description("[On Play] By adding the top or bottom card of your security stack to the hand, 1 of your opponent's Digimon gets -4000 DP for the turn. Then, you may place 1 Digimon card with the [Angel]/[Archangel]/[Three Great Angels] trait from your hand at the bottom of your security stack.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: DP -4000, Add To Hand, Add To Security, Destroy Security"""
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
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            # Add top card of deck to security
            if player:
                player.recovery(1)
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By adding the top or bottom card of your security stack to the hand, 1 of your opponent's Digimon gets -4000 DP for the turn. Then, you may place 1 Digimon card with the [Angel]/[Archangel]/[Three Great Angels] trait from your hand at the bottom of your security stack.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-021 Add 1 card from top or bottom security to hand to -4000 DP an opponent's Digimon, then place 1 card from hand at the bottom of security")
        effect1.set_effect_description("[When Digivolving] By adding the top or bottom card of your security stack to the hand, 1 of your opponent's Digimon gets -4000 DP for the turn. Then, you may place 1 Digimon card with the [Angel]/[Archangel]/[Three Great Angels] trait from your hand at the bottom of your security stack.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP -4000, Add To Hand, Add To Security, Destroy Security"""
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
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            # Add top card of deck to security
            if player:
                player.recovery(1)
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("EX6-021 Blocker")
        effect2.set_effect_description("Blocker")
        effect2.is_inherited_effect = True
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
