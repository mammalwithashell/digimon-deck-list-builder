from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_017(CardScript):
    """BT24-017 Medusamon | Lv.6

    <Raid> <Progress> <Piercing>
    [When Digivolving] Delete 1 of your opponent's lowest DP Digimon.
    Then, by returning 2 cards from their trash to the bottom of the deck,
    they play 2 [Petrification] Tokens. After, this Digimon gets +2000 DP
    for each of your opponent's Digimon until their turn ends.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: raid
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
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-017 Progress")
        effect1.set_effect_description("Progress")
        effect1._is_progress = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: piercing
        effect_p = ICardEffect()
        effect_p.set_effect_name("BT24-017 Piercing")
        effect_p.set_effect_description("Piercing")
        effect_p._is_piercing = True

        def condition_p(context: Dict[str, Any]) -> bool:
            return True
        effect_p.set_can_use_condition(condition_p)
        effects.append(effect_p)

        # [When Digivolving] Delete lowest DP, return 2 trash to deck -> 2 tokens, then DP boost
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT24-017 Delete lowest DP, tokens, DP boost")
        effect2.set_effect_description("[When Digivolving] Delete lowest DP Digimon, play tokens, gain DP.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Delete lowest DP, return 2 trash cards, play 2 tokens, then DP boost."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # 1. Delete 1 of opponent's lowest DP Digimon
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon and p.top_card]
            if opp_digimon:
                min_dp = min((p.dp or 0) for p in opp_digimon)
                def lowest_filter(p):
                    return p.is_digimon and (p.dp or 0) == min_dp
                def on_delete(target_perm):
                    enemy.delete_permanent(target_perm)
                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=lowest_filter, is_optional=False)

            # 2. By returning 2 cards from their trash to deck bottom, play 2 Petrification Tokens
            if len(enemy.trash_cards) >= 2:
                for _ in range(2):
                    if enemy.trash_cards:
                        returned = enemy.trash_cards.pop(0)
                        enemy.library_cards.append(returned)
                game.effect_play_token(player, 'petrification', on_opponent_field=True, count=2)

            # 3. This Digimon gets +2000 DP for each of opponent's Digimon until their turn ends
            if perm:
                opp_count = len([p for p in enemy.battle_area if p.is_digimon])
                if opp_count > 0:
                    perm.change_dp(2000 * opp_count)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
