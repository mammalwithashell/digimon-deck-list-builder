from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from .....core.card_script import CardScript
from .....interfaces.card_effect import ICardEffect
from .....data.enums import EffectTiming

if TYPE_CHECKING:
    from .....core.card_source import CardSource


class ST18_09(CardScript):
    """ST18-09 Deramon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Blocker>
        effect0 = ICardEffect()
        effect0.set_effect_name("ST18-09 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [On Deletion] You may play 1 Digimon card with [Avian], [Bird],
        # [Vegetation] or [Plant] in any of its traits and 3000 DP or less
        # from your hand without paying the cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("ST18-09 On Deletion: Play 1 Avian/Bird/Vegetation/Plant 3000 DP or less")
        effect1.set_effect_description(
            "[On Deletion] You may play 1 Digimon card with [Avian], [Bird], "
            "[Vegetation] or [Plant] in any of its traits and 3000 DP or less "
            "from your hand without paying the cost."
        )
        effect1.is_on_deletion = True
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            target_traits = {'Avian', 'Bird', 'Vegetation', 'Plant'}

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                dp = getattr(c, 'dp', 0) or 0
                if dp > 3000:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any(any(tt in t for tt in target_traits) for t in traits)

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
