from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT18_007(CardScript):
    """BT18-007 Gazimon | Lv.3 Red/Purple Mammal"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] Reveal top 3, add 1 [Millenniummon] name card
        #     and 1 [Composite] or [Wicked God] trait card to hand ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT18-007 Reveal top 3, add Millenniummon + Composite/Wicked God")
        effect0.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 card with "
            "[Millenniummon] in its name and 1 card with the [Composite] or "
            "[Wicked God] trait among them to the hand. Return the rest to "
            "the bottom of the deck."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Reveal top 3, add Millenniummon + Composite/Wicked God"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Reveal top 3
            revealed = []
            for _ in range(min(3, len(player.library_cards))):
                revealed.append(player.library_cards.pop(0))

            added = []
            # Add 1 card with [Millenniummon] in its name
            for c in revealed:
                if c not in added and c.contains_card_name('Millenniummon'):
                    added.append(c)
                    break
            # Add 1 card with [Composite] or [Wicked God] trait
            for c in revealed:
                if c not in added:
                    traits = getattr(c, 'card_traits', [])
                    if 'Composite' in traits or 'Wicked God' in traits:
                        added.append(c)
                        break

            for c in added:
                player.hand_cards.append(c)
            # Remaining go to bottom of deck
            remaining = [c for c in revealed if c not in added]
            for c in remaining:
                player.library_cards.append(c)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Inherited <Retaliation> ---
        effect1 = ICardEffect()
        effect1.set_effect_name("BT18-007 Retaliation (Inherited)")
        effect1.set_effect_description("Inherited: <Retaliation>")
        effect1.is_inherited_effect = True
        effect1.is_on_deletion = True
        effect1._is_retaliation = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
