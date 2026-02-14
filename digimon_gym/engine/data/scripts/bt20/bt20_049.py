from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_049(CardScript):
    """BT20-049 Blimpmon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your opponent's Digimon can't attack players until the end of their turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-049 1 of your opponent's Digimon can't attack players until the end of their turn")
        effect0.set_effect_description("[On Play] 1 of your opponent's Digimon can't attack players until the end of their turn.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Restrict Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Attack restriction — select opponent permanent to restrict
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_restrict(target_perm):
                target_perm.suspend()  # Approximate as suspend
            game.effect_select_opponent_permanent(
                player, on_restrict, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your opponent's Digimon can't attack players until the end of their turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-049 1 of your opponent's Digimon can't attack players until the end of their turn")
        effect1.set_effect_description("[When Digivolving] 1 of your opponent's Digimon can't attack players until the end of their turn.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Restrict Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Attack restriction — select opponent permanent to restrict
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_restrict(target_perm):
                target_perm.suspend()  # Approximate as suspend
            game.effect_select_opponent_permanent(
                player, on_restrict, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: reboot
        # Reboot
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-049 Reboot")
        effect2.set_effect_description("Reboot")
        effect2.is_inherited_effect = True
        effect2._is_reboot = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
