from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_051(CardScript):
    """BT24-051 Merukimon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Lv.5 with [Beastkin] or [TS] for cost 3
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-051 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_trait = "Beastkin"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card):
                return False
            traits = getattr(permanent.top_card, 'card_traits', []) or []
            return (any('Beastkin' in tr for tr in traits)
                    or any('TS' in tr for tr in traits))
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played, if there are 3 or more Digimon, reduce the play cost by 5.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT24-051 Reduce play cost (5)")
        effect1.set_effect_description(
            "When this card would be played, if there are 3 or more Digimon, "
            "reduce the play cost by 5.")
        effect1.cost_reduction = 5

        def condition1(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            own_digi = len([p for p in owner.battle_area if p.is_digimon])
            enemy = owner.enemy if owner else None
            opp_digi = len([p for p in enemy.battle_area if p.is_digimon]) if enemy else 0
            return (own_digi + opp_digi) >= 3

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Not-shown cost reduction (EffectTiming.NoTiming equivalent — always-on modifier)
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-051 Play Cost -5 (not shown)")
        effect2.set_effect_description("Cost -5 (not shown)")
        effect2.cost_reduction = 5

        def condition2(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            own_digi = len([p for p in owner.battle_area if p.is_digimon])
            enemy = owner.enemy if owner else None
            opp_digi = len([p for p in enemy.battle_area if p.is_digimon]) if enemy else 0
            return (own_digi + opp_digi) >= 3

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Shared process for On Play / When Digivolving:
        # Suspend up to 2 opponent's Digimon or Tamers.
        # Then, 1 of your Digimon may get +5000 DP and attack (Rush/Piercing until end of
        # opponent's turn).
        def _shared_suspend_process(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Suspend up to 2 opponent's Digimon or Tamers
            enemy = player.enemy if player else None
            if enemy:
                targets = [p for p in enemy.battle_area if p.is_digimon or p.is_tamer]
                count = min(2, len(targets))
                for i in range(count):
                    if i < len(targets):
                        targets[i].suspend()
            # 1 of your Digimon may get +5000 DP and gain Rush/Piercing until end of opponent's
            # turn, then may attack.
            # NOTE: Temporary DP boost until end of opponent's turn is not yet fully supported
            # by the engine's duration modifiers. We apply a turn-scoped DP boost and a forced
            # attack selection.
            # Engine gap: +DP until end of opponent's turn duration is approximated as +DP
            # for this turn only.
            def on_select_attacker(target_perm):
                target_perm.change_dp(5000)
                # grant Rush and Piercing until end of opponent's turn
                # descriptive-tagged: grant_rush_piercing_until_opponent_turn_end
                pass

            game.effect_select_own_permanent(
                player, on_select_attacker,
                filter_fn=lambda p: p.is_digimon,
                is_optional=True)

        # Timing: EffectTiming.OnEnterFieldAnyone — [On Play]
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT24-051 Suspend 2, then 1 Digimon gains +5000 DP and attacks (On Play)")
        effect3.set_effect_description(
            "[On Play] Suspend 2 of your opponent's Digimon or Tamers. Then, 1 of your Digimon "
            "may get +5000 DP and gain <Rush> and <Piercing> until the end of your opponent's turn.")
        effect3.is_on_play = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_shared_suspend_process)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone — [When Digivolving]
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT24-051 Suspend 2, then 1 Digimon gains +5000 DP and attacks (When Digivolving)")
        effect4.set_effect_description(
            "[When Digivolving] Suspend 2 of your opponent's Digimon or Tamers. Then, 1 of your "
            "Digimon may get +5000 DP and gain <Rush> and <Piercing> until the end of your "
            "opponent's turn.")
        effect4.is_when_digivolving = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(_shared_suspend_process)
        effects.append(effect4)

        # Timing: EffectTiming.WhenRemoveField — [All Turns][Once Per Turn]
        # When this Digimon or any of your other Digimon with [TS] or [Beast]/[Animal]/[Sovereign]
        # would leave the battle area by your opponent's effects, by suspending this Digimon,
        # they don't leave.
        # NOTE: Engine gap — protecting OTHER Digimon via a substitute-suspend mechanism requires
        # engine support for WhenRemoveField to cancel the removal for a different permanent.
        # Current engine only supports self-protection via WhenRemoveField.
        # Implemented as best-effort self-protection; full multi-Digimon coverage is a gap.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.WhenRemoveField)
        effect5.set_effect_name(
            "BT24-051 [All Turns][OPT] Suspend self to prevent TS/Beast/Animal/Sovereign leaving")
        effect5.set_effect_description(
            "[All Turns][Once Per Turn] When this Digimon or any of your other Digimon with the "
            "[TS] trait or with [Beast], [Animal], or [Sovereign] in any of their traits would "
            "leave the battle area by your opponent's effects, by suspending this Digimon, "
            "they don't leave.")
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("SuspendToStay_BT24_051")

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            my_perm = card.permanent_of_this_card()
            # Must not already be suspended (can't suspend to protect if already suspended)
            if my_perm and my_perm.is_suspended:
                return False
            ctx_perm = context.get('permanent')
            if ctx_perm is my_perm:
                return True  # Protecting self
            # Other qualifying Digimon
            if ctx_perm and ctx_perm.top_card:
                traits = getattr(ctx_perm.top_card, 'card_traits', []) or []
                if (any('TS' in t for t in traits)
                        or any('Beast' in t for t in traits)
                        or any('Animal' in t for t in traits)
                        or any('Sovereign' in t for t in traits)):
                    return True
            return False

        effect5.set_can_use_condition(condition5)
        # descriptive-tagged: suspend_self_to_protect_other — engine gap for non-self protection
        effects.append(effect5)

        # Timing: EffectTiming.OnEnterFieldAnyone — [When Digivolving][Once Per Turn]
        # 1 of your Digimon may unsuspend.
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect6.set_effect_name("BT24-051 [When Digivolving][OPT] Unsuspend 1 Digimon")
        effect6.set_effect_description(
            "[When Digivolving] [Once Per Turn] 1 of your Digimon may unsuspend.")
        effect6.is_when_digivolving = True
        effect6.set_max_count_per_turn(1)
        effect6.set_hash_string("BT24_051_WD_WA")

        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend,
                filter_fn=lambda p: p.is_digimon,
                is_optional=True)

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        # Timing: EffectTiming.OnUseAttack — [Your Turn][Once Per Turn]
        # When one of your Digimon attacks, 1 of your Digimon may unsuspend.
        effect7 = ICardEffect()
        effect7.set_timing(EffectTiming.OnUseAttack)
        effect7.set_effect_name("BT24-051 [When Attacking][OPT] Unsuspend 1 Digimon")
        effect7.set_effect_description(
            "[When Attacking] [Once Per Turn] 1 of your Digimon may unsuspend.")
        effect7.is_on_attack = True
        effect7.set_max_count_per_turn(1)
        effect7.set_hash_string("BT24_051_WD_WA")

        def condition7(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = getattr(card, 'owner', None)
            if not (owner and owner.is_my_turn):
                return False
            return True

        effect7.set_can_use_condition(condition7)

        def process7(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend,
                filter_fn=lambda p: p.is_digimon,
                is_optional=True)

        effect7.set_on_process_callback(process7)
        effects.append(effect7)

        # Factory effect: Rush (Your Turn, for Iliad Digimon)
        effect8 = ICardEffect()
        effect8.set_effect_name("BT24-051 Rush (Iliad Digimon, Your Turn)")
        effect8.set_effect_description("Rush")
        effect8._is_rush = True

        def condition8(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = getattr(card, 'owner', None)
            if not (owner and owner.is_my_turn):
                return False
            ctx_perm = context.get('permanent')
            if ctx_perm and ctx_perm.top_card:
                traits = getattr(ctx_perm.top_card, 'card_traits', []) or []
                return any('Iliad' in t for t in traits)
            return False
        effect8.set_can_use_condition(condition8)
        effects.append(effect8)

        # Grant Rush + Piercing to all Iliad Digimon during your turn
        # NOTE: Granting Piercing to other Digimon via AddSkillClass is engine gap.
        # Rush is handled via _is_rush above. Piercing grant is descriptive-tagged.
        effect9 = ICardEffect()
        effect9.set_effect_name(
            "BT24-051 [Your Turn] All of your [Iliad] trait Digimon gain <Rush> and <Piercing>.")
        effect9.set_effect_description("Grant Rush and Piercing to Iliad Digimon")

        def condition9(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = getattr(card, 'owner', None)
            if not (owner and owner.is_my_turn):
                return False
            return True

        effect9.set_can_use_condition(condition9)

        def process9(ctx: Dict[str, Any]):
            # descriptive-tagged: grant_piercing_to_iliad_digimon — engine gap (AddSkillClass)
            pass

        effect9.set_on_process_callback(process9)
        effects.append(effect9)

        return effects
