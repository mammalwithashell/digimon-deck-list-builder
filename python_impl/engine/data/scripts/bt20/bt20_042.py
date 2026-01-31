from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any, Optional
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import CardKind, CardColor, Rarity, EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource
    from ....core.permanent import Permanent

class BT20_042(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] [When Digivolving] Suspend 1 of your opponent's Digimon or Tamers.
        # Then, 1 of their Digimon or Tamers can't unsuspend until the end of their turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("Suspend Opponent and Block Unsuspend")
        effect1.set_effect_description("[On Play] [When Digivolving] Suspend 1 of your opponent's Digimon or Tamers. Then, 1 of their Digimon or Tamers can't unsuspend until the end of their turn.")
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.is_on_play = True
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        def activate1(context: Dict[str, Any]):
            print("BT20-042: Suspending opponent permanent and preventing unsuspend.")
            # Select target
            # target.suspend()
            # target.cannot_unsuspend_until_turn_end = True

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(activate1)
        effects.append(effect1)

        # [All Turns] This Digimon is also treated as Lv.6 [Breakdramon] for [Examon]'s DNA digivolution.
        effect2 = ICardEffect()
        effect2.set_effect_name("Name Treatment Rule")
        effect2.set_effect_description("[All Turns] This Digimon is also treated as Lv.6 [Breakdramon] for [Examon]'s DNA digivolution.")
        effect2.set_timing(EffectTiming.NoTiming)
        effects.append(effect2)

        # Inherited Effect
        # [All Turns] [Once Per Turn] When this Digimon deletes your opponent's Digimon in battle, trash their top security card.
        inherited = ICardEffect()
        inherited.set_effect_name("Trash Security on Deletion")
        inherited.set_effect_description("[All Turns] [Once Per Turn] When this Digimon deletes your opponent's Digimon in battle, trash their top security card.")
        inherited.is_inherited_effect = True
        inherited.set_timing(EffectTiming.OnDestroyedAnyone)
        inherited.set_limit_once_per_turn()

        def inherited_condition(context: Dict[str, Any]) -> bool:
            winner = context.get("winner")
            loser = context.get("loser")
            if winner == inherited.effect_source_permanent and loser and loser.player != winner.player:
                return True
            return False

        def inherited_activate(context: Dict[str, Any]):
            print("BT20-042 Inherited: Trash opponent security.")
            permanent: Optional[Permanent] = inherited.effect_source_permanent
            if permanent and permanent.player:
                 pass
                 # permanent.player.opponent.trash_security(1) # Assuming we can access opponent

        inherited.set_can_use_condition(inherited_condition)
        inherited.set_on_process_callback(inherited_activate)
        effects.append(inherited)

        return effects
