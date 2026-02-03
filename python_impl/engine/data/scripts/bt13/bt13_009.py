from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT13_009(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Effect 1
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-009 Main Effect")
        effect1.set_effect_description("[Your Turn] When you play a Digimon with [Sistermon] in its name, you may digivolve this Digimon into a [BaoHuckmon] in your hand without paying its cost.")

        def condition1(context: Dict[str, Any]) -> bool:
            if context.get("timing") != EffectTiming.OnEnterFieldAnyone:
                return False

            game = context.get("game")
            player = context.get("player")
            if not game or not player: return False
            if game.turn_player != player: return False

            caused_by_card = context.get("caused_by_card")
            if not caused_by_card: return False
            if caused_by_card.owner != player: return False
            if not caused_by_card.is_digimon: return False
            if not any("Sistermon" in name for name in caused_by_card.card_names):
                return False

            return True

        def on_process1():
            permanent = effect1.effect_source_permanent
            if not permanent: return
            owner = card.owner # Captured from arg
            if not owner: return

            bao_huckmons = [c for c in owner.hand_cards if "BaoHuckmon" in c.card_names[0]]
            if not bao_huckmons:
                print("No BaoHuckmon in hand.")
                return

            target_card = bao_huckmons[0]
            print(f"Digivolving {permanent.top_card.card_names[0]} into {target_card.card_names[0]} without paying cost.")
            owner.digivolve(permanent, target_card)
            # owner.digivolve does not charge memory, so no refund needed.

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(on_process1)
        effects.append(effect1)

        # Effect 2 (Inherited)
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-009 Inherited Effect")
        effect2.set_effect_description("[Your Turn] [Once Per Turn] When you play a Digimon with [Sistermon] in its name, gain 1 memory.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)

        def condition2(context: Dict[str, Any]) -> bool:
            if context.get("timing") != EffectTiming.OnEnterFieldAnyone: return False
            game = context.get("game")
            player = context.get("player")
            if not game or not player: return False
            if game.turn_player != player: return False
            caused_by_card = context.get("caused_by_card")
            if not caused_by_card: return False
            if caused_by_card.owner != player: return False
            if not caused_by_card.is_digimon: return False
            if not any("Sistermon" in name for name in caused_by_card.card_names): return False
            return True

        def on_process2():
            owner = card.owner
            if owner:
                owner.gain_memory(1)

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(on_process2)
        effects.append(effect2)

        return effects
