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


class P_166(CardScript):
    """P-166 Galemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _make_suspend_digivolve_effect(*, is_on_play=False, is_when_digivolving=False,
                                           name_tag):
            """
            [On Play] / [When Digivolving] You may suspend 1 Digimon. Then, if it's your
            turn, this Digimon may digivolve into a Digimon card with [Bird] or [Avian] in
            any of its traits in the hand. For every other suspended Digimon, reduce this
            effect's digivolution cost by 1.
            """
            eff = ICardEffect()
            eff.set_timing(EffectTiming.OnEnterFieldAnyone)
            eff.is_optional = True
            if is_on_play:
                eff.is_on_play = True
            elif is_when_digivolving:
                eff.is_when_digivolving = True
            eff.set_effect_name(
                f"P-166 [{name_tag}] Suspend 1 Digimon, maybe digivolve into Bird/Avian"
            )
            eff.set_effect_description(
                f"[{name_tag}] You may suspend 1 Digimon. Then, if it's your turn, this "
                "Digimon may digivolve into a Digimon card with [Bird] or [Avian] in any "
                "of its traits in the hand. For every other suspended Digimon, reduce this "
                "effect's digivolution cost by 1."
            )

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            eff.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                """
                Step 1: Optionally suspend 1 Digimon (any).
                Step 2: If it's your turn, this Digimon may digivolve into a Bird/Avian card.
                        Cost reduction: -1 for every OTHER currently-suspended Digimon on field.
                """
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and game):
                    return

                from ....data.enums import GamePhase

                opponent = player.enemy
                own_area = player.battle_area
                opp_area = opponent.battle_area if opponent else []

                # Build combined valid list for suspend selection
                valid = []
                for i, p in enumerate(own_area):
                    if p.is_digimon:
                        valid.append(_SEL_MY_FIELD_START + i)
                for i, p in enumerate(opp_area):
                    if p.is_digimon:
                        if game.modifiers.can_be_selected_by_effect(p):
                            valid.append(_SEL_OPP_FIELD_START + i)

                def do_digivolve_step():
                    # Only allow digivolve on your own turn
                    if not (card and card.owner and card.owner.is_my_turn):
                        return
                    if not perm:
                        return

                    # Count other suspended Digimon (own and opponent) for cost reduction
                    all_perms = list(own_area) + list(opp_area)
                    other_suspended = sum(
                        1 for p in all_perms
                        if p is not perm and p.is_digimon and p.is_suspended
                    )

                    def bird_avian_filter(c):
                        if not getattr(c, 'is_digimon', False):
                            return False
                        traits = getattr(c, 'card_traits', []) or []
                        return any('Bird' in t or 'Avian' in t for t in traits)

                    game.effect_digivolve_from_hand(
                        player, perm, bird_avian_filter,
                        cost_reduction=other_suspended,
                        is_optional=True,
                        prompt=(
                            f"Digivolve into a Bird/Avian Digimon from hand "
                            f"(cost reduced by {other_suspended} for suspended Digimon)."
                        )
                    )

                if not valid:
                    # No targets to suspend — still allow digivolve
                    do_digivolve_step()
                    return

                def on_select(action_id: int):
                    target_perm = None
                    if _SEL_MY_FIELD_START <= action_id < _SEL_MY_FIELD_START + len(own_area):
                        idx = action_id - _SEL_MY_FIELD_START
                        if 0 <= idx < len(own_area):
                            target_perm = own_area[idx]
                    elif _SEL_OPP_FIELD_START <= action_id < _SEL_OPP_FIELD_START + len(opp_area):
                        idx = action_id - _SEL_OPP_FIELD_START
                        if 0 <= idx < len(opp_area):
                            target_perm = opp_area[idx]
                    if target_perm:
                        target_perm.suspend()
                    # After suspend (or decline), proceed to digivolve step
                    do_digivolve_step()

                def on_decline():
                    # Player chose not to suspend — still allow digivolve
                    do_digivolve_step()

                game.request_selection(
                    GamePhase.SelectTarget, player, on_select, valid,
                    is_optional=True,
                    prompt="You may suspend 1 Digimon (own or opponent).",
                    on_decline=on_decline
                )

            eff.set_on_process_callback(process)
            return eff

        # Effect 0: [On Play]
        effect0 = _make_suspend_digivolve_effect(is_on_play=True, name_tag="On Play")
        effects.append(effect0)

        # Effect 1: [When Digivolving]
        effect1 = _make_suspend_digivolve_effect(
            is_when_digivolving=True, name_tag="When Digivolving")
        effects.append(effect1)

        # Effect 2 (Inherited): [Your Turn] This Digimon gets +2000 DP.
        effect2 = ICardEffect()
        effect2.set_effect_name("P-166 Inherited: +2000 DP")
        effect2.set_effect_description("[Your Turn] This Digimon gets +2000 DP.")
        effect2.is_inherited_effect = True
        effect2.dp_modifier = 2000

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
