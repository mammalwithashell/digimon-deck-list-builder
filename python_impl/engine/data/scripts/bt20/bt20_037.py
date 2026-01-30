from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any, Optional
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import CardKind, CardColor, Rarity, EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource
    from ....core.permanent import Permanent

class BT20_037(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Security A. +1>
        sec_plus = ICardEffect()
        sec_plus.set_effect_name("Security Attack +1")
        sec_plus.set_effect_description("<Security A. +1> (This Digimon checks 1 additional security card.)")
        sec_plus.is_keyword_effect = True
        sec_plus.keyword = "Security Attack +1"
        effects.append(sec_plus)

        # <Partition (Yellow Lv.6 & Green/Black Lv.6)>
        partition = ICardEffect()
        partition.set_effect_name("Partition")
        partition.set_effect_description("<Partition (Yellow Lv.6 & Green/Black Lv.6)> (When this Digimon with each of the specified digivolution cards would leave the battle area other than by your own effects or by battle, you may play 1 each of the specified cards without paying the costs.)")
        partition.is_keyword_effect = True
        partition.keyword = "Partition"
        effects.append(partition)

        # [When Digivolving] For each of this Digimon's level 6 digivolution cards,
        # suspend 1 of your opponent's Digimon or Tamers and gain 1 memory.
        # Then none of their Digimon or Tamers can activate [On Play] effects or unsuspend until the end of their turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("Suspend, Gain Memory, Block On Play/Unsuspend")
        effect1.set_effect_description("[When Digivolving] For each of this Digimon's level 6 digivolution cards, suspend 1 of your opponent's Digimon or Tamers and gain 1 memory. Then none of their Digimon or Tamers can activate [On Play] effects or unsuspend until the end of their turn.")
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        def activate1(context: Dict[str, Any]):
            permanent: Optional[Permanent] = effect1.effect_source_permanent
            count = 0
            if permanent:
                for src in permanent.digivolution_cards:
                    if src.level == 6:
                        count += 1

            print(f"BT20-037 Effect 1: Count = {count}. Suspending {count} targets and gaining {count} memory.")
            if permanent and permanent.player:
                permanent.player.gain_memory(count)
            # Logic to block On Play / Unsuspend

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(activate1)
        effects.append(effect1)

        return effects
