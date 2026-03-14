from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_009(CardScript):
    """EX9-009 Greymon | Lv.4 | Dinosaur/DM/Ver.1
    <Training>
    [When Attacking][Once Per Turn] By placing deck's top card face down as
    bottom digi card, get +1000 DP per face-down digi card until opponent's turn ends.
    Inherited: [Your Turn] This Digimon gets +2000 DP.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Training>
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-009 Training")
        effect0.set_effect_description("<Training>")
        effect0._is_training = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [When Attacking][Once Per Turn] Place top deck as bottom evo, +1000 DP per face-down
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnTappedAnyone)
        effect1.set_effect_name("EX9-009 When Attacking: Place deck top, +DP per face-down")
        effect1.set_effect_description(
            "[When Attacking][Once Per Turn] By placing your deck's top card "
            "face down as bottom digi card, get +1000 DP per face-down digi card."
        )
        effect1.is_on_attack = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("place_dp_EX9_009")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and not card.owner.library_cards:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm):
                return
            if player.library_cards:
                placed = player.library_cards.pop(0)
                perm.add_card_source_bottom(placed)
            # Count face-down digivolution cards
            face_down_count = len([s for s in perm.digivolution_cards
                                  if getattr(s, '_is_face_down', False)])
            # Fallback: count all bottom sources as face-down approximation
            if face_down_count == 0:
                face_down_count = len(perm.digivolution_cards)
            perm.dp_modifier += 1000 * face_down_count
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Inherited: [Your Turn] +2000 DP
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.NoTiming)
        effect2.set_effect_name("EX9-009 Inherited: +2000 DP your turn")
        effect2.set_effect_description("Inherited: [Your Turn] This Digimon gets +2000 DP.")
        effect2.is_inherited_effect = True
        effect2._dp_modifier = 2000

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
