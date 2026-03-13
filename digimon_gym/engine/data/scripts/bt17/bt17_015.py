from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_015(CardScript):
    """BT17-015 WarGreymon | Lv.6 Red Digimon

    Alt digivolve: from Lv.5 [Greymon] for 3.
    Play cost -3 if you have a Tamer with [Tai Kamiya].
    [On Play] Choose 1: Delete 1 opponent Digimon with 8000 DP or less,
        or 1 of your [Gabumon] may digivolve into [MetalGarurumon] from hand
        ignoring requirements and without paying cost.
    [When Digivolving] Same as On Play.
    Inherited: [When Attacking][Once Per Turn] If this Digimon has [Omnimon]
        in its name, trash the top card of opponent's security stack.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from Lv.5 [Greymon] for 3 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT17-015 Alt digivolve from Lv.5 Greymon")
        effect0.set_effect_description("Alt digivolve: from Lv.5 [Greymon] for 3")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_name = "Greymon"
        effect0._alt_digi_level = 5

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: Play cost -3 with [Tai Kamiya] tamer ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT17-015 Play cost -3")
        effect1.set_effect_description("Play cost -3 if you have [Tai Kamiya] tamer")
        effect1.cost_reduction = 3

        def condition1(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            if not (card and card.owner):
                return False
            if card not in card.owner.hand_cards:
                return False
            return any(
                p.is_tamer and p.top_card and
                (p.top_card.contains_card_name('Tai Kamiya') or
                 p.top_card.contains_card_name('TaiKamiya'))
                for p in card.owner.battle_area
            )

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Shared helpers for On Play / When Digivolving ---
        def _process_branch(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_branch(choice: int):
                if choice == 0:
                    # Delete 1 opponent Digimon with 8000 DP or less
                    def delete_filter(p):
                        return p.is_digimon and p.dp <= 8000

                    def on_delete(target_perm):
                        player.enemy.delete_permanent(target_perm)

                    game.effect_select_opponent_permanent(
                        player, on_delete,
                        filter_fn=delete_filter,
                        is_optional=False)
                else:
                    # 1 of your [Gabumon] may digivolve into [MetalGarurumon]
                    def gabumon_filter(p):
                        return (p.is_digimon and p.top_card and
                                p.top_card.contains_card_name('Gabumon') and
                                any('Gabumon' == n for n in (p.top_card.card_names or [])))

                    def mg_hand_filter(c):
                        if not getattr(c, 'is_digimon', False):
                            return False
                        return any('MetalGarurumon' == n for n in (getattr(c, 'card_names', []) or []))

                    def on_select_gabumon(target_perm):
                        game.effect_digivolve_from_hand(
                            player, target_perm, mg_hand_filter,
                            cost_override=0, ignore_requirements=True,
                            is_optional=True)

                    has_gabumon = any(gabumon_filter(p) for p in player.battle_area)
                    has_mg_hand = any(mg_hand_filter(c) for c in player.hand_cards)
                    if has_gabumon and has_mg_hand:
                        game.effect_select_own_permanent(
                            player, on_select_gabumon,
                            filter_fn=gabumon_filter,
                            is_optional=True)

            game.effect_choose_branch(
                player, 2,
                callback=on_branch,
                branch_labels=[
                    "Delete 1 opponent Digimon with 8000 DP or less",
                    "1 of your [Gabumon] digivolves into [MetalGarurumon]"
                ])

        # --- Effect 2: [On Play] ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT17-015 On Play: choose effect")
        effect2.set_effect_description(
            "[On Play] Delete 1 opponent Digimon with 8000 DP or less, "
            "or 1 of your [Gabumon] digivolves into [MetalGarurumon]."
        )
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_process_branch)
        effects.append(effect2)

        # --- Effect 3: [When Digivolving] same as On Play ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT17-015 When Digivolving: choose effect")
        effect3.set_effect_description(
            "[When Digivolving] Delete 1 opponent Digimon with 8000 DP or less, "
            "or 1 of your [Gabumon] digivolves into [MetalGarurumon]."
        )
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_process_branch)
        effects.append(effect3)

        # --- Effect 4: Inherited [When Attacking][Once Per Turn] trash top security ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUseAttack)
        effect4.set_effect_name("BT17-015 Inherited: When Attacking trash security")
        effect4.set_effect_description(
            "[When Attacking][Once Per Turn] If this Digimon has [Omnimon] in "
            "its name, trash the top card of opponent's security stack."
        )
        effect4.is_inherited_effect = True
        effect4.is_on_attack = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("ESS_BT17_015")

        def condition4(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            ctx_perm = context.get('attacker') or context.get('permanent')
            if perm and ctx_perm and ctx_perm is not perm:
                return False
            if not perm.top_card or not perm.top_card.contains_card_name('Omnimon'):
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if enemy and enemy.security_cards:
                top_sec = enemy.security_cards[-1]
                enemy.trash_security_card(top_sec)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
