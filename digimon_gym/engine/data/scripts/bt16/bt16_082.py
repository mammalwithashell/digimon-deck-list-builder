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
            # OnMove context key is 'moved_permanent' (the Digimon that moved
            # from breeding to battle area).  Must be owner's Digimon.
            moved_perm = context.get('moved_permanent')
            if moved_perm is None:
                return False
            if not moved_perm.is_digimon:
                return False
            # Check the moved Digimon belongs to the same owner
            if moved_perm not in (card.owner.battle_area or []):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def _maybe_hatch():
                """Then, you may hatch in your breeding area (optional)."""
                if player.digitama_library_cards and player.breeding_area is None:
                    def on_hatch_choice(choice_idx):
                        if choice_idx == 0:
                            player.hatch()
                    game.effect_choose_branch(
                        player, 2, on_hatch_choice,
                        prompt="Hatch in your breeding area?",
                        branch_labels=["Yes, hatch", "No, skip"],
                    )

            # Reveal top 3, pick 1 Digimon or Tamer card to hand, rest to deck bottom
            def reveal_filter(c):
                return getattr(c, 'is_digimon', False) or getattr(c, 'is_tamer', False)

            def on_selected(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
                # Chain the hatch choice after reveal selection resolves
                _maybe_hatch()

            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_selected, is_optional=False
            )
            # If no selection was created (empty deck or no valid cards),
            # the hatch choice can proceed synchronously.
            if game.pending_selection is None:
                _maybe_hatch()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
