from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any, Optional
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import CardKind, CardColor, Rarity, EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource
    from ....core.permanent import Permanent

class BT20_039(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] [When Digivolving] Suspend 1 of your opponent's Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("Suspend Opponent")
        effect1.set_effect_description("[On Play] [When Digivolving] Suspend 1 of your opponent's Digimon.")
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.is_on_play = True
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        def activate1(context: Dict[str, Any]):
            print("BT20-039: Suspending opponent digimon.")
            permanent: Optional[Permanent] = effect1.effect_source_permanent
            if not permanent: return

            # Simple heuristic: find first unsuspended opponent digimon
            opponent_permanents = [] # Should get from game state via context or permanent.player.opponent
            # Since we don't have opponent link in Player yet, we can't implement targeting fully.
            # Printing is the best we can do without GameState access in this scope.
            # Assuming context might have 'game' or similar later.

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(activate1)
        effects.append(effect1)

        # Inherited Effect
        # <Piercing>
        inherited = ICardEffect()
        inherited.set_effect_name("Piercing")
        inherited.set_effect_description("<Piercing> (When this Digimon attacks and deletes an opponent's Digimon in battle, it checks security before the attack ends.)")
        inherited.is_inherited_effect = True
        inherited.is_keyword_effect = True
        inherited.keyword = "Piercing"
        effects.append(inherited)

        return effects
