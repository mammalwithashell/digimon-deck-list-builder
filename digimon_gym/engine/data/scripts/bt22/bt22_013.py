from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_013(CardScript):
    """BT22-013 WarGreymon | Lv.6

    Alt digivolve: from Lv.5 w/ [Greymon] name or CS trait for 3.
    [Hand][Main] If you have [Nokia Shiramine], 1 of your [Agumon] digivolves
        into this card for a digivolution cost of 6, ignoring digivolution
        requirements.
    [When Digivolving] Activate 1 of the effects below:
        - 1 of your [Gabumon] may digivolve into [MetalGarurumon] in the hand,
          ignoring digivolution requirements and without paying the cost.
        - Delete 1 of your opponent's Digimon with the lowest DP.
    Inherited: [When Attacking][Once Per Turn] If this Digimon has [Omnimon]
        in its name, trash your opponent's top security card.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from Lv.5 w/ Greymon or CS trait for 3 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-013 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [Hand][Main] Nokia warp digivolve Agumon into this for 6 ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDeclaration)
        effect1._is_hand_main = True
        effect1.set_effect_name("BT22-013 Digivolve for a cost of 6")
        effect1.set_effect_description(
            "[Hand][Main] If you have [Nokia Shiramine], 1 of your [Agumon] "
            "digivolves into this card for a digivolution cost of 6, ignoring "
            "digivolution requirements."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            # Card must be in hand
            if not card or card.permanent_of_this_card() is not None:
                return False
            player = card.owner
            if not player or not player.is_my_turn:
                return False
            # Must have Nokia Shiramine on field
            has_nokia = any(
                p.is_tamer and p.contains_card_name('Nokia Shiramine')
                for p in player.battle_area
            )
            if not has_nokia:
                return False
            # Must have Agumon on field
            has_agumon = any(
                p.is_digimon and p.contains_card_name('Agumon')
                for p in player.battle_area
            )
            return has_agumon

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Select an Agumon and digivolve into this card for cost 6."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def agumon_filter(p):
                return p.is_digimon and p.contains_card_name('Agumon')

            def hand_filter(c):
                return c is card

            def on_select_agumon(target_perm):
                game.effect_digivolve_from_hand(
                    player, target_perm, hand_filter,
                    cost_override=6, ignore_requirements=True,
                    is_optional=True)

            game.effect_select_own_permanent(
                player, on_select_agumon,
                filter_fn=agumon_filter,
                is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [When Digivolving] Choose 1 branch ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT22-013 Choose 1 effect")
        effect2.set_effect_description(
            "[When Digivolving] Activate 1 of the effects below:\r\n"
            "- 1 of your [Gabumon] may digivolve into [MetalGarurumon] in the "
            "hand, ignoring digivolution requirements and without paying the "
            "cost.\r\n"
            "- Delete 1 of your opponent's Digimon with the lowest DP."
        )
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Branch: digivolve Gabumon into MetalGarurumon OR delete lowest DP."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_branch(choice: int):
                if choice == 0:
                    # Digivolve 1 of your [Gabumon] into [MetalGarurumon]
                    def gabumon_filter(p):
                        return p.is_digimon and p.contains_card_name('Gabumon')

                    def mg_hand_filter(c):
                        if not getattr(c, 'is_digimon', False):
                            return False
                        return c.contains_card_name('MetalGarurumon')

                    def on_select_gabumon(target_perm):
                        game.effect_digivolve_from_hand(
                            player, target_perm, mg_hand_filter,
                            cost_override=0, ignore_requirements=True,
                            is_optional=True)

                    has_gabumon = any(gabumon_filter(p) for p in player.battle_area)
                    has_mg = any(mg_hand_filter(c) for c in player.hand_cards)
                    if has_gabumon and has_mg:
                        game.effect_select_own_permanent(
                            player, on_select_gabumon,
                            filter_fn=gabumon_filter,
                            is_optional=True)
                else:
                    # Delete 1 opponent Digimon with lowest DP
                    enemy = player.enemy
                    if not enemy:
                        return
                    enemy_digimon = [p for p in enemy.battle_area if p.is_digimon]
                    if not enemy_digimon:
                        return
                    min_dp = min(p.dp for p in enemy_digimon if p.dp is not None)

                    def lowest_dp_filter(p):
                        return p.is_digimon and p.dp is not None and p.dp == min_dp

                    def on_delete(target_perm):
                        enemy.delete_permanent(target_perm)

                    game.effect_select_opponent_permanent(
                        player, on_delete,
                        filter_fn=lowest_dp_filter,
                        is_optional=False)

            game.effect_choose_branch(
                player, 2,
                callback=on_branch,
                branch_labels=[
                    "1 of your [Gabumon] digivolves into [MetalGarurumon]",
                    "Delete 1 opponent Digimon with lowest DP"
                ])

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: Inherited [When Attacking][OPT] Trash security if Omnimon ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUseAttack)
        effect3.set_effect_name("BT22-013 Trash the top card of opponent's security")
        effect3.set_effect_description(
            "[When Attacking] [Once Per Turn] If this Digimon has [Omnimon] "
            "in its name, trash your opponent's top security card."
        )
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("TrashSecurity_BT22_013")
        effect3.is_on_attack = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if not (perm and perm.top_card and perm.top_card.contains_card_name('Omnimon')):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Trash opponent's top security card."""
            player = ctx.get('player')
            enemy = player.enemy if player else None
            if enemy and enemy.security_cards:
                trashed = enemy.security_cards.pop(0)
                enemy.trash_cards.append(trashed)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
