from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT7_045(CardScript):
    """BT7-045 Tortomon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Inherited Effect: [When Attacking] You may reveal 1 green Digimon card
        # from your hand and place it on top of your deck to have this Digimon
        # get +3000 DP for the turn.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDeclaration)
        effect0.is_on_attack = True
        effect0.is_inherited_effect = True
        effect0.set_effect_name("BT7-045 Inherited: Reveal green Digimon for +3000 DP")
        effect0.set_effect_description(
            "[When Attacking] You may reveal 1 green Digimon card from your "
            "hand and place it on top of your deck to have this Digimon get "
            "+3000 DP for the turn."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            from ....data.enums import CardColor
            # Must have a green Digimon in hand
            return any(
                getattr(c, 'is_digimon', False)
                and CardColor.Green in (getattr(c, 'card_colors', []) or [])
                for c in owner.hand_cards
            )

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            perm = card.permanent_of_this_card() if card else None
            if not (player and game and perm):
                return

            from ....data.enums import CardColor

            def hand_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = getattr(c, 'card_colors', []) or []
                return CardColor.Green in colors

            def on_selected(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    # Place on top of deck
                    player.library_cards.insert(0, selected)
                    # Grant +3000 DP for the turn
                    from ....interfaces.modifiers import ModifierType
                    game.register_modifier(
                        perm, ModifierType.CHANGE_DP,
                        value_fn=lambda current, p, c: current + 3000,
                        expiry='end_of_turn',
                    )

            game.effect_select_hand_card(
                player, hand_filter, on_selected,
                is_optional=True,
                prompt="You may reveal 1 green Digimon card from hand and place it on top of deck for +3000 DP."
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
