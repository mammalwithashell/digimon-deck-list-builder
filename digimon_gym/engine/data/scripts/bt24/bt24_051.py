from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_051(CardScript):
    """BT24-051 Merukimon | Lv.6

    When this card would be played, if there are 3 or more Digimon, reduce the
    play cost by 5.
    [On Play] [When Digivolving] Suspend 2 of your opponent's Digimon or Tamers.
    Then, 1 of your Digimon may get +5000 DP for the turn and attack your
    opponent's Digimon.
    [When Digivolving] [When Attacking] [Once Per Turn] 1 of your Digimon may
    unsuspend.
    [Your Turn] All of your [Iliad] trait Digimon gain <Rush> and <Piercing>.
    """

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
        # When this card would be played, if there are 3 or more Digimon, reduce
        # the play cost by 5.
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

        # Not-shown cost reduction (EffectTiming.NoTiming equivalent)
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
        # Then, 1 of your Digimon may get +5000 DP for the turn and attack your
        # opponent's Digimon.
        def _shared_suspend_process(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Suspend up to 2 opponent's Digimon or Tamers (player selects)
            suspended_targets = []

            def target_filter(p):
                return (p.is_digimon or p.is_tamer) and p not in suspended_targets

            def on_suspend_1(target_perm):
                target_perm.suspend()
                suspended_targets.append(target_perm)

                # Second suspend
                def on_suspend_2(target_perm2):
                    target_perm2.suspend()

                game.effect_select_opponent_permanent(
                    player, on_suspend_2, filter_fn=target_filter,
                    is_optional=True,
                    prompt="Suspend a second opponent's Digimon or Tamer (optional).")

            game.effect_select_opponent_permanent(
                player, on_suspend_1, filter_fn=target_filter,
                is_optional=False,
                prompt="Suspend 1 of your opponent's Digimon or Tamers.")

            # Then, 1 of your Digimon may get +5000 DP for the turn and attack
            # your opponent's Digimon.
            def on_select_attacker(target_perm):
                target_perm.change_dp(5000)
                # Grant Rush so it can attack this turn even with sickness
                target_perm.grant_keyword('_is_rush')
                # Unsuspend so it's ready to attack
                if target_perm.is_suspended:
                    target_perm.unsuspend()

            game.effect_select_own_permanent(
                player, on_select_attacker,
                filter_fn=lambda p: p.is_digimon,
                is_optional=True,
                prompt="Select 1 of your Digimon to get +5000 DP and attack.")

        # Timing: EffectTiming.OnEnterFieldAnyone — [On Play]
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT24-051 Suspend 2, then 1 Digimon gains +5000 DP and attacks (On Play)")
        effect3.set_effect_description(
            "[On Play] Suspend 2 of your opponent's Digimon or Tamers. Then, 1 of "
            "your Digimon may get +5000 DP for the turn and attack your opponent's "
            "Digimon.")
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
            "[When Digivolving] Suspend 2 of your opponent's Digimon or Tamers. "
            "Then, 1 of your Digimon may get +5000 DP for the turn and attack "
            "your opponent's Digimon.")
        effect4.is_when_digivolving = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(_shared_suspend_process)
        effects.append(effect4)

        # Timing: EffectTiming.WhenRemoveField — [All Turns][Once Per Turn]
        # When this Digimon or any of your other Digimon with [TS] or
        # [Beast]/[Animal]/[Sovereign] would leave the battle area by your
        # opponent's effects, by suspending this Digimon, they don't leave.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.WhenRemoveField)
        effect5.set_effect_name(
            "BT24-051 [All Turns][OPT] Suspend self to prevent TS/Beast/Animal/Sovereign leaving")
        effect5.set_effect_description(
            "[All Turns][Once Per Turn] When this Digimon or any of your other "
            "Digimon with the [TS] trait or with [Beast], [Animal], or [Sovereign] "
            "in any of their traits would leave the battle area by your opponent's "
            "effects, by suspending this Digimon, they don't leave.")
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
            # Must not already be suspended (need to suspend as cost)
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

        def process5(ctx: Dict[str, Any]):
            """Suspend this Digimon to prevent it or a qualifying ally from leaving."""
            my_perm = card.permanent_of_this_card()
            if my_perm and not my_perm.is_suspended:
                my_perm.suspend()

        effect5.set_on_process_callback(process5)
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
                is_optional=True,
                prompt="Select 1 of your Digimon to unsuspend.")

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        # Timing: EffectTiming.OnUseAttack — [When Attacking][Once Per Turn]
        # When one of your Digimon attacks, 1 of your Digimon may unsuspend.
        effect7 = ICardEffect()
        effect7.set_timing(EffectTiming.OnUseAttack)
        effect7.set_effect_name("BT24-051 [When Attacking][OPT] Unsuspend 1 Digimon")
        effect7.set_effect_description(
            "[When Attacking] [Once Per Turn] When one of your Digimon attacks, "
            "1 of your Digimon may unsuspend.")
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
                is_optional=True,
                prompt="Select 1 of your Digimon to unsuspend.")

        effect7.set_on_process_callback(process7)
        effects.append(effect7)

        # Helper: check if a permanent has [Iliad] trait
        def _is_iliad(perm):
            top = getattr(perm, 'top_card', None)
            if not top:
                return False
            traits = getattr(top, 'card_traits', []) or []
            return any('Iliad' in t for t in traits)

        # [Your Turn] All of your [Iliad] trait Digimon gain <Rush>.
        # Uses _applies_to_all_own_digimon aura pattern so has_keyword scans
        # this effect on other permanents.
        effect8 = ICardEffect()
        effect8.set_effect_name("BT24-051 [Your Turn] Iliad Digimon gain Rush")
        effect8.set_effect_description(
            "[Your Turn] All of your [Iliad] trait Digimon gain <Rush>.")
        effect8._is_rush = True
        effect8._applies_to_all_own_digimon = True
        effect8._keyword_permanent_condition = _is_iliad

        def condition8(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = getattr(card, 'owner', None)
            if not (owner and owner.is_my_turn):
                return False
            # When checked for self via has_keyword, context['permanent'] is self
            ctx_perm = context.get('permanent')
            if ctx_perm is not None:
                return _is_iliad(ctx_perm)
            return True
        effect8.set_can_use_condition(condition8)
        effects.append(effect8)

        # [Your Turn] All of your [Iliad] trait Digimon gain <Piercing>.
        effect9 = ICardEffect()
        effect9.set_effect_name("BT24-051 [Your Turn] Iliad Digimon gain Piercing")
        effect9.set_effect_description(
            "[Your Turn] All of your [Iliad] trait Digimon gain <Piercing>.")
        effect9._is_piercing = True
        effect9._applies_to_all_own_digimon = True
        effect9._keyword_permanent_condition = _is_iliad

        def condition9(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = getattr(card, 'owner', None)
            if not (owner and owner.is_my_turn):
                return False
            ctx_perm = context.get('permanent')
            if ctx_perm is not None:
                return _is_iliad(ctx_perm)
            return True
        effect9.set_can_use_condition(condition9)
        effects.append(effect9)

        return effects
