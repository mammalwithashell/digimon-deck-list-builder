from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_036(CardScript):
    """EX11-036 Dalphomon | Lv.6

    Alt digivolution: Lv.5 with [Maquinamon] in text for cost 3.

    <Vortex>
    [On Play] [When Digivolving] [When Attacking] [Once Per Turn] Suspend 2
    of your opponent's Digimon or Tamers. Then, 1 of their Digimon or Tamers
    can't unsuspend until their turn ends.
    [End of Your Turn] [Once Per Turn] 1 of your other Digimon may digivolve
    into a black Digimon card with [Maquinamon] in its text in the hand
    without paying the cost.

    --- Inherited ---
    [Your Turn] [Once Per Turn] When this Digimon gets linked, suspend 1 of
    your opponent's Digimon. Then, this Digimon may attack.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Alt digivolution: Lv.5 with [Maquinamon] in text for cost 3 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-036 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Maquinamon' in text):
                    return False
            else:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Vortex ---
        effect1 = ICardEffect()
        effect1.set_effect_name("EX11-036 Vortex")
        effect1.set_effect_description("Vortex")
        effect1._is_vortex = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Shared process for On Play / When Digivolving / When Attacking:
        # Suspend 2 of opponent's Digimon or Tamers. Then, 1 can't unsuspend.
        def _shared_suspend_and_cannot_unsuspend(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            def target_filter(p):
                return p.is_digimon or getattr(p, 'is_tamer', False)

            # Count valid targets
            valid_targets = [p for p in enemy.battle_area if target_filter(p)]
            suspend_count = min(2, len(valid_targets))
            if suspend_count <= 0:
                return

            suspended_so_far = []

            def suspend_next(remaining):
                if remaining <= 0:
                    # After suspending, select 1 opponent Digimon/Tamer
                    # that can't unsuspend until their turn ends
                    _select_cannot_unsuspend(player, game)
                    return

                def cannot_unsuspend_filter(p):
                    return (p.is_digimon or getattr(p, 'is_tamer', False)) and p not in suspended_so_far

                def on_suspend(target_perm):
                    target_perm.suspend()
                    suspended_so_far.append(target_perm)
                    suspend_next(remaining - 1)

                # Check if there are still valid targets
                opp_valid = [p for p in enemy.battle_area
                             if cannot_unsuspend_filter(p)]
                if not opp_valid:
                    _select_cannot_unsuspend(player, game)
                    return

                game.effect_select_opponent_permanent(
                    player, on_suspend,
                    filter_fn=cannot_unsuspend_filter,
                    is_optional=False,
                    prompt=f"Select a Digimon or Tamer to suspend ({len(suspended_so_far)+1}/{suspend_count}).")

            def _select_cannot_unsuspend(player, game):
                enemy = player.enemy
                if not enemy:
                    return

                def cu_filter(p):
                    return p.is_digimon or getattr(p, 'is_tamer', False)

                opp_valid = [p for p in enemy.battle_area if cu_filter(p)]
                if not opp_valid:
                    return

                def on_cannot_unsuspend(target_perm):
                    target_perm.grant_keyword('_is_cannot_unsuspend')

                game.effect_select_opponent_permanent(
                    player, on_cannot_unsuspend,
                    filter_fn=cu_filter,
                    is_optional=False,
                    prompt="Select 1 Digimon or Tamer that can't unsuspend.")

            suspend_next(suspend_count)

        # --- [On Play] ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX11-036 Suspend 2, 1 can't unsuspend")
        effect2.set_effect_description(
            "[On Play] [Once Per Turn] Suspend 2 of your opponent's Digimon "
            "or Tamers. Then, 1 of their Digimon or Tamers can't unsuspend "
            "until their turn ends.")
        effect2.is_on_play = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("EX11_036_OP_WD_WA")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_shared_suspend_and_cannot_unsuspend)
        effects.append(effect2)

        # --- [When Digivolving] ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX11-036 Suspend 2, 1 can't unsuspend")
        effect3.set_effect_description(
            "[When Digivolving] [Once Per Turn] Suspend 2 of your opponent's "
            "Digimon or Tamers. Then, 1 of their Digimon or Tamers can't "
            "unsuspend until their turn ends.")
        effect3.is_when_digivolving = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("EX11_036_OP_WD_WA")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_shared_suspend_and_cannot_unsuspend)
        effects.append(effect3)

        # --- [When Attacking] ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUseAttack)
        effect4.set_effect_name("EX11-036 Suspend 2, 1 can't unsuspend")
        effect4.set_effect_description(
            "[When Attacking] [Once Per Turn] Suspend 2 of your opponent's "
            "Digimon or Tamers. Then, 1 of their Digimon or Tamers can't "
            "unsuspend until their turn ends.")
        effect4.is_on_attack = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("EX11_036_OP_WD_WA")

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(_shared_suspend_and_cannot_unsuspend)
        effects.append(effect4)

        # --- [End of Your Turn] 1 other Digimon may digivolve into black
        # Digimon with [Maquinamon] in text from hand for free ---
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEndTurn)
        effect5.set_effect_name("EX11-036 1 other digimon may digivolve into a black card w/[Maquinamon] in text for free")
        effect5.set_effect_description(
            "[End of Your Turn] [Once Per Turn] 1 of your other Digimon may "
            "digivolve into a black Digimon card with [Maquinamon] in its "
            "text in the hand without paying the cost.")
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("EX11_036_EOYT")

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """1 of your OTHER Digimon may digivolve into a black card with [Maquinamon] in text."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def digi_filter(c):
                colors = [col.name for col in getattr(c, 'card_colors', [])]
                if 'Black' not in colors:
                    return False
                text = getattr(c, 'card_text', '') or ''
                if 'Maquinamon' not in text:
                    return False
                return True

            # Must target OTHER Digimon
            def other_digimon_filter(p):
                return p.is_digimon and p is not perm

            def on_target_selected(target_perm):
                if target_perm is None:
                    return
                game.effect_digivolve_from_hand(
                    player, target_perm, digi_filter, is_optional=True,
                    cost_override=0)

            game.effect_select_own_permanent(
                player, on_target_selected, filter_fn=other_digimon_filter,
                is_optional=True,
                prompt="Select 1 of your other Digimon to digivolve into a black [Maquinamon] card.")

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # --- Assembly: 5 Green Digimon with [Maquinamon] in text, reduce cost by 5 ---
        # (Assembly is a continuous cost-reduction mechanic, modeled as alt-digi)
        effect6 = ICardEffect()
        effect6.set_effect_name("EX11-036 Assembly")
        effect6.set_effect_description("Assembly: 5 Green Digimon cards with [Maquinamon] in text")

        def condition6(context: Dict[str, Any]) -> bool:
            return True
        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            pass
        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        # --- Inherited: [Your Turn] [Once Per Turn] When this Digimon gets
        # linked, suspend 1 of your opponent's Digimon. Then, this Digimon
        # may attack. ---
        effect7 = ICardEffect()
        effect7.set_timing(EffectTiming.WhenLinked)
        effect7.set_effect_name("EX11-036 Suspend 1 Digimon, then this may attack")
        effect7.set_effect_description(
            "[Your Turn] [Once Per Turn] When this Digimon gets linked, "
            "suspend 1 of your opponent's Digimon. Then, this Digimon may attack.")
        effect7.is_inherited_effect = True
        effect7.is_optional = True
        effect7.set_max_count_per_turn(1)
        effect7.set_hash_string("EX11_036_YT_ESS")

        def condition7(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect7.set_can_use_condition(condition7)

        def process7(ctx: Dict[str, Any]):
            """Suspend 1 opponent's Digimon. Then, this Digimon may attack."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon

            def on_suspend(target_perm):
                target_perm.suspend()

            # Suspend 1 opponent's Digimon (optional if no targets)
            enemy = player.enemy if player else None
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon] if enemy else []
            if opp_digimon:
                game.effect_select_opponent_permanent(
                    player, on_suspend, filter_fn=target_filter, is_optional=False,
                    prompt="Select 1 opponent's Digimon to suspend.")

            # Then, this Digimon may attack
            if perm and game:
                from ....interfaces.modifiers import ModifierType
                game.register_modifier(
                    perm, ModifierType.FORCE_ATTACK,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect7.set_on_process_callback(process7)
        effects.append(effect7)

        return effects
