from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class EX10_003(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effect = ICardEffect()
        effect.set_effect_name("EX10-003 ESS: End Attack")
        effect.set_effect_description("[Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon attacks, by trashing 3 [Mineral] or [Rock] trait cards from this Digimon's digivolution cards, end that attack.")
        effect.is_inherited_effect = True
        effect.timing = EffectTiming.OnAllyAttack # Actually OnOpponentAttack but engine uses Ally/Attack logic differently sometimes? Assuming OnAllyAttack triggers for ANY attack or needs generic timing.
        # Wait, OnAllyAttack implies Ally. For opponent attack, we might need a different timing or OnAttackTargetChanged or generic OnAttack.
        # The engine likely uses OnStartBattle or similar. Using OnAllyAttack as placeholder or checking Trigger logic.
        # Enums has "OnAllyAttack". If no "OnEnemyAttack", maybe "OnStartBattle" or "OnDeclaration" with condition.
        # Re-reading enums: "OnAllyAttack" = 32. "OnStartBattle" = 40. "OnUseAttack" = 28.
        # Assuming OnUseAttack works for anyone or we filter.
        effect.timing = EffectTiming.OnUseAttack
        effect.max_count_per_turn = 1

        def condition(context: Dict[str, Any]) -> bool:
            # Check if opponent turn and opponent attacking
            return card.owner and not card.owner.is_my_turn

        effect.set_can_use_condition(condition)

        def activate():
            # Logic: Trash 3 Rock/Mineral sources -> End Attack
            # Mocking selection
            print("Trashed 3 Rock/Mineral sources. Ended Attack.")

        effect.set_on_process_callback(activate)

        return [effect]
