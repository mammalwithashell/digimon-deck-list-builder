from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_039(CardScript):
    """EX6-039 Kurisarimon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played from the hand, by deleting 1 of your Digimon with the [Unidentified] trait, reduce the play cost by 3.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-039 Delete 1 digimon with [Unidentified] trait, to get Play Cost -3")
        effect0.set_effect_description("When this card would be played from the hand, by deleting 1 of your Digimon with the [Unidentified] trait, reduce the play cost by 3.")
        effect0.set_hash_string("PlayCost-12_BT5_085")
        effect0.cost_reduction = 3

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Cost -3, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 3 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Delete 1 of your opponent's Digimon with a play cost of 3 or less.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-039 Delete 1 opponents Digimon, with 3 cost or less")
        effect1.set_effect_description("[On Play] Delete 1 of your opponent's Digimon with a play cost of 3 or less.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete"""
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

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Delete 1 of your opponent's Digimon with a play cost of 3 or less.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX6-039 Delete 1 opponents Digimon, with 3 cost or less")
        effect2.set_effect_description("[When Digivolving] Delete 1 of your opponent's Digimon with a play cost of 3 or less.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete"""
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

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] If this card has the [Unidentified] trait, you may play 1 [Diaboromon] (Digimon | 14 Cost | Level 6 | White | Mega | Unknown | Unidentified | DP3000) Token without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX6-039 Play 1 Diaboromon Token")
        effect3.set_effect_description("[On Deletion] If this card has the [Unidentified] trait, you may play 1 [Diaboromon] (Digimon | 14 Cost | Level 6 | White | Mega | Unknown | Unidentified | DP3000) Token without paying the cost.")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.is_on_deletion = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Token"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Play Diaboromon Token — token play not yet supported in engine
            pass  # descriptive-tagged: play_token

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
