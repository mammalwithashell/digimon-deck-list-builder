from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_027(CardScript):
    """BT17-027 MetalGarurumon | Lv.6 Blue Digimon

    Alt digivolve: from Lv.5 [Garurumon] for 3.
    Play cost -3 if you have a Tamer with [Matt Ishida].
    [On Play] Choose 1: 1 opponent Digimon or Tamer can't suspend until
        end of their turn, or 1 of your [Agumon] may digivolve into
        [WarGreymon] from hand ignoring requirements without paying cost.
    [When Digivolving] Same as On Play.
    Inherited: [When Attacking][Once Per Turn] If this Digimon has [Omnimon]
        in its name, unsuspend this Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from Lv.5 [Garurumon] for 3 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT17-027 Alt digivolve from Lv.5 Garurumon")
        effect0.set_effect_description("Alt digivolve: from Lv.5 [Garurumon] for 3")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_name = "Garurumon"
        effect0._alt_digi_level = 5

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: Play cost -3 with [Matt Ishida] tamer ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT17-027 Play cost -3")
        effect1.set_effect_description("Play cost -3 if you have [Matt Ishida] tamer")
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
                (p.top_card.contains_card_name('Matt Ishida') or
                 p.top_card.contains_card_name('MattIshida'))
                for p in card.owner.battle_area
            )

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Shared process for On Play / When Digivolving ---
        def _process_branch(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_branch(choice: int):
                if choice == 0:
                    # 1 opponent Digimon or Tamer can't suspend until end of their turn
                    def opp_filter(p):
                        return p.is_digimon or p.is_tamer

                    def on_cant_suspend(target_perm):
                        from ....interfaces.modifiers import ModifierType
                        game.register_modifier(
                            target_perm, ModifierType.CANNOT_SUSPEND,
                            condition=lambda perm, c: perm is target_perm,
                            expiry='end_of_opponent_turn')

                    game.effect_select_opponent_permanent(
                        player, on_cant_suspend,
                        filter_fn=opp_filter,
                        is_optional=False)
                else:
                    # 1 of your [Agumon] may digivolve into [WarGreymon] from hand
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
                    has_wg_hand = any(wg_hand_filter(c) for c in player.hand_cards)
                    if has_agumon and has_wg_hand:
                        game.effect_select_own_permanent(
                            player, on_select_agumon,
                            filter_fn=agumon_filter,
                            is_optional=True)

            game.effect_choose_branch(
                player, 2,
                callback=on_branch,
                branch_labels=[
                    "1 opponent Digimon/Tamer can't suspend",
                    "1 of your [Agumon] digivolves into [WarGreymon]"
                ])

        # --- Effect 2: [On Play] ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT17-027 On Play: choose effect")
        effect2.set_effect_description(
            "[On Play] Can't suspend 1 opponent, or Agumon->WarGreymon."
        )
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            return perm is not None
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_process_branch)
        effects.append(effect2)

        # --- Effect 3: [When Digivolving] same as On Play ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT17-027 When Digivolving: choose effect")
        effect3.set_effect_description(
            "[When Digivolving] Can't suspend 1 opponent, or Agumon->WarGreymon."
        )
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            return perm is not None
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_process_branch)
        effects.append(effect3)

        # --- Effect 4: Inherited [When Attacking][Once Per Turn] unsuspend if Omnimon ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUseAttack)
        effect4.set_effect_name("BT17-027 Inherited: When Attacking unsuspend")
        effect4.set_effect_description(
            "[When Attacking][Once Per Turn] If this Digimon has [Omnimon] in "
            "its name, unsuspend this Digimon."
        )
        effect4.is_inherited_effect = True
        effect4.is_on_attack = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("Unsuspend_BT17-027")

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
            perm = card.permanent_of_this_card() if card else None
            if perm:
                perm.unsuspend()

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
