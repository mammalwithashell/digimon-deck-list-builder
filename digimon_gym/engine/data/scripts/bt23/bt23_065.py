from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_065(CardScript):
    """BT23-065"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDeclaration
        # [Hand] [Main] If you have [Violet Inboots], by placing 1 [Bakemon] from your trash as any of your [Ghostmon]'s bottom digivolution card, it digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-065 Place 1 [Bakemon] from trash under 1 [Ghostmon], to digivolve for 3")
        effect0.set_effect_description("[Hand] [Main] If you have [Violet Inboots], by placing 1 [Bakemon] from your trash as any of your [Ghostmon]'s bottom digivolution card, it digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Violet Inboots') or permanent.contains_card_name('Ghostmon'))):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-065 Play 1 level 4 [Ghost] trait from trash")
        effect1.set_effect_description("Effect")
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-065 Play 1 level 4 [Ghost] trait from trash")
        effect2.set_effect_description("Effect")
        effect2.is_inherited_effect = True
        effect2.is_on_deletion = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
