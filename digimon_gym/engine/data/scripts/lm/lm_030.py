from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class LM_030(CardScript):
    """LM-030 Green Scramble (Option)"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Main] 1 of your green Digimon may digivolve into a green Digimon card
        # in the hand with the digivolution cost reduced by 3. Then, place this
        # card in that Digimon's digivolution cards.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("LM-030 Green Digimon digivolve from hand with cost -3")
        effect0.set_effect_description(
            "[Main] 1 of your green Digimon may digivolve into a green Digimon "
            "card in the hand with the digivolution cost reduced by 3. Then, "
            "place this card in that Digimon's digivolution cards."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def digi_filter(c):
                colors = [col.name for col in getattr(c, 'card_colors', [])]
                return 'Green' in colors and getattr(c, 'is_digimon', False)

            # Select own green Digimon to digivolve
            def perm_filter(p):
                return p.is_digimon and any(
                    'Green' in col.name
                    for col in getattr(p.top_card, 'card_colors', [])
                ) if p.top_card else False

            def on_select_perm(target_perm):
                game.effect_digivolve_from_hand(
                    player, target_perm, digi_filter,
                    cost_reduction=3, is_optional=True)

            game.effect_select_own_permanent(
                player, on_select_perm, filter_fn=perm_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Security Effect [Security] You may play 1 green Digimon card with
        # 2000 DP or less from your trash without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("LM-030 Security: Play green Digimon 2000 DP or less from trash")
        effect1.set_effect_description(
            "Security: You may play 1 green Digimon card with 2000 DP or less "
            "from your trash without paying the cost."
        )
        effect1.is_security_effect = True
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                dp = getattr(c, 'dp', 0) or 0
                if dp > 2000:
                    return False
                colors = [col.name for col in getattr(c, 'card_colors', [])]
                return 'Green' in colors

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
