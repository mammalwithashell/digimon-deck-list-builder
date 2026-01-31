from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any, Optional
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import CardKind, CardColor, Rarity, EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource
    from ....core.permanent import Permanent

class BT20_040(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Raid>
        raid = ICardEffect()
        raid.set_effect_name("Raid")
        raid.set_effect_description("<Raid> (When this Digimon attacks, you may switch the target of attack to 1 of your opponent's unsuspended Digimon with the highest DP.)")
        raid.is_keyword_effect = True
        raid.keyword = "Raid"
        effects.append(raid)

        # [Your Turn] When any of your blue Digimon with [Dracomon] or [Examon] in their texts are played,
        # this Digimon may digivolve into [Groundramon] in the hand with the digivolution cost reduced by 2.
        effect1 = ICardEffect()
        effect1.set_effect_name("Digivolve into Groundramon")
        effect1.set_effect_description("[Your Turn] When any of your blue Digimon with [Dracomon] or [Examon] in their texts are played, this Digimon may digivolve into [Groundramon] in the hand with the digivolution cost reduced by 2.")
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)

        def condition1(context: Dict[str, Any]) -> bool:
            permanent: Optional[Permanent] = effect1.effect_source_permanent
            played_perm: Optional[Permanent] = context.get("permanent")
            if not permanent or not played_perm: return False
            if played_perm.player != permanent.player: return False

            # Simplified check (assuming traits/text populated)
            # return True if criteria met
            return True

        def activate1(context: Dict[str, Any]):
            print("BT20-040: Triggering digivolution to Groundramon.")
            # permanent.digivolve(card_from_hand, cost_reduction=2)

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(activate1)
        effects.append(effect1)

        # Inherited Effect
        # [Your Turn] This Digimon gets +2000 DP.
        inherited = ICardEffect()
        inherited.set_effect_name("DP Boost")
        inherited.set_effect_description("[Your Turn] This Digimon gets +2000 DP.")
        inherited.is_inherited_effect = True
        inherited.set_timing(EffectTiming.NoTiming)

        def inherited_get_dp(permanent: Permanent) -> int:
            # Need to access player turn state
            # Assuming permanent.player.is_my_turn is available
            # We need to ensure we don't crash if player is None
            if permanent.player and permanent.player.is_my_turn:
                return 2000
            return 0

        # Attach the DP modifier function
        # The engine (Permanent.dp) needs to know to call this.
        # Ideally ICardEffect has a standard field for this or `get_change_dp_value` method
        # I will implement `get_change_dp_value` in the script logic here and ensure interface supports it.
        inherited.get_change_dp_value = inherited_get_dp

        effects.append(inherited)

        return effects
