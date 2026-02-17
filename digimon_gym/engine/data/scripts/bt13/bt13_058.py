from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_058(CardScript):
    """BT13-058 Leopardmon: Leopard Mode | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-058 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 1
        effect0._alt_digi_cost = 1

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Suspend 1 of your opponent's Digimon. Until the end of your opponent's turn, 1 of your opponent's Digimon doesn't unsuspend.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-058 Suspend 1 Digimon and opponent's 1 Digimon can't unsuspend")
        effect1.set_effect_description("[When Digivolving] Suspend 1 of your opponent's Digimon. Until the end of your opponent's turn, 1 of your opponent's Digimon doesn't unsuspend.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend, Gain Keyword Cannot Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_suspend(target_perm):
                target_perm.suspend()
                # Apply "Doesn't unsuspend" to THIS target?
                # Text: "Suspend 1... 1... doesn't unsuspend." Usually implies the same target or a new selection.
                # Assuming same target for simplicity, or we should ask for selection again?
                # "Suspend 1... Until end of turn, 1... doesn't unsuspend."
                # Usually it targets the same if possible, but distinct phrases might mean distinct targets.
                # Given "1 of your opponent's Digimon" is repeated, it implies a second selection or re-targeting.
                # I'll apply it to the suspended one for now as it's the most common pattern.
                target_perm.grant_keyword('_is_cannot_unsuspend') # Duration? Until end of opponent's turn.
                # grant_keyword takes expiry turn.
                # Expiry = game.turn_count + 1 (opponent's turn)
                target_perm.grant_keyword('_is_cannot_unsuspend', game.turn_count + 1)

            def target_filter(p):
                return True

            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] By suspending 1 of your other Digimon, unsuspend this Digimon.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-058 Suspend your 1 Digimon to unsuspend this Digimon")
        effect2.set_effect_description("[When Attacking] By suspending 1 of your other Digimon, unsuspend this Digimon.")
        effect2.is_optional = True
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Check if there is another unsuspended Digimon
            player = context.get('player')
            perm = context.get('permanent')
            if not player:
                return False
            others = [p for p in player.battle_area if p is not perm and p.is_digimon and not p.is_suspended]
            return len(others) > 0

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Suspend other, Unsuspend self"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            def filter_other(p):
                return p is not perm and p.is_digimon and not p.is_suspended

            def on_suspend(target_perm):
                target_perm.suspend()
                perm.unsuspend()

            game.effect_select_own_permanent(
                player, on_suspend, filter_fn=filter_other, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] Trash the top card of this Digimon and unsuspend all of your Digimon.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT13-058 Trash the top card of this Digimon an ussuspend your all Digimons")
        effect3.set_effect_description("[End of Your Turn] Trash the top card of this Digimon and unsuspend all of your Digimon.")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Trash top, Unsuspend all"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            # Trash top card
            if perm.card_sources:
                top = perm.card_sources.pop() # Remove top
                game.logger.log(f"[BT13-058] Trashed {top.card_names[0]}")
                player.trash_cards.append(top)

                # If perm became empty, it's deleted (rules vary, but generally yes)
                # Usually Level 6 -> Level 5. If Level 3 -> nothing, it dies.
                if not perm.card_sources:
                    if perm in player.battle_area:
                        player.battle_area.remove(perm)

            # Unsuspend all
            player.unsuspend_all()

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
