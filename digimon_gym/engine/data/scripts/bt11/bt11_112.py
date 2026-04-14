from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_112(CardScript):
    """BT11-112 Rina Shinomiya"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Until the end of your opponent's turn, 1 of your Digimon with [Veemon] or [Veedramon] in its name gains <Blocker> and <Evade>.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT11-112 Your 1 Digimon gets Blocker and Evade")
        effect0.set_effect_description("[On Play] Until the end of your opponent's turn, 1 of your Digimon with [Veemon] or [Veedramon] in its name gains <Blocker> and <Evade>.")
        effect0.is_on_play = True
        effect0._is_blocker = True
        effect0._is_evade = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain Keyword Blocker, Gain Keyword Evade"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (p.contains_card_name('Veemon') or p.contains_card_name('Veedramon')):
                    return False
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_blocker')
                target_perm.grant_keyword('_is_evade')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] When one of your Digimon with [Veedramon] in its name becomes suspended, by suspending this Tamer, activate 1 of that Digimon's [When Digivolving] effects.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnTappedAnyone)
        effect1.set_effect_name("BT11-112 Activate [When Digivolving] effect")
        effect1.set_effect_description("[All Turns] When one of your Digimon with [Veedramon] in its name becomes suspended, by suspending this Tamer, activate 1 of that Digimon's [When Digivolving] effects.")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (p.contains_card_name('Veedramon')):
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnUnTappedAnyone
        # [Your Turn][Once Per Turn] When one of your blue Digimon becomes unsuspended, gain 1 memory.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUnTappedAnyone)
        effect2.set_effect_name("BT11-112 Memory +1")
        effect2.set_effect_description("[Your Turn][Once Per Turn] When one of your blue Digimon becomes unsuspended, gain 1 memory.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Memory+1_BT11_112")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must be one of owner's blue Digimon that unsuspended
            unsuspended_perm = context.get('event_permanent', context.get('permanent'))
            if unsuspended_perm is None:
                return False
            if not getattr(unsuspended_perm, 'is_digimon', False):
                return False
            if getattr(unsuspended_perm, 'owner', None) != (card.owner if card else None):
                return False
            top = getattr(unsuspended_perm, 'top_card', None)
            if top is None:
                return False
            colors = [col.name for col in (getattr(top, 'card_colors', []) or [])]
            if 'Blue' not in colors:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: security_play
        # Security: Play this card
        effect3 = ICardEffect()
        effect3.set_effect_name("BT11-112 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
