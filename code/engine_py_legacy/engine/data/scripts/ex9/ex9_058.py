from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_058(CardScript):
    """EX9-058 Gazimon | Lv.3 Purple Mammal/DM/Ver.5
    NOTE: EX9 set not yet in CardDatabase. Script ready for when data is added.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] Reveal top 3, add 1 [DM] trait card to hand,
        #     place 1 [Ver.5] trait card as bottom digi card of a [DM] Digimon,
        #     return rest to bottom of deck ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX9-058 Reveal top 3, add DM + place Ver.5")
        effect0.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Among them, add "
            "1 card with the [DM] trait to the hand and place 1 card with the "
            "[Ver.5] trait face down as the bottom digivolution card of any of "
            "your Digimon with the [DM] trait. Return the rest to the bottom "
            "of the deck."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Reveal top 3, add DM card, place Ver.5 as bottom evo"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Reveal top 3
            revealed = []
            for _ in range(min(3, len(player.library_cards))):
                revealed.append(player.library_cards.pop(0))

            added = []
            # Add 1 card with [DM] trait to hand
            for c in revealed:
                if c not in added:
                    traits = getattr(c, 'card_traits', [])
                    if 'DM' in traits:
                        added.append(c)
                        player.hand_cards.append(c)
                        break

            # Place 1 card with [Ver.5] trait as bottom digi card of a DM Digimon
            # NOTE: Placing cards as face-down bottom digivolution cards is
            # a Training-related mechanic. Partial implementation.
            placed = []
            for c in revealed:
                if c not in added and c not in placed:
                    traits = getattr(c, 'card_traits', [])
                    if 'Ver.5' in traits:
                        # Find a DM Digimon on our field
                        for perm in player.battle_area:
                            if perm.is_digimon and perm.has_trait('DM'):
                                perm.add_card_source_bottom(c)
                                placed.append(c)
                                break
                        break

            # Return rest to bottom of deck
            remaining = [c for c in revealed if c not in added and c not in placed]
            for c in remaining:
                player.library_cards.append(c)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Inherited <Retaliation> ---
        effect1 = ICardEffect()
        effect1.set_effect_name("EX9-058 Retaliation (Inherited)")
        effect1.set_effect_description("Inherited: <Retaliation>")
        effect1.is_inherited_effect = True
        effect1.is_on_deletion = True
        effect1._is_retaliation = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
