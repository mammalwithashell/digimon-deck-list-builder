from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX1_071(CardScript):
    """EX1-071 Win Rate: 60%! | Option (Yellow, Cost 2)

    While you have a Tamer, you can ignore this card's color requirements.
    [Main] The next time one of your Digimon would digivolve this turn,
    you may trash 1 Digimon card in your hand of the same color as the
    digivolving Digimon to reduce the memory cost of the digivolution by 4.
    [Security] Add this card to the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Ignore color requirements while you have a Tamer (handled by engine)
        effect0 = ICardEffect()
        effect0.set_effect_name("EX1-071 Ignore color requirements")
        effect0.set_effect_description(
            "While you have a Tamer, you can ignore this card's color requirements."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [Main] Reduce next digivolve cost by 4
        # Approximation: Register a CHANGE_DIGIVOLUTION_COST modifier
        # that reduces cost by 4 for all Digimon for the rest of the turn.
        # The "trash 1 Digimon card of same color" cost is hard to model
        # as a conditional cost on a future event, so we apply the reduction
        # directly and skip the trash cost (engine limitation).
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("EX1-071 Reduce next digi cost by 4")
        effect1.set_effect_description(
            "[Main] The next time one of your Digimon would digivolve this "
            "turn, you may trash 1 Digimon card in your hand of the same "
            "color as the digivolving Digimon to reduce the memory cost of "
            "the digivolution by 4."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Register digivolution cost reduction for all Digimon
            for field_perm in player.battle_area:
                if field_perm.is_digimon:
                    game.register_modifier(
                        field_perm, ModifierType.CHANGE_DIGIVOLUTION_COST,
                        value_fn=lambda current, target, c: current - 4,
                        expiry='end_of_turn',
                    )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Security] Add this card to the hand.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("EX1-071 Add to hand")
        effect2.set_effect_description("[Security] Add this card to the hand.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                player.hand_cards.append(card)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
