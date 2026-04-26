from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT9_047(CardScript):
    """BT9-047 Pomumon | Lv.3, Green, Vegetation, DP 2000, Cost 3

    [All Turns] Players can't play Digimon by effects.

    Implemented via CANNOT_PLAY_BY_EFFECT modifier registered when Pomumon
    enters the field. This blocks effect-based Digimon plays (e.g., play from
    trash) while allowing normal hand plays from the Main phase.
    Auto-cleaned when Pomumon leaves field via cleanup_modifiers.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [All Turns] Players can't play Digimon by effects ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT9-047 Players can't play Digimon by effects")
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

            # Register CANNOT_PLAY_BY_EFFECT for Digimon only
            # Blocks effect-based plays; normal hand plays are unaffected
            game.register_modifier(
                perm,
                ModifierType.CANNOT_PLAY_BY_EFFECT,
                condition=lambda target, c: c.get('card') and c['card'].is_digimon,
                source_effect=effect0,
                expiry='permanent',
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
