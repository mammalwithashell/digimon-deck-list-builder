from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class LM_027(CardScript):
    """LM-027 Red Scramble"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] 1 of your red Digimon may digivolve into a red Digimon card
        # in the hand with the digivolution cost reduced by 3.
        # Then, place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("LM-027 Digivolve 1 red Digimon with cost reduced by 3")
        effect0.set_effect_description("[Main] 1 of your red Digimon may digivolve into a red Digimon card in the hand with the digivolution cost reduced by 3. Then, place this card in the battle area.")
        effect0.cost_reduction = 3

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # Select one of your red Digimon to digivolve
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('Red' == col.name for col in getattr(c, 'card_colors', []))):
                    return False
                return True
            # Find a red Digimon on the field to digivolve
            def own_filter(p):
                if not p.is_digimon:
                    return False
                top = p.top_card
                if top and hasattr(top, 'card_colors'):
                    if not any(col.name == 'Red' for col in top.card_colors):
                        return False
                return True
            def on_select(target_perm):
                if game:
                    game.effect_digivolve_from_hand(
                        player, target_perm, digi_filter, cost_reduction=3, is_optional=True)
            game.effect_select_own_permanent(
                player, on_select, filter_fn=own_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        # <Delay> — place in battle area, trash next turn for delayed effect
        effect1 = ICardEffect()
        effect1.set_effect_name("LM-027 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            # [Start of Your Turn] If your opponent has a Digimon
            enemy = card.owner.enemy if card and card.owner else None
            if not enemy:
                return False
            has_opp_digimon = any(
                p.is_digimon for p in (enemy.battle_area or [])
            )
            if not has_opp_digimon:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnStartMainPhase
        # Delay effect: Return 1 red Digimon card from your trash to the top
        # of the deck. Then, if you don't have a Digimon, you may play 1 red
        # Digimon card with 2000 DP or less from your trash without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnStartMainPhase)
        effect2.set_effect_name("LM-027 Return 1 red Digimon from trash to deck top, then play from trash")
        effect2.set_effect_description("Return 1 red Digimon card from your trash to the top of the deck. Then, if you don't have a Digimon, you may play 1 red Digimon card with 2000 DP or less from your trash without paying the cost.")
        effect2.is_optional = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Return 1 red Digimon from trash to deck top, then play from trash."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # Return 1 red Digimon card from your trash to the top of the deck (player selects)
            from ....game.constants import SEL_TRASH_START as _SEL_TRASH
            from ....data.enums import GamePhase

            def red_digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = getattr(c, 'card_colors', [])
                return any(col.name == 'Red' for col in colors)

            valid_trash = [_SEL_TRASH + i for i, c in enumerate(player.trash_cards) if red_digi_filter(c)]
            if not valid_trash:
                return

            def on_trash_selected(action_id):
                idx = action_id - _SEL_TRASH
                if 0 <= idx < len(player.trash_cards):
                    returned = player.trash_cards.pop(idx)
                    player.library_cards.insert(0, returned)
                # Then, if you don't have a Digimon, play 1 red Digimon
                # with 2000 DP or less from trash without paying cost
                has_digimon = any(
                    p.is_digimon for p in (player.battle_area or [])
                )
                if not has_digimon:
                    def play_filter(c):
                        if not getattr(c, 'is_digimon', False):
                            return False
                        if not (any(col.name == 'Red' for col in getattr(c, 'card_colors', []))):
                            return False
                        if getattr(c, 'dp', None) is not None and c.dp > 2000:
                            return False
                        return True
                    game.effect_play_from_zone(
                        player, 'trash', play_filter, free=True, is_optional=True)

            game.request_selection(
                GamePhase.SelectTarget, player, on_trash_selected,
                valid_trash, is_optional=False,
                prompt="Select 1 red Digimon card from your trash to return to deck top.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
