from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_084(CardScript):
    """BT8-084 Kimeramon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Jogress Condition
        effect0 = ICardEffect()
        effect0.set_effect_name("BT8-084 Jogress Condition")
        effect0.set_effect_description("Jogress Condition")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may place 1 level 5 or lower Digimon card from your trash under this Digimon as its bottom digivolution card. Then, up to 4 of your opponent's Digimon get -1000 DP for each of this Digimon's colors until the end of your opponent's next turn.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT8-084 Place a Card to digivolution cards from trash and DP -")
        effect1.set_effect_description("[When Digivolving] You may place 1 level 5 or lower Digimon card from your trash under this Digimon as its bottom digivolution card. Then, up to 4 of your opponent's Digimon get -1000 DP for each of this Digimon's colors until the end of your opponent's next turn.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Place Lv.5- Digimon from trash as bottom evo card, then -1000 DP per color to up to 4 opp Digimon"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def _apply_dp_reduction():
                """Apply -1000 DP per color to up to 4 opponent Digimon."""
                enemy = player.enemy if player else None
                if not enemy:
                    return
                # Count unique colors across all card sources
                all_colors = set()
                for cs in perm.card_sources:
                    if cs.c_entity_base:
                        all_colors.update(cs.c_entity_base.card_colors)
                color_count = len(all_colors)
                if color_count == 0:
                    return
                dp_reduction = -1000 * color_count
                # Select up to 4 opponent Digimon to apply reduction
                targets_remaining = [4]
                def target_filter(p):
                    return p.is_digimon
                def on_target_selected(target_perm):
                    target_perm.change_dp(dp_reduction)
                    targets_remaining[0] -= 1
                    if targets_remaining[0] > 0:
                        opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
                        if opp_digimon:
                            game.effect_select_opponent_permanent(
                                player, on_target_selected, filter_fn=target_filter, is_optional=True)
                game.effect_select_opponent_permanent(
                    player, on_target_selected, filter_fn=target_filter, is_optional=True)

            # Step 1: Optionally place 1 Lv.5- Digimon from trash as bottom evo card
            eligible = [tc for tc in player.trash_cards
                        if getattr(tc, 'is_digimon', False)
                        and (getattr(tc, 'level', 99) or 99) <= 5]
            if eligible:
                chosen = eligible[0]  # Pick first eligible
                player.trash_cards.remove(chosen)
                perm.add_card_source_bottom(chosen)
            # Step 2: Apply DP reduction
            _apply_dp_reduction()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: dp_modifier
        # [Your Turn] This Digimon gets +1000 DP for each of its colors.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT8-084 DP modifier per color")
        effect2.set_effect_description("DP modifier per color")
        effect2.dp_modifier = 2000  # default; dynamically updated in condition

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            # Dynamically set DP modifier based on number of unique colors
            all_colors = set()
            for cs in perm.card_sources:
                if cs.c_entity_base:
                    all_colors.update(cs.c_entity_base.card_colors)
            effect2.dp_modifier = 1000 * max(1, len(all_colors))
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
