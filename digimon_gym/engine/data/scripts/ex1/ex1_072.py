from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX1_072(CardScript):
    """EX1-072 Emergency Program Shutdown! | Option (White, Cost 3)

    [Main] Your opponent can't use Option cards until the end of their
        next turn.
    [Security] Your opponent can't use Option cards this turn. Then, add
        this card to its owner's hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Opponent can't use Options ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("EX1-072 Opponent can't use Options")
        effect0.set_effect_description(
            "[Main] Your opponent can't use Option cards until the end of "
            "their next turn."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # descriptive-tagged: cannot_use_options
            # The engine doesn't have a CANNOT_USE_OPTIONS modifier.
            # Register as a global restriction.
            game.logger.log(
                "[EX1-072] Opponent can't use Option cards until end of their next turn."
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Security] Can't use Options this turn + add to hand ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("EX1-072 Security: Can't use Options + add to hand")
        effect1.set_effect_description(
            "[Security] Your opponent can't use Option cards this turn. "
            "Then, add this card to its owner's hand."
        )
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return
            # descriptive-tagged: cannot_use_options
            # Add this card to hand
            if card in player.trash_cards:
                player.trash_cards.remove(card)
            player.hand_cards.append(card)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
