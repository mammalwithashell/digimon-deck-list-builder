from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX7_034(CardScript):
    """EX7-034 GrandGalemon | Lv.5, Green, Bird Dragon/Vortex Warriors/LIBERATOR

    <Vortex>
    [When Digivolving] You may Suspend 1 Digimon. If this effect suspends your
        Digimon, this Digimon isn't affected by your opponent's Digimon's effects
        until the end of their turn.
    [Inherited] [Your Turn] [Once Per Turn] When this Digimon attacks your
        opponent's Digimon, you may unsuspend this Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Keyword: <Vortex> ---
        effect_vortex = ICardEffect()
        effect_vortex.set_effect_name("EX7-034 Vortex")
        effect_vortex.set_effect_description("<Vortex>")
        effect_vortex._is_vortex = True

        def cond_vortex(context: Dict[str, Any]) -> bool:
            return True
        effect_vortex.set_can_use_condition(cond_vortex)
        effects.append(effect_vortex)

        # --- [When Digivolving] You may Suspend 1 Digimon.
        #     If own Digimon was suspended => CANNOT_BE_AFFECTED until end of opponent's turn ---
        effect_wd = ICardEffect()
        effect_wd.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_wd.set_effect_name("EX7-034 Suspend 1 Digimon; effect immunity if own")
        effect_wd.set_effect_description(
            "[When Digivolving] You may Suspend 1 Digimon. If this effect "
            "suspends your Digimon, this Digimon isn't affected by your "
            "opponent's Digimon's effects until the end of their turn."
        )
        effect_wd.is_when_digivolving = True
        effect_wd.is_optional = True

        def cond_wd(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_wd.set_can_use_condition(cond_wd)

        def process_wd(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            perm = ctx.get('permanent')
            if not (player and game):
                return

            def _grant_immunity():
                """Grant CANNOT_BE_AFFECTED until end of opponent's turn."""
                target_perm = card.permanent_of_this_card() if card else None
                if target_perm and game:
                    from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                    game.register_modifier(
                        target_perm,
                        ModifierType.CANNOT_BE_AFFECTED,
                        value_fn=lambda: True,
                        expiry='end_of_opponent_turn',
                    )

            def on_own_suspended(target_perm):
                target_perm.suspend()
                _grant_immunity()

            # "You may Suspend 1 Digimon" — any Digimon (own or opponent).
            # If own Digimon suspended => immunity triggered.
            # We offer own first (optional); if no own exists, offer opponent.
            own_suspendable = [
                p for p in player.battle_area
                if p.is_digimon and not p.is_suspended
            ]
            opp_suspendable = [
                p for p in player.enemy.battle_area
                if p.is_digimon and not p.is_suspended
            ] if player.enemy else []

            if own_suspendable:
                game.effect_select_own_permanent(
                    player,
                    on_own_suspended,
                    filter_fn=lambda p: p.is_digimon and not p.is_suspended,
                    is_optional=True,
                    prompt="You may suspend 1 of your Digimon (grants effect immunity).",
                )
            elif opp_suspendable:
                game.effect_select_opponent_permanent(
                    player,
                    lambda p: p.suspend(),
                    filter_fn=lambda p: p.is_digimon and not p.is_suspended,
                    is_optional=True,
                    prompt="You may suspend 1 of your opponent's Digimon.",
                )

        effect_wd.set_on_process_callback(process_wd)
        effects.append(effect_wd)

        # --- [Inherited] [Your Turn] [Once Per Turn] When this Digimon attacks
        #     opponent's Digimon, you may unsuspend this Digimon ---
        effect_inh = ICardEffect()
        effect_inh.set_timing(EffectTiming.OnUseAttack)
        effect_inh.set_effect_name("EX7-034 Unsuspend when attacking opponent's Digimon (Inherited)")
        effect_inh.set_effect_description(
            "[Your Turn] [Once Per Turn] When this Digimon attacks your "
            "opponent's Digimon, you may unsuspend this Digimon."
        )
        effect_inh.is_inherited_effect = True
        effect_inh.is_on_attack = True
        effect_inh.is_optional = True
        effect_inh.set_max_count_per_turn(1)
        effect_inh.set_hash_string("EX7_034_inh_unsuspend")

        def cond_inh(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not (owner and owner.is_my_turn):
                return False
            perm_now = card.permanent_of_this_card()
            if perm_now is None:
                return False
            # Attack target must be opponent's Digimon (not a Player)
            game_obj = owner.game if owner else None
            if game_obj and game_obj.pending_attack:
                pa = game_obj.pending_attack
                if pa.attacker is not perm_now:
                    return False
                target = pa.original_target
                from engine_py_legacy.engine.core.player import Player as _Player
                if isinstance(target, _Player):
                    return False
                if not getattr(target, 'is_digimon', False):
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
