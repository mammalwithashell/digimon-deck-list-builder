from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_040(CardScript):
    """BT24-040 Venusmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-040 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played, if you have 3 or fewer security cards, reduce the play cost by 5.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-040 Reduce play cost (5)")
        effect1.set_effect_description("When this card would be played, if you have 3 or fewer security cards, reduce the play cost by 5.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Effect"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction (variable amount) — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.None
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-040 Play Cost -5")
        effect2.set_effect_description("Effect")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Effect"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction (variable amount) — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Trash Digivolution Cards, Gain Keyword Cannot Suspend Player, Disable Effect
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-040 Trash Digivolution Cards, Gain Keyword Cannot Suspend Player, Disable Effect")
        effect3.set_effect_description("Trash Digivolution Cards, Gain Keyword Cannot Suspend Player, Disable Effect")
        effect3.is_on_play = True
        effect3._is_cannot_suspend_player = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards, Gain Keyword Cannot Suspend Player, Disable Effect"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            if perm:
                perm.grant_keyword('_is_cannot_suspend_player')
            # Disable/invalidate effects on target — not yet in engine
            pass  # descriptive-tagged: disable_effect

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Trash Digivolution Cards, Gain Keyword Cannot Suspend Player, Disable Effect
        effect4 = ICardEffect()
        effect4.set_effect_name("BT24-040 Trash Digivolution Cards, Gain Keyword Cannot Suspend Player, Disable Effect")
        effect4.set_effect_description("Trash Digivolution Cards, Gain Keyword Cannot Suspend Player, Disable Effect")
        effect4.is_when_digivolving = True
        effect4._is_cannot_suspend_player = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards, Gain Keyword Cannot Suspend Player, Disable Effect"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            if perm:
                perm.grant_keyword('_is_cannot_suspend_player')
            # Disable/invalidate effects on target — not yet in engine
            pass  # descriptive-tagged: disable_effect

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When any of your [TS] trait Digimon would leave the battle area other than by your effects, by placing 1 other Digimon with no digivolution cards as the bottom security card, they don't leave.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT24-040 By placing a sourceless Digimon to Security, your [TS] digimon won't leave the field")
        effect5.set_effect_description("[All Turns] [Once Per Turn] When any of your [TS] trait Digimon would leave the battle area other than by your effects, by placing 1 other Digimon with no digivolution cards as the bottom security card, they don't leave.")
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BT24_040_AT")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Put To Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Place a permanent into the security stack
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_put_security(target_perm):
                if player:
                    player.put_permanent_to_security(target_perm)
            game.effect_select_own_permanent(
                player, on_put_security, filter_fn=target_filter, is_optional=True)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
