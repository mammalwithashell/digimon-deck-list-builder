from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_096(CardScript):
    """BT10-096 Burning Star Crusher"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Choose 1 of your level 4 or higher Digimon with [Shoutmon] in its name. Delete 1 of your opponent's Digimon with DP less than or equal to the chosen Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-096 Delete")
        effect0.set_effect_description("[Main] Choose 1 of your level 4 or higher Digimon with [Shoutmon] in its name. Delete 1 of your opponent's Digimon with DP less than or equal to the chosen Digimon.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level < 4:
                    return False
                if not (p.contains_card_name('Shoutmon')):
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Reveal the top 3 cards of your deck. You may add 1 Digimon card with [Xros Heart] in its traits among them to your hand and play 1 [Taiki Kudo] among them without paying its memory cost. Place the rest at the bottom of your deck in any order.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT10-096 Play Card, Add To Hand")
        effect1.set_effect_description("[Security] Reveal the top 3 cards of your deck. You may add 1 Digimon card with [Xros Heart] in its traits among them to your hand and play 1 [Taiki Kudo] among them without paying its memory cost. Place the rest at the bottom of your deck in any order.")
        effect1.is_security_effect = True
        effect1.is_security_effect = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card, Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not (any('Xros Heart' in _t or 'XrosHeart' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
