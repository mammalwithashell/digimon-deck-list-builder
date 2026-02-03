from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT13_011(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Effect 1: [On Play] [When Digivolving] Delete 1 of your opponent's Digimon with 3000 DP or less.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-011 Main Effect")
        effect1.set_effect_description("[On Play] [When Digivolving] Delete 1 of your opponent's Digimon with 3000 DP or less.")
        effect1.is_on_play = True
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        def on_process1():
            # Delete 1 of opponent's Digimon with 3000 DP or less.
            permanent = effect1.effect_source_permanent
            if not permanent: return
            owner = card.owner
            if not owner or not owner.game: return

            opponent = owner.game.opponent_player
            # Find targets
            targets = [perm for perm in opponent.battle_area if perm.dp <= 3000 and perm.is_digimon]

            if not targets:
                print("No targets to delete (DP <= 3000).")
                return

            # Select target (First one for now)
            target = targets[0]
            print(f"Deleting opponent's {target.top_card.card_names[0]}")
            opponent.delete_permanent(target)

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(on_process1)
        effects.append(effect1)

        # Effect 2: [On Deletion] <Draw 1>
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-011 Inherited Effect")
        effect2.set_effect_description("[On Deletion] <Draw 1>")
        effect2.is_inherited_effect = True
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            if context.get("timing") != EffectTiming.OnDestroyedAnyone: return False
            victim = context.get("caused_by_permanent")
            # Only trigger if the deleted card is THIS card
            if victim and victim == effect2.effect_source_permanent:
                return True
            return False

        def on_process2():
            owner = card.owner
            if owner:
                owner.draw()

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(on_process2)
        effects.append(effect2)

        return effects
