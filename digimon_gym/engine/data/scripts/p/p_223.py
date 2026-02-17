from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_223(CardScript):
    """P-223 Kuzuhamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("P-223 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 1
        effect0._alt_digi_cost = 1

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played from the hand, if you have 3 or fewer security, reduce the play cost by 4.
        effect1 = ICardEffect()
        effect1.set_effect_name("P-223 Play cost reduction -4")
        effect1.set_effect_description("When this card would be played from the hand, if you have 3 or fewer security, reduce the play cost by 4.")
        effect1.set_hash_string("P_223_ReducePlayCost")
        effect1.cost_reduction = 4

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Cost -4"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 4 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.None
        # Cost -4
        effect2 = ICardEffect()
        effect2.set_effect_name("P-223 Play Cost -4")
        effect2.set_effect_description("Cost -4")
        effect2.cost_reduction = 4

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Cost -4"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 4 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.None
        # Also Treated As
        effect3 = ICardEffect()
        effect3.set_effect_name("P-223 Also treated as [Sakuyamon]")
        effect3.set_effect_description("Also Treated As")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Also Treated As"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Also treated as [Name] — name aliasing not modeled in engine
            pass  # descriptive-tagged: also_treated_as_name

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Trash From Hand
        effect4 = ICardEffect()
        effect4.set_effect_name("P-223 You may use an [Onmyōjutsu] or [Plug-In] Option from hand or trash")
        effect4.set_effect_description("Trash From Hand")
        effect4.is_on_play = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Trash From Hand
        effect5 = ICardEffect()
        effect5.set_effect_name("P-223 You may use an [Onmyōjutsu] or [Plug-In] Option from hand or trash")
        effect5.set_effect_description("Trash From Hand")
        effect5.is_when_digivolving = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.OnUseOption
        # [All Turns][Once Per Turn] When you use Option cards, you may play 1 [Pipe Fox] Token (Digimon/Yellow/6000 DP/<Blocker>).
        effect6 = ICardEffect()
        effect6.set_effect_name("P-223 Play a [Pipe Fox] Token")
        effect6.set_effect_description("[All Turns][Once Per Turn] When you use Option cards, you may play 1 [Pipe Fox] Token (Digimon/Yellow/6000 DP/<Blocker>).")
        effect6.is_optional = True
        effect6.set_max_count_per_turn(1)
        effect6.set_hash_string("PlayToken_P_223")

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect6.set_can_use_condition(condition6)
        effects.append(effect6)

        return effects
