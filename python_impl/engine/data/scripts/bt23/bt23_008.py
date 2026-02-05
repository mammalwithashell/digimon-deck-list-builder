from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT23_008(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Raid
        effect1 = ICardEffect()
        effect1.set_effect_name("Raid")
        effect1.set_effect_description("<Raid>")
        effect1.is_keyword_effect = True
        effect1.keyword = "Raid"
        effects.append(effect1)

        # Main Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-008 Main Effect")
        effect2.set_effect_description("[Main] [Once Per Turn] By placing this Digimon's top stacked card as its bottom digivolution card, you may play 1 [Gabumon] or [Nokia Shiramine] from your hand with the play cost reduced by 2.")
        effect2.set_max_count_per_turn(1)

        def condition2(context: Dict[str, Any]) -> bool:
            if not card.owner or not card.owner.is_my_turn:
                return False

            permanent = effect2.effect_source_permanent
            if not permanent:
                return False

            # Must have at least 2 cards (Top + Source) to move Top to Bottom
            if len(permanent.card_sources) < 2:
                return False

            return True

        def on_process2():
            if not card.owner: return
            permanent = effect2.effect_source_permanent
            if not permanent or len(permanent.card_sources) < 2:
                return

            # Move top to bottom
            # NOTE: Permanent.card_sources INCLUDES the top card at the last index.
            # We remove the top card (the Digimon itself) and insert it at index 0 (bottom).
            # The card previously at [-2] becomes the new top card (De-Digivolve effect logic).
            top_card = permanent.card_sources.pop()
            permanent.card_sources.insert(0, top_card)
            print(f"BT23-008: Moved {top_card.card_names[0]} to bottom sources.")

            # Play Gabumon or Nokia from hand
            player = card.owner
            candidates = []
            for c in player.hand_cards:
                is_valid = False
                for name in c.card_names:
                    if "Gabumon" in name or "Nokia Shiramine" in name:
                        is_valid = True
                        break
                if is_valid:
                    candidates.append(c)

            if not candidates:
                print("BT23-008: No targets to play.")
                return

            # Select one (Heuristic: First one)
            target = candidates[0]

            base_cost = 0
            if target.c_entity_base and target.c_entity_base.play_cost:
                base_cost = target.c_entity_base.play_cost

            # NOTE: Player.play_card() places the card but does NOT deduct memory.
            # We must deduct memory manually based on the effect's cost reduction.
            final_cost = max(0, base_cost - 2)
            player.memory -= final_cost
            print(f"BT23-008: Paying {final_cost} memory (reduced from {base_cost}).")

            player.play_card(target)

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(on_process2)
        effects.append(effect2)

        # Inherited
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-008 Inherited Effect")
        effect3.set_effect_description("[Your Turn] This Digimon gets +2000 DP.")
        effect3.is_inherited_effect = True
        effect3.dp_modifier = 2000

        def condition3(context: Dict[str, Any]) -> bool:
            permanent = context.get("permanent")
            if permanent and permanent.top_card and permanent.top_card.owner:
                return permanent.top_card.owner.is_my_turn
            return False

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
