from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_090(CardScript):
    """BT22-090 Rie Kishibe"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: gain_memory_tamer
        # Gain 1 memory (Tamer)
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-090 Gain 1 memory (Tamer)")
        effect0.set_effect_description("Gain 1 memory (Tamer)")
        # [Start of Main] Gain 1 memory if opponent has Digimon

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] [Once Per Turn] By deleting 1 of your other Digimon or Tamers with [Knightmon] in its text or the [CS] trait, this Tamer may digivolve into [LordKnightmon] in the hand with the digivolution cost reduced by 3.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-090 Delete 1 digimon or tamer [Knightmon] in text/[CS] trait, Digivolve into [LordKnightmon]")
        effect1.set_effect_description("[End of Your Turn] [Once Per Turn] By deleting 1 of your other Digimon or Tamers with [Knightmon] in its text or the [CS] trait, this Tamer may digivolve into [LordKnightmon] in the hand with the digivolution cost reduced by 3.")
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BT22_090_Digivolve")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Knightmon' in text):
                    return False
            else:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
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

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-090 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
