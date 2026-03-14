from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_004(CardScript):
    """EX10-004 Cupimon | Lv.2

    Inherited Effect [Your Turn] [Once Per Turn] When any of your Digimon
        with [Lucemon] in their names move from the breeding area to the
        battle area, by trashing 1 card in your hand, Draw 1 and gain 1 memory.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnMove)
        effect0.set_effect_name("EX10-004 Lucemon moves: trash hand, draw 1, +1 memory")
        effect0.set_effect_description(
            "[Your Turn] [Once Per Turn] When any of your Digimon with [Lucemon] "
            "in their names move from the breeding area to the battle area, by "
            "trashing 1 card in your hand, Draw 1 and gain 1 memory."
        )
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("ESS_EX10-004")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check that the moved Digimon has [Lucemon] in its name
            moved_perm = context.get('moved_permanent')
            if moved_perm:
                if not moved_perm.contains_card_name('Lucemon'):
                    return False
            player = card.owner if card else None
            if not player or not player.hand_cards:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Cost: trash 1 card in hand (any card)
            def hand_filter(c):
                return True

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
                # Draw 1
                player.draw_cards(1)
                # Gain 1 memory
                player.add_memory(1)

            if player.hand_cards:
                game.effect_select_hand_card(
                    player, hand_filter, on_trashed, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
