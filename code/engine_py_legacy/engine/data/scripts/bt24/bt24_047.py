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


class BT24_047(CardScript):
    """BT24-047 Kokatorimon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _make_on_play_or_digivolve_effect(is_on_play: bool) -> 'ICardEffect':
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect.set_effect_name("BT24-047 Suspend, then unsuspend Avian/Bird/Vortex Warriors and may attack")
            effect.set_effect_description(
                "[On Play] [When Digivolving] You may suspend 1 Digimon. If this "
                "effect suspended your Digimon, 1 of your Digimon with [Avian] or "
                "[Bird] in any of its traits or the [Vortex Warriors] trait "
                "unsuspends. If this effect unsuspended, that Digimon may attack."
            )
            if is_on_play:
                effect.is_on_play = True
            else:
                effect.is_when_digivolving = True

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                from ....data.enums import GamePhase

                opponent = player.enemy
                own_area = player.battle_area
                opp_area = opponent.battle_area if opponent else []

                # Build combined valid list: own + opponent Digimon
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

                suspended_own = [False]

                def on_select(action_id: int):
                    target_perm = None
                    if _SEL_MY_FIELD_START <= action_id < _SEL_MY_FIELD_START + len(own_area):
                        idx = action_id - _SEL_MY_FIELD_START
                        if 0 <= idx < len(own_area):
                            target_perm = own_area[idx]
                            suspended_own[0] = True
                    elif _SEL_OPP_FIELD_START <= action_id < _SEL_OPP_FIELD_START + len(opp_area):
                        idx = action_id - _SEL_OPP_FIELD_START
                        if 0 <= idx < len(opp_area):
                            target_perm = opp_area[idx]

                    if target_perm:
                        target_perm.suspend()

                    # Follow-up only if own Digimon was suspended
                    if not suspended_own[0]:
                        return

                    # Unsuspend 1 of your Digimon with Avian/Bird/Vortex Warriors trait
                    unsuspended_perm = [None]

                    def trait_filter(p):
                        if not p.is_digimon:
                            return False
                        top = getattr(p, 'top_card', None)
                        traits = getattr(top, 'card_traits', []) if top else []
                        traits = traits or []
                        if any('Avian' in t or 'Bird' in t for t in traits):
                            return True
                        if any('Vortex Warriors' in t for t in traits):
                            return True
                        return False

                    def on_unsuspend(target_unsuspend_perm):
                        target_unsuspend_perm.unsuspend()
                        unsuspended_perm[0] = target_unsuspend_perm
                        # If that Digimon was unsuspended, it may attack (grant Rush)
                        target_unsuspend_perm.grant_keyword('_is_rush')

                    game.effect_select_own_permanent(
                        player, on_unsuspend, filter_fn=trait_filter, is_optional=False)

                game.request_selection(
                    GamePhase.SelectTarget, player, on_select, valid,
                    is_optional=True,
                    prompt="You may suspend 1 Digimon (own or opponent)."
                )

            effect.set_on_process_callback(process)
            return effect

        effects.append(_make_on_play_or_digivolve_effect(is_on_play=True))
        effects.append(_make_on_play_or_digivolve_effect(is_on_play=False))

        # Timing: EffectTiming.OnEndBattle
        # [All Turns] [Once Per Turn] When this Digimon deletes your opponent's
        # Digimon in battle, gain 1 memory.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndBattle)
        effect2.set_effect_name("BT24-047 Memory +1")
        effect2.set_effect_description(
            "[All Turns] [Once Per Turn] When this Digimon deletes your opponent's "
            "Digimon in battle, gain 1 memory."
        )
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT24_047_Inherited")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
