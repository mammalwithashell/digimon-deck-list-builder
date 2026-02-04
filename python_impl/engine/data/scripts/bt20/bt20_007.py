from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT20_007(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Main Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-007 Main Effect")
        effect1.set_effect_description("[Start of Your Main Phase] By trashing 1 card with [Dracomon] or [Examon] in its text in your hand, <Draw 1> and gain 1 memory.")

        def condition1(context: Dict[str, Any]) -> bool:
            timing = context.get("timing")
            # Filter for correct timing
            if timing != EffectTiming.OnStartMainPhase:
                return False

            player = context.get("player")
            if not player: return False
            if not player.is_my_turn: return False

            # Check cost availability
            for h_card in player.hand_cards:
                names = [n.lower() for n in h_card.card_names]
                if ("Dracomon" in h_card.card_traits or "Examon" in h_card.card_traits or
                    any("dracomon" in n for n in names) or any("examon" in n for n in names)):
                    return True
            return False

        def on_process1(context: Dict[str, Any]):
            game = context.get("game")
            player = context.get("player")
            if not game or not player: return

            # Pay Cost (Trash card)
            candidates = []
            for h_card in player.hand_cards:
                 names = [n.lower() for n in h_card.card_names]
                 if ("Dracomon" in h_card.card_traits or "Examon" in h_card.card_traits or
                     any("dracomon" in n for n in names) or any("examon" in n for n in names)):
                     candidates.append(h_card)

            if candidates:
                # Greedy selection: first valid card
                card_to_trash = candidates[0]
                if card_to_trash in player.hand_cards:
                    player.hand_cards.remove(card_to_trash)
                    player.trash_cards.append(card_to_trash)
                    print(f"BT20-007: Trashed {card_to_trash.card_names[0]}")

                    # Effect
                    player.draw()
                    game.memory += 1
                    print(f"BT20-007: +1 Memory (Now {game.memory})")

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(on_process1)
        effects.append(effect1)

        # Inherited Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-007 Inherited")
        effect2.set_effect_description("[Your Turn] This Digimon gets +2000 DP.")
        effect2.is_inherited_effect = True
        effect2.dp_modifier = 2000

        def condition2(context: Dict[str, Any]) -> bool:
            # DP modifiers are continuous, usually handled by Permanent.dp checking can_use_condition.
            # Permanent.dp implementation likely sets up context.
            # I assume context has "permanent".
            perm = context.get("permanent")
            if perm and perm.top_card and perm.top_card.owner:
                return perm.top_card.owner.is_my_turn
            return False

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
