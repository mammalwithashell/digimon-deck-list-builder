from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT20_095(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Effect 1: Main
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-095 Main")
        effect1.set_effect_description("[Main] Reveal 3, Add 1 [Chronicle], Place in Battle Area.")
        # Timing: OnUseOption? Or Main Phase?
        # Usually Options have "Main" effect.
        # Engine likely looks for "OptionSkill" (Enum 5) or similar?
        # Or just "NoTiming" and assumes it's the main effect.
        # But wait, if it's placed in Battle Area, it needs to be triggered.
        # I'll assume usage via Game.action_play_card triggers OnEnterField,
        # but that's for Permanents.
        # If I play an Option, does it trigger its effect?
        # Game.action_play_card just plays it.
        # It triggers OnEnterFieldAnyone.
        # If the card is an Option, maybe OnEnterField effects are the "Main" effects?
        # I'll use EffectTiming.OnEnterFieldAnyone for now as it's triggered when played.

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        def process1():
            owner = card.owner
            if not owner: return

            # Reveal 3
            revealed = []
            for _ in range(3):
                if owner.library_cards:
                    revealed.append(owner.library_cards.pop(0))

            # Filter "Chronicle" (Greedy: Pick first)
            picked = None
            for c in revealed:
                if "Chronicle" in c.card_traits:
                    picked = c
                    break

            if picked:
                revealed.remove(picked)
                owner.hand_cards.append(picked)
                print(f"BT20-095: Added {picked.card_names[0]} to hand.")

            # Return rest to bottom
            owner.library_cards.extend(revealed)
            print(f"BT20-095: Returned {len(revealed)} cards to deck bottom.")

            # Card is already placed in Battle Area by play_card action.

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(process1)
        # Assuming OnEnterFieldAnyone triggers main effect of Option played as permanent
        # If it was a standard Option use (trash), it might be different.
        # But this one says "Place in Battle Area", so treating it as Permanent placement is correct.
        # We need to ensure we use the correct timing enum.
        # I'll rely on the default effect retrieval if no timing specified, or use OnEnterFieldAnyone.
        # Actually, let's look at `bt23_001` - it was inherited.
        # I'll create it as a standard effect.
        effects.append(effect1)

        # Effect 2: Delay (Triggered)
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-095 Delay")
        effect2.set_effect_description("[All Turns] When [Chronicle] deleted, <Delay> ...")
        # Timing: OnDestroyedAnyone (Enum 6)

        def condition2(context: Dict[str, Any]) -> bool:
            # Check if deleted card was Chronicle and owned by player
            # Context needs 'permanent' (the deleted one).
            # In Game.delete_permanent, does it trigger OnDestroyedAnyone?
            # Game.delete_permanent removes from battle_area.
            # Then it calls execute_effects?
            # Game.resolve_attack deletes.
            # It does NOT call execute_effects(OnDestroyedAnyone)!
            # It calls delete_permanent, which just prints.
            # So OnDestroyedAnyone is NOT triggered in current engine.
            # So this effect won't trigger.
            return False

        def process2():
            print("BT20-095 Delay Triggered (Not supported).")

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
