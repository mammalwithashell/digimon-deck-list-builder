from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_085(CardScript):
    """BT15-085 Izzy Izumi"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: set_memory_3
        # Set memory to 3
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-085 Set memory to 3")
        effect0.set_effect_description("Set memory to 3")
        # [Start of Your Turn] Set memory to 3 if <= 2

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [Opponent's Turn] When an opponent's Digimon attacks, by suspending this Tamer, switch the target of attack to 1 of your suspended Digimon with the [Insectoid] trait.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT15-085 Switch Attack Target to your Digimon")
        effect1.set_effect_description("[Opponent's Turn] When an opponent's Digimon attacks, by suspending this Tamer, switch the target of attack to 1 of your suspended Digimon with the [Insectoid] trait.")
        effect1.is_optional = True
        effect1.is_on_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend, Redirect Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (any('Insectoid' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)
            # Redirect attack target (SwitchDefender) — not yet in engine
            pass  # descriptive-tagged: redirect_attack

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
