from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any, Optional
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import CardKind, CardColor, Rarity, EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource
    from ....core.permanent import Permanent
    from ....core.player import Player

class BT20_035(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Fortitude>
        fortitude = ICardEffect()
        fortitude.set_effect_name("Fortitude")
        fortitude.set_effect_description("<Fortitude> (When this Digimon with digivolution cards is deleted, play this card without paying the cost.)")
        fortitude.is_keyword_effect = True
        fortitude.keyword = "Fortitude"
        effects.append(fortitude)

        # [When Digivolving] Suspend 1 of your opponent's Digimon or Tamers.
        # Then, 1 of their Digimon or Tamers can't unsuspend until the end of their turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("Suspend Opponent")
        effect1.set_effect_description("[When Digivolving] Suspend 1 of your opponent's Digimon or Tamers. Then, 1 of their Digimon or Tamers can't unsuspend until the end of their turn.")
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone) # Using OnEnterField as proxy for Digivolve in this simplified engine

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        def activate1(context: Dict[str, Any]):
            print("BT20-035 Effect 1: Suspending opponent permanent.")
            # In real engine: Select target, target.suspend(), apply "cannot unsuspend" status.

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(activate1)
        effects.append(effect1)

        # [All Turns] When Tamer cards are placed in this Digimon's digivolution cards,
        # activate 1 of this Digimon's [When Digivolving] effects.
        # Then, 1 of your Digimon may attack your opponent's Digimon.
        effect2 = ICardEffect()
        effect2.set_effect_name("Activate When Digivolving")
        effect2.set_effect_description("[All Turns] When Tamer cards are placed in this Digimon's digivolution cards, activate 1 of this Digimon's [When Digivolving] effects. Then, 1 of your Digimon may attack your opponent's Digimon.")
        effect2.set_timing(EffectTiming.OnAddDigivolutionCards)

        def condition2(context: Dict[str, Any]) -> bool:
            permanent: Optional[Permanent] = effect2.effect_source_permanent
            target_permanent: Optional[Permanent] = context.get("permanent")

            if permanent != target_permanent:
                return False

            added_cards: List[CardSource] = context.get("added_cards", [])
            for c in added_cards:
                if c.card_kind == CardKind.Tamer:
                    return True
            return False

        def activate2(context: Dict[str, Any]):
            print("BT20-035 Effect 2: Triggering When Digivolving effect and allowing attack.")
            # Logic to find and execute "When Digivolving" effect would go here.

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(activate2)
        effects.append(effect2)

        # Inherited Effect
        # [All Turns] [Once Per Turn] When your security stack is removed from,
        # if this Digimon has [Fenriloogamon] in its name, <Recovery +1 (Deck)>
        inherited = ICardEffect()
        inherited.set_effect_name("Recovery on Security Removal")
        inherited.set_effect_description("[All Turns] [Once Per Turn] When your security stack is removed from, if this Digimon has [Fenriloogamon] in its name, <Recovery +1 (Deck)>")
        inherited.is_inherited_effect = True
        inherited.set_timing(EffectTiming.OnLoseSecurity)
        inherited.set_limit_once_per_turn()

        def inherited_condition(context: Dict[str, Any]) -> bool:
            permanent: Optional[Permanent] = inherited.effect_source_permanent
            if not permanent: return False
            if "Fenriloogamon" not in permanent.card_source.card_name_eng:
                return False
            return True

        def inherited_activate(context: Dict[str, Any]):
            print("BT20-035 Inherited: Recovery +1 (Deck)")
            # permanent.player.recovery(1)

        inherited.set_can_use_condition(inherited_condition)
        inherited.set_on_process_callback(inherited_activate)
        effects.append(inherited)

        return effects
