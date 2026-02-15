from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_060(CardScript):
    """BT20-060 Alphamon: Ouryuken | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Jogress Condition
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-060 Jogress Condition")
        effect0.set_effect_description("Jogress Condition")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Jogress Condition"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DNA/Jogress digivolution condition — handled by engine
            pass

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your opponent's Digimon gets -15000 DP until the end of their turn. Then, if DNA digivolving, trash your opponent's top security card and <Recovery +1 (Deck)>.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-060 One of your opponent's Digimon gets -15000 DP")
        effect1.set_effect_description("[On Play] 1 of your opponent's Digimon gets -15000 DP until the end of their turn. Then, if DNA digivolving, trash your opponent's top security card and <Recovery +1 (Deck)>.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP -15000, Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-15000)
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop()
                        enemy.trash_cards.append(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your opponent's Digimon gets -15000 DP until the end of their turn. Then, if DNA digivolving, trash your opponent's top security card and <Recovery +1 (Deck)>.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-060 One of your opponent's Digimon gets -15000 DP")
        effect2.set_effect_description("[When Digivolving] 1 of your opponent's Digimon gets -15000 DP until the end of their turn. Then, if DNA digivolving, trash your opponent's top security card and <Recovery +1 (Deck)>.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP -15000, Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-15000)
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop()
                        enemy.trash_cards.append(trashed)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns] (Once Per Turn) When security stacks are removed from, When security stacks are removed from, gain 3 memory.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-060 Gain 3 memory")
        effect3.set_effect_description("[All Turns] (Once Per Turn) When security stacks are removed from, When security stacks are removed from, gain 3 memory.")
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("RemovedSec_BT20_060")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Gain 3 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(3)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
