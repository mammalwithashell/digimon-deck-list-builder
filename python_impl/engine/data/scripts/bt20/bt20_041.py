from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any, Optional
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import CardKind, CardColor, Rarity, EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource
    from ....core.permanent import Permanent

class BT20_041(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] [When Digivolving] Suspend 1 of your opponent's Digimon and 1 of your Digimon gets +3000 DP for the turn.
        # Then, 1 of your Digimon may attack.
        effect1 = ICardEffect()
        effect1.set_effect_name("Suspend Opponent, Buff Self, Attack")
        effect1.set_effect_description("[On Play] [When Digivolving] Suspend 1 of your opponent's Digimon and 1 of your Digimon gets +3000 DP for the turn. Then, 1 of your Digimon may attack.")
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.is_on_play = True
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        def activate1(context: Dict[str, Any]):
            print("BT20-041: Activated On Play/When Digivolving.")
            permanent: Optional[Permanent] = effect1.effect_source_permanent
            if not permanent: return

            # 1. Suspend Opponent (mock)
            # 2. Buff Self (or ally) +3000 DP (mock using modifier)
            permanent.dp_modifier += 3000
            # 3. Attack (mock)
            # permanent.attack(None)

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(activate1)
        effects.append(effect1)

        # Inherited Effect
        # [When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -4000 DP for the turn.
        inherited = ICardEffect()
        inherited.set_effect_name("DP Minus on Attack")
        inherited.set_effect_description("[When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -4000 DP for the turn.")
        inherited.is_inherited_effect = True
        inherited.set_timing(EffectTiming.OnUseAttack)
        inherited.set_limit_once_per_turn()

        def inherited_condition(context: Dict[str, Any]) -> bool:
            attacker = context.get("attacker")
            if attacker == inherited.effect_source_permanent:
                return True
            return False

        def inherited_activate(context: Dict[str, Any]):
            print("BT20-041 Inherited: Opponent Digimon -4000 DP.")
            # Select target and apply -4000 DP

        inherited.set_can_use_condition(inherited_condition)
        inherited.set_on_process_callback(inherited_activate)
        effects.append(inherited)

        return effects
