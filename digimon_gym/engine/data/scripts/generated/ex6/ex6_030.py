from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_030(CardScript):
    """EX6-030 Dominimon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Search your security stack. You may play 1 level 5 or lower Digimon card with the [Angel]/[Archangel] trait among them without paying the cost. Then, shuffle your security stack, and 1 of your opponent's Digimon gets -7000 DP until the end of the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-030 Play 1 level 5 or lower Digimon from security, then 1 of your opponent's Digimon gets -7000 DP until the end of the turn")
        effect0.set_effect_description("[When Digivolving] Search your security stack. You may play 1 level 5 or lower Digimon card with the [Angel]/[Archangel] trait among them without paying the cost. Then, shuffle your security stack, and 1 of your opponent's Digimon gets -7000 DP until the end of the turn.")
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: DP -7000, Play Card, Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-7000)
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 5:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When one of your Digimon with the [Angel]/[Archangel]/[Three Great Angels] trait would leave the battle area other than in battle, by trashing the top card of your security stack, prevent it from leaving.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-030 By trashing the top card of your security stack, prevent one Digimon from leaving the battle area")
        effect1.set_effect_description("[All Turns] When one of your Digimon with the [Angel]/[Archangel]/[Three Great Angels] trait would leave the battle area other than in battle, by trashing the top card of your security stack, prevent it from leaving.")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
