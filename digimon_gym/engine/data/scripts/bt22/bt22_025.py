from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_025(CardScript):
    """BT22-025 UlforceVeedramon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-025 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blast_digivolve
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-025 Blast Digivolve")
        effect1.set_effect_description("Blast Digivolve")
        effect1.is_counter_effect = True
        effect1._is_blast_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Shared process for On Play / When Digivolving:
        # Choose 1: Return lowest level opp Digimon to deck bottom, OR play blue Tamer (cost<=4) free
        def _shared_choice_process(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None

            def on_choice(choice: int):
                if choice == 0:
                    # Branch 0: Return 1 of opponent's lowest level Digimon to deck bottom
                    if not enemy:
                        return
                    opp_digimon = [p for p in enemy.battle_area if p.is_digimon and p.top_card]
                    if not opp_digimon:
                        return
                    min_level = min(getattr(p.top_card, 'level', 99) or 99 for p in opp_digimon)
                    def lowest_filter(p):
                        return p.is_digimon and p.top_card and (getattr(p.top_card, 'level', 99) or 99) == min_level
                    def on_bottom_deck(target_perm):
                        if enemy:
                            enemy.return_permanent_to_deck_bottom(target_perm)
                    game.effect_select_opponent_permanent(
                        player, on_bottom_deck, filter_fn=lowest_filter, is_optional=False)
                else:
                    # Branch 1: Play 1 blue Tamer (cost<=4) from hand free
                    from ....data.enums import CardColor
                    def blue_tamer_filter(c):
                        if not getattr(c, 'is_tamer', False):
                            return False
                        colors = getattr(c, 'card_colors', []) or []
                        if CardColor.Blue not in colors:
                            return False
                        cost = c.get_cost_itself
                        return cost <= 4
                    game.effect_play_from_zone(
                        player, 'hand', blue_tamer_filter, free=True, is_optional=True)

            game.effect_choose_branch(
                player, 2, on_choice,
                branch_labels=["Return lowest level Digimon to deck bottom",
                               "Play blue Tamer (cost 4 or less) from hand"])

        # [On Play] Choose 1 effect
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT22-025 Bottom deck lowest level or play blue tamer")
        effect2.set_effect_description("[On Play] Activate 1 of the effects below.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_shared_choice_process)
        effects.append(effect2)

        # [When Digivolving] Choose 1 effect
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT22-025 Bottom deck lowest level or play blue tamer")
        effect3.set_effect_description("[When Digivolving] Activate 1 of the effects below.")
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_shared_choice_process)
        effects.append(effect3)

        # [When Attacking] [Once Per Turn] This Digimon may unsuspend.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUseAttack)
        effect4.set_effect_name("BT22-025 Unsuspend this digimon")
        effect4.set_effect_description("[When Attacking] [Once Per Turn] This Digimon may unsuspend.")
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT22_025_Unsuspend")
        effect4.is_on_attack = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Unsuspend this Digimon."""
            perm = card.permanent_of_this_card() if card else None
            if perm:
                perm.unsuspend()

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Inherited: Ace Overflow <-4>
        effect_ace = ICardEffect()
        effect_ace.set_effect_name("BT22-025 Ace Overflow <-4>")
        effect_ace.set_effect_description("Ace Overflow <-4>")
        effect_ace.is_inherited_effect = True
        def condition_ace(context: Dict[str, Any]) -> bool:
            return True
        effect_ace.set_can_use_condition(condition_ace)
        effects.append(effect_ace)

        return effects
