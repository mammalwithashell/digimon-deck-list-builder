from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any, Optional
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import CardKind, CardColor, Rarity, EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource
    from ....core.permanent import Permanent

class BT20_017_token(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Token stats: 6000 DP, White, Reboot, Blocker, Decoy (Red/Black)
        # Note: Stats are usually in cards.json, but keywords are effects.

        # <Reboot>
        reboot = ICardEffect()
        reboot.set_effect_name("Reboot")
        reboot.set_effect_description("<Reboot> (Unsuspend this Digimon during your opponent's unsuspend phase.)")
        reboot.is_keyword_effect = True
        reboot.keyword = "Reboot"
        effects.append(reboot)

        # <Blocker>
        blocker = ICardEffect()
        blocker.set_effect_name("Blocker")
        blocker.set_effect_description("<Blocker> (When an opponent's Digimon attacks, you may suspend this Digimon to force the opponent to attack it instead.)")
        blocker.is_keyword_effect = True
        blocker.keyword = "Blocker"
        effects.append(blocker)

        # <Decoy (Red/Black)>
        decoy = ICardEffect()
        decoy.set_effect_name("Decoy (Red/Black)")
        decoy.set_effect_description("<Decoy (Red/Black)> (When one of your other Red or Black Digimon would be deleted by an opponent's effect, you may delete this Digimon to prevent that deletion.)")
        decoy.is_keyword_effect = True
        decoy.keyword = "Decoy"
        # Logic for Decoy usually involves an interruption timing, simplified here as keyword.
        effects.append(decoy)

        return effects
