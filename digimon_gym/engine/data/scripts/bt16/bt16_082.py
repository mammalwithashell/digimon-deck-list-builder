from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_082(CardScript):
    """BT16-082 Ukkomon | Lv.3 Digimon"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Your Turn][Once Per Turn] When one of your Digimon moves from
        #    breeding area to battle area, reveal top 3, add 1 Digimon or Tamer to hand,
        #    return rest to bottom. Then, you may hatch. ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnMove)
        effect0.set_effect_name(
            "BT16-082 Reveal top 3, add 1 Digimon/Tamer to hand, may hatch"
        )
        effect0.set_effect_description(
            "[Your Turn][Once Per Turn] When one of your Digimon moves from the breeding "
            "area to the battle area, reveal the top 3 cards of your deck. Add 1 Digimon "
            "card or Tamer card among them to the hand. Return the rest to the bottom of "
            "the deck. Then, you may hatch in your breeding area."
        )
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("BT16_082_RevealHatch")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Event must be owner's Digimon moving (from breeding to battle area)
            event_perm = context.get('event_permanent')
            if event_perm is None:
                return False
            event_player = context.get('event_player')
            if event_player is None:
                return False
            if event_player is not card.owner:
                return False
            if not event_perm.is_digimon:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Reveal top 3, pick 1 Digimon or Tamer card to hand, rest to deck bottom
            def reveal_filter(c):
                return getattr(c, 'is_digimon', False) or getattr(c, 'is_tamer', False)

            def on_selected(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)

            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_selected, is_optional=True
            )
            # You may hatch in your breeding area
            if player.digitama_library_cards and player.breeding_area is None:
                player.hatch()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
