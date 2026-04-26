from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_171(CardScript):
    """P-171 Pukumon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("P-171 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [DS] trait for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_trait = "DS"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('DS' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Jogress Condition
        effect1 = ICardEffect()
        effect1.set_effect_name("P-171 Jogress Condition")
        effect1.set_effect_description("Jogress Condition")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played, if [Deep Savers] is in your face up security cards, reduce the play cost by 4.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.BeforePayCost)
        effect2.set_effect_name("P-171 If [Deep Savers] is in your face up security cards get Play Cost -4")
        effect2.set_effect_description("When this card would be played, if [Deep Savers] is in your face up security cards, reduce the play cost by 4.")
        effect2.set_hash_string("PlayCost-4_P_171")
        effect2.cost_reduction = 4

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
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
        # Cost -4
        effect3 = ICardEffect()
        effect3.set_effect_name("P-171 Play Cost -1")
        effect3.set_effect_description("Cost -4")
        effect3.cost_reduction = 4

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Cost -4"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 4 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Factory effect: blocker
        # Blocker
        effect4 = ICardEffect()
        effect4.set_effect_name("P-171 Blocker")
        effect4.set_effect_description("Blocker")
        effect4._is_blocker = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Trash the top 2 digivolution cards of all of your opponent's Digimon. Then, delete 1 of their Digimon with no digivolution cards.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect5.set_effect_name("P-171 Trash digivolution cards and destory 1 Digimon without digivolution cards")
        effect5.set_effect_description("[On Play] Trash the top 2 digivolution cards of all of your opponent's Digimon. Then, delete 1 of their Digimon with no digivolution cards.")
        effect5.is_on_play = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Delete, Trash Digivolution Cards"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Trash the top 2 digivolution cards of all of your opponent's Digimon. Then, delete 1 of their Digimon with no digivolution cards.
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect6.set_effect_name("P-171 Trash digivolution cards and destory 1 Digimon without digivolution cards")
        effect6.set_effect_description("[When Digivolving] Trash the top 2 digivolution cards of all of your opponent's Digimon. Then, delete 1 of their Digimon with no digivolution cards.")
        effect6.is_when_digivolving = True

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Delete, Trash Digivolution Cards"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
