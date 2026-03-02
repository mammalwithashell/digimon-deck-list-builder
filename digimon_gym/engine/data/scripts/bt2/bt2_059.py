from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT2_059(CardScript):
    """BT2-059 Kurisarimon | Lv.4 Champion | Black | Unidentified

    Inherited Effect: [Your Turn] When you play another Digimon with the
        same name as this Digimon, gain 1 memory.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Inherited [Your Turn] Gain 1 memory on same-name play ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT2-059 Inherited: Gain 1 memory on same-name play")
        effect0.set_effect_description(
            "[Your Turn] When you play another Digimon with the same name "
            "as this Digimon, gain 1 memory."
        )
        effect0.is_inherited_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check if the played Digimon has the same name as this Digimon's top card
            own_perm = card.permanent_of_this_card() if card else None
            trigger_perm = context.get('permanent')
            if own_perm and trigger_perm and trigger_perm != own_perm:
                if trigger_perm.top_card and own_perm.top_card:
                    trigger_name = trigger_perm.top_card.c_entity_base.card_name_eng if trigger_perm.top_card.c_entity_base else ''
                    own_name = own_perm.top_card.c_entity_base.card_name_eng if own_perm.top_card.c_entity_base else ''
                    if trigger_name and trigger_name == own_name:
                        return True
            return False
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Gain 1 memory."""
            game = ctx.get('game')
            if game:
                game.memory += 1
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
