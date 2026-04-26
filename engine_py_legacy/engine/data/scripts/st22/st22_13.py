from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST22_13(CardScript):
    """ST22-13 GrandGalemon | Lv.5, Green, Bird Dragon/Vortex Warriors/LIBERATOR

    <Fortitude> <Vortex>
    [On Play] [When Digivolving] [When Attacking] You may suspend 1 Digimon.
        Then, this Digimon gains +3000 DP for the turn.
    [Inherited] [When Attacking] [Once Per Turn] If your opponent has no
        unsuspended Digimon, this Digimon with the [Vortex Warriors] trait may
        unsuspend.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Keyword: <Fortitude> ---
        effect_fortitude = ICardEffect()
        effect_fortitude.set_effect_name("ST22-13 Fortitude")
        effect_fortitude.set_effect_description("<Fortitude>")
        effect_fortitude._is_fortitude = True

        def cond_fortitude(context: Dict[str, Any]) -> bool:
            return True
        effect_fortitude.set_can_use_condition(cond_fortitude)
        effects.append(effect_fortitude)

        # --- Keyword: <Vortex> ---
        effect_vortex = ICardEffect()
        effect_vortex.set_effect_name("ST22-13 Vortex")
        effect_vortex.set_effect_description("<Vortex>")
        effect_vortex._is_vortex = True

        def cond_vortex(context: Dict[str, Any]) -> bool:
            return True
        effect_vortex.set_can_use_condition(cond_vortex)
        effects.append(effect_vortex)

        # --- Helper: grant +3000 DP for the turn on a permanent ---
        def _grant_dp(perm_now, game_obj):
            if perm_now and game_obj:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game_obj.register_modifier(
                    perm_now,
                    ModifierType.CHANGE_DP,
                    value_fn=lambda: 3000,
                    expiry='end_of_turn',
                )

        # --- Helper: offer to suspend 1 Digimon (optional) then grant DP ---
        def _suspend_and_dp(player, game_obj, perm_now):
            """You may suspend 1 Digimon (own or opponent). Then gain +3000 DP."""
            own_suspendable = [
                p for p in player.battle_area
                if p.is_digimon and not p.is_suspended
            ]
            opp_suspendable = [
                p for p in player.enemy.battle_area
                if p.is_digimon and not p.is_suspended
            ] if player.enemy else []

            if own_suspendable:
                game_obj.effect_select_own_permanent(
                    player,
                    lambda p: p.suspend(),
                    filter_fn=lambda p: p.is_digimon and not p.is_suspended,
                    is_optional=True,
                    prompt="You may suspend 1 of your Digimon.",
                )
            elif opp_suspendable:
                game_obj.effect_select_opponent_permanent(
                    player,
                    lambda p: p.suspend(),
                    filter_fn=lambda p: p.is_digimon and not p.is_suspended,
                    is_optional=True,
                    prompt="You may suspend 1 of your opponent's Digimon.",
                )
            # "+3000 DP for the turn" is mandatory (unconditional "Then" clause)
            _grant_dp(perm_now, game_obj)

        # --- [On Play] [When Digivolving]: both flags on one OnEnterFieldAnyone effect ---
        effect_op_wd = ICardEffect()
        effect_op_wd.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_op_wd.set_effect_name("ST22-13 Suspend + +3000DP [On Play / When Digivolving]")
        effect_op_wd.set_effect_description(
            "[On Play] [When Digivolving] You may suspend 1 Digimon. "
            "Then, this Digimon gains +3000 DP for the turn."
        )
        effect_op_wd.is_on_play = True
        effect_op_wd.is_when_digivolving = True
        effect_op_wd.is_optional = True

        def cond_op_wd(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_op_wd.set_can_use_condition(cond_op_wd)

        def process_op_wd(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game_obj = ctx.get('game')
            if not (player and game_obj):
                return
            perm_now = card.permanent_of_this_card() if card else None
            _suspend_and_dp(player, game_obj, perm_now)

        effect_op_wd.set_on_process_callback(process_op_wd)
        effects.append(effect_op_wd)

        # --- [When Attacking] ---
        effect_wa = ICardEffect()
        effect_wa.set_timing(EffectTiming.OnUseAttack)
        effect_wa.set_effect_name("ST22-13 Suspend + +3000DP [When Attacking]")
        effect_wa.set_effect_description(
            "[When Attacking] You may suspend 1 Digimon. "
            "Then, this Digimon gains +3000 DP for the turn."
        )
        effect_wa.is_on_attack = True
        effect_wa.is_optional = True

        def cond_wa(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_wa.set_can_use_condition(cond_wa)

        def process_wa(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game_obj = ctx.get('game')
            if not (player and game_obj):
                return
            perm_now = card.permanent_of_this_card() if card else None
            _suspend_and_dp(player, game_obj, perm_now)

        effect_wa.set_on_process_callback(process_wa)
        effects.append(effect_wa)

        # --- [Inherited] [When Attacking] [Once Per Turn] ---
        # If opponent has no unsuspended Digimon, this Digimon with [Vortex Warriors]
        # trait may unsuspend.
        effect_inh = ICardEffect()
        effect_inh.set_timing(EffectTiming.OnUseAttack)
        effect_inh.set_effect_name("ST22-13 Unsuspend if no opp unsuspended (Inherited)")
        effect_inh.set_effect_description(
            "[When Attacking] [Once Per Turn] If your opponent has no unsuspended "
            "Digimon, this Digimon with the [Vortex Warriors] trait may unsuspend."
        )
        effect_inh.is_inherited_effect = True
        effect_inh.is_on_attack = True
        effect_inh.is_optional = True
        effect_inh.set_max_count_per_turn(1)
        effect_inh.set_hash_string("ST22_13_inh_unsuspend")

        def cond_inh(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm_now = card.permanent_of_this_card()
            if perm_now is None:
                return False
            # This Digimon must have the [Vortex Warriors] trait on its top card
            top = perm_now.top_card if perm_now else None
            if top is None:
                return False
            traits = getattr(top, 'card_traits', []) or []
            if not any('Vortex Warriors' in t for t in traits):
                return False
            # Opponent must have no unsuspended Digimon
            owner = card.owner if card else None
            if owner and owner.enemy:
                opp_unsuspended = any(
                    p.is_digimon and not p.is_suspended
                    for p in owner.enemy.battle_area
                )
                if opp_unsuspended:
                    return False
            return True
        effect_inh.set_can_use_condition(cond_inh)

        def process_inh(ctx: Dict[str, Any]):
            perm_now = card.permanent_of_this_card() if card else None
            if perm_now:
                perm_now.unsuspend()
        effect_inh.set_on_process_callback(process_inh)
        effects.append(effect_inh)

        return effects
