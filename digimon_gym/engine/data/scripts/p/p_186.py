from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_186(CardScript):
    """P-186 Gallantmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("P-186 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 WarGrowlmon for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_name = "WarGrowlmon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("P-186 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: rush
        # Rush
        effect2 = ICardEffect()
        effect2.set_effect_name("P-186 Rush")
        effect2.set_effect_description("Rush")
        effect2._is_rush = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played, if there is a Digimon with 13000 DP or more,
        # reduce the play cost by 2 for every 5 total cards in both players' trashes.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.BeforePayCost)
        effect3.set_effect_name("P-186 Reduce play cost by 2 per 5 trash cards")
        effect3.set_effect_description("When this card would be played, if there is a Digimon with 13000 DP or more, reduce the play cost by 2 for every 5 total cards in both players' trashes.")

        def condition3(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            enemy = owner.enemy if owner else None
            # Must have a Digimon with 13000+ DP on either side
            has_big = any(p.is_digimon and p.dp is not None and p.dp >= 13000 for p in owner.battle_area)
            if not has_big and enemy:
                has_big = any(p.is_digimon and p.dp is not None and p.dp >= 13000 for p in enemy.battle_area)
            return has_big

        effect3.set_can_use_condition(condition3)
        effect3._cost_reduction_value_fn = (
            lambda context: 2 * ((len(getattr(card, 'owner', None).trash_cards if getattr(card, 'owner', None) else []) +
                                   len(getattr(card, 'owner', None).enemy.trash_cards if (getattr(card, 'owner', None) and getattr(card, 'owner', None).enemy) else [])) // 5)
        )
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Delete 1 Digimon with 13000 DP or more. If this effect didn't delete, <Recovery +1 (Deck)> (Place the top card of your deck on top of your security stack).
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("P-186 Delete a digimon, if you didnt <Recovery +1 (Deck)>")
        effect4.set_effect_description("[On Play] Delete 1 Digimon with 13000 DP or more. If this effect didn't delete, <Recovery +1 (Deck)> (Place the top card of your deck on top of your security stack).")
        effect4.is_on_play = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def _p186_delete_or_recovery(player, game):
            """Delete 1 Digimon (either field) with 13000+ DP. If none deleted, Recovery +1."""
            from ....game.constants import (
                SEL_MY_FIELD_START, SEL_OPP_FIELD_START, FIELD_SLOTS,
            )
            from ....data.enums import GamePhase as GP

            enemy = player.enemy if player else None
            high_filter = lambda p: p.is_digimon and p.dp is not None and p.dp >= 13000

            # Build combined valid indices from both fields
            valid = []
            for i, perm in enumerate(player.battle_area):
                if game.modifiers.can_be_selected_by_effect(perm) and high_filter(perm):
                    valid.append(SEL_MY_FIELD_START + i)
            if enemy:
                for i, perm in enumerate(enemy.battle_area):
                    if game.modifiers.can_be_selected_by_effect(perm) and high_filter(perm):
                        valid.append(SEL_OPP_FIELD_START + i)

            if not valid:
                # No eligible targets — Recovery +1
                player.recovery(1)
                return

            def on_select(action_id: int):
                deleted = False
                if SEL_MY_FIELD_START <= action_id < SEL_MY_FIELD_START + FIELD_SLOTS:
                    idx = action_id - SEL_MY_FIELD_START
                    if 0 <= idx < len(player.battle_area):
                        target = player.battle_area[idx]
                        deleted = player.delete_permanent(target)
                elif SEL_OPP_FIELD_START <= action_id < SEL_OPP_FIELD_START + FIELD_SLOTS:
                    idx = action_id - SEL_OPP_FIELD_START
                    opp = game.player2 if player is game.player1 else game.player1
                    if 0 <= idx < len(opp.battle_area):
                        target = opp.battle_area[idx]
                        deleted = opp.delete_permanent(target, is_opponent_effect=True)
                if not deleted:
                    # Deletion was prevented — Recovery +1
                    player.recovery(1)

            game.request_selection(
                GP.SelectTarget, player, on_select, valid,
                is_optional=False,
                prompt="Delete 1 Digimon with 13000 DP or more."
            )

        def process4(ctx: Dict[str, Any]):
            """Action: Delete 13000+ DP or Recovery +1"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                _p186_delete_or_recovery(player, game)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Delete 1 Digimon with 13000 DP or more. If this effect didn't delete, <Recovery +1 (Deck)>.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect5.set_effect_name("P-186 Delete a digimon, if you didnt <Recovery +1 (Deck)>")
        effect5.set_effect_description("[When Digivolving] Delete 1 Digimon with 13000 DP or more. If this effect didn't delete, <Recovery +1 (Deck)>.")
        effect5.is_when_digivolving = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Delete 13000+ DP or Recovery +1"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                _p186_delete_or_recovery(player, game)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
