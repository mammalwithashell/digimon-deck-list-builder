from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_028(CardScript):
    """EX5-028 Kudamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If there're 6 or fewer total cards in both players' security stacks, you may play 1 yellow Tamer card from your hand without paying the cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX5-028 Play 1 yello Tamer from hand")
        effect0.set_effect_description("[On Play] If there're 6 or fewer total cards in both players' security stacks, you may play 1 yellow Tamer card from your hand without paying the cost.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = getattr(card, 'owner', None)
            if not owner or not owner.enemy:
                return False
            total_security = len(owner.security_cards) + len(owner.enemy.security_cards)
            return total_security <= 6

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                return 'Yellow' in [col.name for col in getattr(c, 'card_colors', [])]

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] [Once Per Turn] If there're 6 or fewer total cards in both players' security stacks, 1 of your opponent's Digimon gets -2000 DP for the turn.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("EX5-028 DP -2000")
        effect1.set_effect_description("[When Attacking] [Once Per Turn] If there're 6 or fewer total cards in both players' security stacks, 1 of your opponent's Digimon gets -2000 DP for the turn.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("DP-2000_EX5_028")
        effect1.is_on_attack = True
        effect1.dp_modifier = -2000

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP -2000"""
            player = ctx.get('player')
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-2000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
