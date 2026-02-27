from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_085(CardScript):
    """BT14-085 Mimi Tachikawa"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with
        # [Vegetation], [Plant] or [Fairy] in one of its traits among them to the hand.
        # Return the rest to the bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-085 Reveal the top 3 cards of deck")
        effect0.set_effect_description("[On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with [Vegetation], [Plant] or [Fairy] in one of its traits among them to the hand. Return the rest to the bottom of the deck.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                for t in traits:
                    if 'Vegetation' in t or 'Plant' in t or 'Fairy' in t:
                        return True
                return False

            def on_revealed(selected, remaining):
                if selected is not None:
                    player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)

            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=True
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Your Turn] When an effect suspends a Digimon, by suspending this Tamer, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-085 Memory +1")
        effect1.set_effect_description("[Your Turn] When an effect suspends a Digimon, by suspending this Tamer, gain 1 memory.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            suspended_by_effect = context.get('suspended_by_effect', False)
            suspended_target = context.get('suspended_target')
            if suspended_by_effect:
                return True
            if suspended_target is not None:
                return True
            return False

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if not player:
                return
            if perm is not None:
                perm.suspend()
            player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-085 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
