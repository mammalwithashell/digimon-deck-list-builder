from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_017(CardScript):
    """BT24-017 Medusamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: raid
        # Raid
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-017 Raid")
        effect0.set_effect_description("Raid")
        effect0.is_on_attack = True
        effect0._is_raid = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: progress
        # Progress
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-017 Progress")
        effect1.set_effect_description("Progress")
        effect1._is_progress = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: piercing
        # Piercing
        effect_p = ICardEffect()
        effect_p.set_effect_name("BT24-017 Piercing")
        effect_p.set_effect_description("Piercing")
        effect_p._is_piercing = True

        def condition_p(context: Dict[str, Any]) -> bool:
            return True
        effect_p.set_can_use_condition(condition_p)
        effects.append(effect_p)

        # [When Digivolving] DP +2000, Delete, Play Token
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-017 Delete lowest DP Digimon, Return 2 cards from their trash to deck to play 2 Tokens and gain 2k DP per opponent's Digimon.")
        effect2.set_effect_description("DP +2000, Delete, Play Token")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete, trash-to-deck cost + Tokens, then DP +2000 per opp Digimon"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # 1. Delete 1 of opponent's Digimon
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            # 2. By returning 2 cards from opponent's trash to deck bottom,
            #    play 2 Petrification Tokens on opponent's field
            enemy = player.enemy if player else None
            if enemy and len(enemy.trash_cards) >= 2:
                for _ in range(2):
                    if enemy.trash_cards:
                        card_to_return = enemy.trash_cards.pop(0)
                        enemy.deck_cards.append(card_to_return)
                game.effect_play_token(player, 'petrification', on_opponent_field=True, count=2)
            # 3. This Digimon gets +2000 DP for each of opponent's Digimon (AFTER tokens)
            if perm:
                enemy = player.enemy if player else None
                opp_digimon_count = 0
                if enemy:
                    opp_digimon_count = len([p for p in enemy.battle_area if p.is_digimon])
                if opp_digimon_count > 0:
                    perm.change_dp(2000 * opp_digimon_count)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
