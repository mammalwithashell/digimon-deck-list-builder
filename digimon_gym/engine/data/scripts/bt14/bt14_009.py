from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_009(CardScript):
    """BT14-009 Gotsumon | Lv.3 Red Rock Digimon

    [All Turns] Players can't play Digimon by effects.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns] Players can't play Digimon by effects.
        # Implemented by registering a CANNOT_PLAY_CARD modifier when this Digimon
        # enters the field (on play). The modifier persists while this Digimon is on
        # the field and is automatically cleaned up when it leaves.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT14-009 Players can't play Digimon by effects")
        effect0.set_effect_description(
            "[All Turns] Players can't play Digimon by effects."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            game = ctx.get('game')
            if not game:
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            # Register CANNOT_PLAY_CARD blocking Digimon cards for both players.
            # The condition receives (target_permanent, context_dict) and blocks
            # any play where the card being played is a Digimon.
            game.register_modifier(
                perm,
                ModifierType.CANNOT_PLAY_CARD,
                condition=lambda target, c: c.get('card') and c['card'].is_digimon,
                source_effect=effect0,
                expiry='permanent',
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
