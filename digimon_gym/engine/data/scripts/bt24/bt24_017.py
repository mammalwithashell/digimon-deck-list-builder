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

            # 2. "By returning 2 cards from their trash to the bottom of the deck,
            #    they play 2 [Petrification] Tokens."
            # "By returning" = cost. Player chooses which 2 cards from opponent's trash.
            # Per C# canNoSelect: () => true, the player CAN decline.
            # If declined or not enough cards, tokens + DP boost are SKIPPED.
            from ....game.constants import SEL_TRASH_START as _SEL_TRASH
            from ....data.enums import GamePhase

            returned_count = [0]

            def _after_trash_return():
                """Called after trash return attempt. Tokens + DP only if 2 cards returned."""
                if returned_count[0] < 2:
                    return
                # Play 2 Petrification Tokens on opponent's field
                game.effect_play_token(player, 'petrification', on_opponent_field=True, count=2)
                # DP boost: +2000 per opponent Digimon until their turn ends
                this_perm = card.permanent_of_this_card() if card else None
                if this_perm:
                    from ....interfaces.modifiers import ModifierType
                    def dp_value_fn():
                        e = player.enemy if player else None
                        if not e:
                            return 0
                        opp_count = len([p for p in e.battle_area if p.is_digimon])
                        return 2000 * opp_count
                    game.register_modifier(
                        this_perm, ModifierType.CHANGE_DP,
                        value_fn=dp_value_fn, expiry='end_of_opponent_turn')

            def _select_trash_card(remaining):
                """Select next card from opponent's trash to return to deck bottom."""
                if remaining <= 0:
                    _after_trash_return()
                    return
                valid_trash = [_SEL_TRASH + i for i in range(len(enemy.trash_cards))]
                if not valid_trash:
                    # Not enough cards in trash -- cost can't be paid fully
                    _after_trash_return()
                    return

                def on_selected(action_id):
                    idx = action_id - _SEL_TRASH
                    if 0 <= idx < len(enemy.trash_cards):
                        chosen = enemy.trash_cards[idx]
                        enemy.trash_cards.remove(chosen)
                        enemy.library_cards.append(chosen)
                        returned_count[0] += 1
                    _select_trash_card(remaining - 1)

                def on_decline():
                    # Player declined -- cost not paid, skip tokens + DP
                    _after_trash_return()

                game.request_selection(
                    GamePhase.SelectTarget, player, on_selected,
                    valid_trash, is_optional=True,
                    prompt=f"Select a card from opponent's trash to return to deck bottom ({remaining} remaining).",
                    on_decline=on_decline)

            if len(enemy.trash_cards) >= 2:
                _select_trash_card(2)
            else:
                # Not enough trash cards -- cost can't be paid, skip tokens + DP boost
                pass

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
