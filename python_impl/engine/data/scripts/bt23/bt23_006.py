from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT23_006(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # On Play
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-006 On Play")
        effect1.set_effect_description("[On Play] Reveal the top 3 cards of your deck. Add 1 card with [Huckmon] or [Sistermon] in its name and 1 card with the [Royal Knight] trait among them to the hand. Return the rest to the bottom of the deck.")
        effect1.is_on_play = True

        def on_process1():
            if not card.owner: return
            player = card.owner

            # Reveal 3
            revealed = []
            for _ in range(3):
                if player.library_cards:
                    revealed.append(player.library_cards.pop(0)) # Top is 0

            if not revealed:
                return

            # Helper to check conditions
            def is_huckmon_sistermon(c):
                for name in c.card_names:
                    if "Huckmon" in name or "Sistermon" in name:
                        return True
                return False

            def is_royal_knight(c):
                return "Royal Knight" in c.card_traits

            # Find distinct pair
            target1_obj = None
            target2_obj = None

            # Iterate all pairs to find optimal (both satisfied)
            found_pair = False
            for i in range(len(revealed)):
                for j in range(len(revealed)):
                    if i == j: continue

                    if is_huckmon_sistermon(revealed[i]) and is_royal_knight(revealed[j]):
                        target1_obj = revealed[i]
                        target2_obj = revealed[j]
                        found_pair = True
                        break
                if found_pair: break

            # If no pair found, try to satisfy at least one
            if not found_pair:
                # Prioritize Huckmon/Sistermon
                for c in revealed:
                    if is_huckmon_sistermon(c):
                        target1_obj = c
                        break
                # If still nothing, try Royal Knight
                if not target1_obj:
                    for c in revealed:
                        if is_royal_knight(c):
                            target2_obj = c
                            break

            # Add to hand
            to_hand = []
            if target1_obj:
                to_hand.append(target1_obj)
            if target2_obj:
                to_hand.append(target2_obj)

            # Uniqueify if same object picked? (Logic prevents it for pair, but fallback picks only one)
            # Just iterating list is fine.

            for c in to_hand:
                if c in revealed:
                    revealed.remove(c)
                    player.add_to_hand(c)

            # Return rest to bottom
            player.library_cards.extend(revealed)
            print(f"BT23-006: Returned {len(revealed)} cards to bottom of deck.")

        effect1.set_on_process_callback(on_process1)
        effects.append(effect1)

        # Inherited Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-006 Inherited Effect")
        effect2.set_effect_description("[Your Turn] [Once Per Turn] When any of your white Digimon are played, gain 1 memory.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)

        def condition2(context: Dict[str, Any]) -> bool:
            if not card.owner or not card.owner.is_my_turn:
                return False

            target_card = context.get("card_source")
            if target_card and target_card.card_colors:
                 if CardColor.White in target_card.card_colors:
                     return target_card.owner == card.owner
            return False

        def on_process2():
            if card.owner:
                card.owner.memory += 1
                print("BT23-006 Inherited: Gained 1 memory.")

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(on_process2)
        effects.append(effect2)

        return effects
