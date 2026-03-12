from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


# Own-field selection indices: 100-113, opponent field: 114-127
_SEL_MY_FIELD_START = 100
_SEL_OPP_FIELD_START = 114
_FIELD_SLOTS = 14


class EX11_026(CardScript):
    """EX11-026 Pteromon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _make_suspend_dp_effect(timing, *, is_on_play=False, is_when_digivolving=False,
                                    name_tag):
            """
            [When Moving] / [On Play] You may suspend 1 Digimon. If this effect suspended
            your Digimon, 1 of your Digimon with [Avian] or [Bird] in any of its traits
            or the [Vortex Warriors] trait gets +3000 DP until your opponent's turn ends.
            """
            eff = ICardEffect()
            eff.set_timing(timing)
            eff.is_optional = True
            if is_on_play:
                eff.is_on_play = True
            elif is_when_digivolving:
                eff.is_when_digivolving = True
            eff.set_effect_name(f"EX11-026 [{name_tag}] Suspend 1 Digimon, +3000 DP if own")
            eff.set_effect_description(
                f"[{name_tag}] You may suspend 1 Digimon. If this effect suspended your Digimon, "
                "1 of your Digimon with [Avian] or [Bird] in any of its traits or the "
                "[Vortex Warriors] trait gets +3000 DP until your opponent's turn ends."
            )

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            eff.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                from ....data.enums import GamePhase

                opponent = player.enemy
                own_area = player.battle_area
                opp_area = opponent.battle_area if opponent else []

                # Build valid indices spanning both sides
                valid = []
                for i, p in enumerate(own_area):
                    if p.is_digimon:
                        valid.append(_SEL_MY_FIELD_START + i)
                for i, p in enumerate(opp_area):
                    if p.is_digimon:
                        if game.modifiers.can_be_selected_by_effect(p):
                            valid.append(_SEL_OPP_FIELD_START + i)

                if not valid:
                    return

                def on_select(action_id: int):
                    suspended_own = False
                    target_perm = None

                    if _SEL_MY_FIELD_START <= action_id < _SEL_MY_FIELD_START + len(own_area):
                        idx = action_id - _SEL_MY_FIELD_START
                        if 0 <= idx < len(own_area):
                            target_perm = own_area[idx]
                            suspended_own = True
                    elif _SEL_OPP_FIELD_START <= action_id < _SEL_OPP_FIELD_START + len(opp_area):
                        idx = action_id - _SEL_OPP_FIELD_START
                        if 0 <= idx < len(opp_area):
                            target_perm = opp_area[idx]

                    if target_perm:
                        target_perm.suspend()

                    # If suspended own Digimon, grant +3000 DP to Bird/Avian/VW Digimon
                    if suspended_own and player and game:
                        def dp_target_filter(p):
                            if not p.is_digimon:
                                return False
                            top = getattr(p, 'top_card', None)
                            if top is None:
                                return False
                            traits = getattr(top, 'card_traits', []) or []
                            return any(
                                'Bird' in t or 'Avian' in t or 'Vortex Warriors' in t
                                for t in traits
                            )

                        def grant_dp(dp_perm):
                            from ....interfaces.modifiers import ModifierType
                            game.register_modifier(
                                ModifierType.CHANGE_DP, dp_perm,
                                value_fn=lambda: 3000,
                                expiry='end_of_opponent_turn'
                            )

                        game.effect_select_own_permanent(
                            player, grant_dp,
                            filter_fn=dp_target_filter,
                            is_optional=False,
                            prompt="Select 1 of your Digimon with Bird, Avian, or Vortex Warriors to get +3000 DP."
                        )

                game.request_selection(
                    GamePhase.SelectTarget, player, on_select, valid,
                    is_optional=True,
                    prompt="You may suspend 1 Digimon (own or opponent)."
                )

            eff.set_on_process_callback(process)
            return eff

        # Effect 0: [When Moving]
        effect0 = _make_suspend_dp_effect(
            EffectTiming.OnMove,
            is_when_digivolving=True,
            name_tag="When Moving"
        )
        effects.append(effect0)

        # Effect 1: [On Play]
        effect1 = _make_suspend_dp_effect(
            EffectTiming.OnEnterFieldAnyone,
            is_on_play=True,
            name_tag="On Play"
        )
        effects.append(effect1)

        # Effect 2 (Inherited): [Your Turn] [Once Per Turn] When this Digimon wins a battle,
        # gain 1 memory.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndBattle)
        effect2.set_effect_name("EX11-026 Inherited: Win battle -> Gain 1 memory")
        effect2.set_effect_description(
            "[Your Turn] [Once Per Turn] When this Digimon wins a battle, gain 1 memory."
        )
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("EX11_026_ESS_WinBattle_Mem")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
