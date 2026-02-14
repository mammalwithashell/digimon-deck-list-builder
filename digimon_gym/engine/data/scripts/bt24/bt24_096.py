from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_096(CardScript):
    """BT24-096 Seventh Graviton"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Trash] [Your Turn] When any of your Digimon digivolve into [Creepymon (X Antibody)], by returning this card to the bottom of the deck, activate this card's [Main] effects.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-096 By returning this card to deck, Activate [Main] effect")
        effect0.set_effect_description("[Trash] [Your Turn] When any of your Digimon digivolve into [Creepymon (X Antibody)], by returning this card to the bottom of the deck, activate this card's [Main] effects.")
        effect0.is_optional = True
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Creepymon (X Antibody)'))):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Return To Deck"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_return(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)
            game.effect_select_opponent_permanent(
                player, on_return, filter_fn=target_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] Delete 1 of your opponent's level 6 or higher Digimon. If this effect didn't delete, trash the top 3 cards of your opponent's deck.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-096 Delete 1 level 6 or higher. If not trash opponent's top 3 deck")
        effect1.set_effect_description("[Main] Delete 1 of your opponent's level 6 or higher Digimon. If this effect didn't delete, trash the top 3 cards of your opponent's deck.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
