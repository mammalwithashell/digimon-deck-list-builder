from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_026(CardScript):
    """BT22-026 MetalGarurumon | Lv.6

    Alt digivolve: from Lv.5 w/ [Garurumon] name or CS trait for 3.
    [Hand][Main] If you have [Nokia Shiramine], 1 of your [Gabumon] digivolves
        into this card for a digivolution cost of 6, ignoring digivolution
        requirements.
    [When Digivolving] Activate 1 of the effects below:
        - 1 of your [Agumon] may digivolve into [WarGreymon] in the hand,
          ignoring digivolution requirements and without paying the cost.
        - Return 1 of your opponent's Digimon with the lowest level to the hand.
    Inherited: [When Attacking][Once Per Turn] If this Digimon has [Omnimon]
        in its name, it unsuspends.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from Lv.5 w/ Garurumon or CS trait for 3 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-026 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [Hand][Main] Nokia warp digivolve Gabumon into this for 6 ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDeclaration)
        effect1.set_effect_name("BT22-026 Digivolve 1 [Gabumon] into this card for 6")
        effect1.set_effect_description(
            "[Hand] [Main] If you have [Nokia Shiramine], 1 of your [Gabumon] "
            "digivolves into this card for a digivolution cost of 6, ignoring "
            "digivolution requirements."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            if not card or card.permanent_of_this_card() is not None:
                return False
            player = card.owner
            if not player or not player.is_my_turn:
                return False
            has_nokia = any(
                p.is_tamer and p.top_card and
                any('Nokia Shiramine' == n for n in (p.top_card.card_names or []))
                for p in player.battle_area
            )
            if not has_nokia:
                return False
            has_gabumon = any(
                p.is_digimon and p.top_card and
                any('Gabumon' == n for n in (p.top_card.card_names or []))
                for p in player.battle_area
            )
            return has_gabumon

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Select a Gabumon and digivolve into this card for cost 6."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def gabumon_filter(p):
                return (p.is_digimon and p.top_card and
                        any('Gabumon' == n for n in (p.top_card.card_names or [])))

            def hand_filter(c):
                return c is card

            def on_select_gabumon(target_perm):
                game.effect_digivolve_from_hand(
                    player, target_perm, hand_filter,
                    cost_override=6, ignore_requirements=True,
                    is_optional=True)

            game.effect_select_own_permanent(
                player, on_select_gabumon,
                filter_fn=gabumon_filter,
                is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [When Digivolving] Choose 1 branch ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT22-026 Choose 1 effect")
        effect2.set_effect_description(
            "[When Digivolving] Activate 1 of the effects below:\r\n"
            "- 1 of your [Agumon] may digivolve into [WarGreymon] in the hand, "
            "ignoring digivolution requirements and without paying the cost.\r\n"
            "- Return 1 of your opponent's Digimon with the lowest level to the hand."
        )
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Branch: digivolve Agumon into WarGreymon OR bounce lowest level."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_branch(choice: int):
                if choice == 0:
                    # Digivolve 1 of your [Agumon] into [WarGreymon]
                    def agumon_filter(p):
                        return (p.is_digimon and p.top_card and
                                any('Agumon' == n for n in (p.top_card.card_names or [])))

                    def wg_hand_filter(c):
                        if not getattr(c, 'is_digimon', False):
                            return False
                        return any('WarGreymon' == n for n in (getattr(c, 'card_names', []) or []))

                    def on_select_agumon(target_perm):
                        game.effect_digivolve_from_hand(
                            player, target_perm, wg_hand_filter,
                            cost_override=0, ignore_requirements=True,
                            is_optional=True)

                    has_agumon = any(agumon_filter(p) for p in player.battle_area)
                    has_wg = any(wg_hand_filter(c) for c in player.hand_cards)
                    if has_agumon and has_wg:
                        game.effect_select_own_permanent(
                            player, on_select_agumon,
                            filter_fn=agumon_filter,
                            is_optional=True)
                else:
                    # Return 1 opponent Digimon with lowest level to hand
                    enemy = player.enemy
                    if not enemy:
                        return
                    enemy_digimon = [p for p in enemy.battle_area
                                     if p.is_digimon and p.level is not None]
                    if not enemy_digimon:
                        return
                    min_level = min(p.level for p in enemy_digimon)

                    def lowest_level_filter(p):
                        return p.is_digimon and p.level is not None and p.level == min_level

                    def on_bounce(target_perm):
                        enemy.bounce_permanent_to_hand(target_perm)

                    game.effect_select_opponent_permanent(
                        player, on_bounce,
                        filter_fn=lowest_level_filter,
                        is_optional=False)

            game.effect_choose_branch(
                player, 2,
                callback=on_branch,
                branch_labels=[
                    "1 of your [Agumon] digivolves into [WarGreymon]",
                    "Return 1 opponent Digimon with lowest level to hand"
                ])

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: Inherited [When Attacking][OPT] Unsuspend if Omnimon ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUseAttack)
        effect3.set_effect_name("BT22-026 Unsuspend this digimon")
        effect3.set_effect_description(
            "[When Attacking] [Once Per Turn] If this Digimon has [Omnimon] "
            "in its name, it unsuspends."
        )
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT22_026_Unsuspend")
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
            """Unsuspend this Digimon (self only)."""
            perm = ctx.get('permanent')
            if perm and perm.is_suspended:
                perm.unsuspend()

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
