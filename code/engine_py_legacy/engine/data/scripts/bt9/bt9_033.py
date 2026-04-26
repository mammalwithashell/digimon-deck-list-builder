from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT9_033(CardScript):
    """BT9-033 Pillomon | Lv.3 Yellow Digimon | DP 2000 | Cost 3

    [All Turns] Players can't play Digimon by effects.

    Implemented via CANNOT_PLAY_BY_EFFECT modifier registered when Pillomon
    enters the field. This blocks effect-based Digimon plays (e.g., play from
    trash) while allowing normal hand plays from the Main phase.
    Auto-cleaned when Pillomon leaves field via cleanup_modifiers.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [All Turns] Players can't play Digimon by effects ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT9-033 Players can't play Digimon by effects")
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

            # Register CANNOT_PLAY_BY_EFFECT for Digimon (and Digi-Eggs per C#).
            # Blocks effect-based plays; normal hand plays are unaffected.
            # C# CardCondition: cardSource.IsDigimon || cardSource.IsDigiEgg
            def _block_condition(target, c):
                card_in_ctx = c.get('card')
                if card_in_ctx is None:
                    return False
                return bool(card_in_ctx.is_digimon or card_in_ctx.is_digi_egg)

            game.register_modifier(
                perm,
                ModifierType.CANNOT_PLAY_BY_EFFECT,
                condition=_block_condition,
                source_effect=effect0,
                expiry='permanent',
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
