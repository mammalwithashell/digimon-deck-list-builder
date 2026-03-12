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


class BT24_044(CardScript):
    """BT24-044 Muchomon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may suspend 1 level 6 or lower Digimon. If this effect
        # suspended your Digimon, reveal the top 3 cards of your deck. Add 1
        # [Shoto Kazama] and 1 card with [Avian] or [Bird] in any of its traits
        # or the [Vortex Warriors] trait among them to the hand. Return the rest
        # to the bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT24-044 Suspend 1 Digimon, then maybe search top 3.")
        effect0.set_effect_description(
            "[On Play] You may suspend 1 level 6 or lower Digimon. If this effect "
            "suspended your Digimon, reveal the top 3 cards of your deck. Add 1 "
            "[Shoto Kazama] and 1 card with [Avian] or [Bird] in any of its traits "
            "or the [Vortex Warriors] trait among them to the hand. Return the rest "
            "to the bottom of the deck."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            from ....data.enums import GamePhase

            opponent = player.enemy
            own_area = player.battle_area
            opp_area = opponent.battle_area if opponent else []

            def level_ok(p):
                top = getattr(p, 'top_card', None)
                lvl = getattr(top, 'level', None) if top else None
                if lvl is None:
                    lvl = getattr(p, 'level', None)
                return p.is_digimon and (lvl is None or lvl <= 6)

            # Build combined valid list: own + opponent Digimon level 6 or lower
            valid = []
            for i, p in enumerate(own_area):
                if level_ok(p):
                    valid.append(_SEL_MY_FIELD_START + i)
            for i, p in enumerate(opp_area):
                if level_ok(p):
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

                # The follow-up reveal/search only happens if an own Digimon was suspended.
                if not suspended_own[0]:
                    return

                def reveal_filter_shoto(c):
                    return any('Shoto Kazama' in n for n in getattr(c, 'card_names', []))

                def reveal_filter_avian(c):
                    traits = getattr(c, 'card_traits', []) or []
                    if any('Avian' in t or 'Bird' in t for t in traits):
                        return True
                    if any('Vortex Warriors' in t for t in traits):
                        return True
                    return False

                game.effect_reveal_and_select_multi(
                    player, 3,
                    [(reveal_filter_shoto, 'hand'), (reveal_filter_avian, 'hand')],
                    remaining_placement='deck_bottom',
                    is_optional=False,
                )

            game.request_selection(
                GamePhase.SelectTarget, player, on_select, valid,
                is_optional=True,
                prompt="You may suspend 1 level 6 or lower Digimon (own or opponent)."
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndBattle
        # [All Turns] [Once Per Turn] When this Digimon deletes your opponent's
        # Digimon in battle, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndBattle)
        effect1.set_effect_name("BT24-044 Memory +1")
        effect1.set_effect_description(
            "[All Turns] [Once Per Turn] When this Digimon deletes your opponent's "
            "Digimon in battle, gain 1 memory."
        )
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BT24_044_Inherited")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
