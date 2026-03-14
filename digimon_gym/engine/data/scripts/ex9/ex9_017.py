from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_017(CardScript):
    """EX9-017 Garurumon | Lv.4 | Beast/DM/Ver.2
    <Training>
    [On Play][When Digivolving] By placing 1 hand card face down as bottom digi
    card, for each face-down digi card, trash 1 digi card from opponent's Digimon.
    Inherited: <Jamming>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Training>
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-017 Training")
        effect0.set_effect_description("<Training>")
        effect0._is_training = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [On Play] Place hand card, trash opponent's digi cards
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX9-017 On Play: Place hand, strip opponent sources")
        effect1.set_effect_description(
            "[On Play] By placing 1 hand card as bottom digi card, for each "
            "face-down digi card, trash 1 digi card from opponent's Digimon."
        )
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def _place_and_strip(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            if player.hand_cards:
                placed = player.hand_cards.pop()
                perm.add_card_source_bottom(placed)
            # Count face-down digi cards (approximate as all digi cards)
            face_down_count = len(perm.digivolution_cards)
            # Trash that many digi cards from opponent's Digimon
            for _ in range(face_down_count):
                def target_filter(p):
                    return p.is_digimon and len(p.digivolution_cards) > 0

                def on_strip(target_perm):
                    if target_perm.digivolution_cards:
                        removed = target_perm.digivolution_cards.pop()
                        enemy.trash_cards.append(removed)

                game.effect_select_opponent_permanent(
                    player, on_strip, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(_place_and_strip)
        effects.append(effect1)

        # [When Digivolving] Same effect
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX9-017 When Digivolving: Place hand, strip opponent sources")
        effect2.set_effect_description(
            "[When Digivolving] By placing 1 hand card as bottom digi card, "
            "for each face-down digi card, trash 1 digi card from opponent's Digimon."
        )
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_place_and_strip)
        effects.append(effect2)

        # Inherited: <Jamming>
        effect3 = ICardEffect()
        effect3.set_effect_name("EX9-017 Inherited: Jamming")
        effect3.set_effect_description("Inherited: <Jamming>")
        effect3.is_inherited_effect = True
        effect3._is_jamming = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
