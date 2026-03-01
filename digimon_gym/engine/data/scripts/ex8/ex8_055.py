from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_055(CardScript):
    """EX8-055 Pyramidimon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: fragment
        # Fragment
        effect0 = ICardEffect()
        effect0.set_effect_name("EX8-055 Fragment")
        effect0.set_effect_description("Fragment")
        effect0._is_fragment = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By trashing any 3 [Mineral] or [Rock] trait cards from any of your Digimon's digivolution cards, this Digimon unsuspends, and it gains <Security A. +1> for the turn.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX8-055 By trashing 3 sources, Unsuspend and gain Security A. +1")
        effect1.set_effect_description("[When Digivolving] By trashing any 3 [Mineral] or [Rock] trait cards from any of your Digimon's digivolution cards, this Digimon unsuspends, and it gains <Security A. +1> for the turn.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards, Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] By trashing any 3 [Mineral] or [Rock] trait cards from any of your Digimon's digivolution cards, this Digimon unsuspends, and it gains <Security A. +1> for the turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnAllyAttack)
        effect2.set_effect_name("EX8-055 By trashing 3 sources, Unsuspend and gain Security A. +1")
        effect2.set_effect_description("[When Attacking] By trashing any 3 [Mineral] or [Rock] trait cards from any of your Digimon's digivolution cards, this Digimon unsuspends, and it gains <Security A. +1> for the turn.")
        effect2.is_optional = True
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards, Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] [Once Per Turn] You may place up to 3 [Mineral] or [Rock] cards from your trash as this Digimon's bottom digivolution cards.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEndTurn)
        effect3.set_effect_name("EX8-055 Place 3 cards from trash as bottom sources")
        effect3.set_effect_description("[End of Your Turn] [Once Per Turn] You may place up to 3 [Mineral] or [Rock] cards from your trash as this Digimon's bottom digivolution cards.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("EOT_EX8_055")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
