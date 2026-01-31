from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class EX10_008(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # On Play / When Digivolving
        # Effect: Give 1 opponent Digimon <Collision> and "Start of Main Phase: Attack"
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-008: Give Collision/Attack")
        effect1.timing = EffectTiming.OnEnterFieldAnyone # Simplified timing mapping

        def activate1():
            # Stub: Select opponent digimon and apply effect
            print(f"{card.card_names[0]}: Gave opponent Digimon <Collision> and forced attack effect.")

        effect1.set_on_process_callback(activate1)
        effects.append(effect1)

        # ESS: Opponent's Turn On Attack Target Changed
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-008 ESS: Trash Security")
        effect2.is_inherited_effect = True
        effect2.timing = EffectTiming.OnAttackTargetChanged
        effect2.max_count_per_turn = 1

        def condition2(context: Dict[str, Any]) -> bool:
            permanent = context.get("permanent")
            if permanent and not permanent.top_card.owner.is_my_turn and "Greymon" in permanent.top_card.card_names[0]:
                return True
            return False

        effect2.set_can_use_condition(condition2)

        def activate2():
            opponent = card.owner.opponent if hasattr(card.owner, 'opponent') else None
            # In PoC player doesn't strictly link opponent yet, skip logic or assumes global state
            print("Trashed opponent's top security card.")

        effect2.set_on_process_callback(activate2)
        effects.append(effect2)

        return effects
