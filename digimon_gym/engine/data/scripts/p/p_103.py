from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_103(CardScript):
    """P-103 Offense Training"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Reveal the top 2 cards of your deck. Add 1 red card among them to your hand. Place the rest at the bottom of your deck in any order. Then, place this card into your battle area.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("P-103 Add To Hand, Reveal And Select")
        effect0.set_effect_description("[Main] Reveal the top 2 cards of your deck. Add 1 red card among them to your hand. Place the rest at the bottom of your deck in any order. Then, place this card into your battle area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Reveal top 2 cards, add 1 red card to hand, rest to bottom of deck"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def reveal_filter(c):
                colors = getattr(c, 'card_colors', [])
                return any(col.name == 'Red' for col in colors)
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 2, reveal_filter, on_revealed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        # Delay
        effect1 = ICardEffect()
        effect1.set_effect_name("P-103 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] <Delay> (Trash this card in your battle area to activate the effect below. You can't activate this effect the turn this card enters play.) - Your Digimon may digivolve into a red Digimon card in your hand for its digivolution cost. When it would digivolve by this effect, reduce the cost by 2.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2.set_effect_name("P-103 Your 1 Digimon digivolves")
        effect2.set_effect_description("[Main] <Delay> (Trash this card in your battle area to activate the effect below. You can't activate this effect the turn this card enters play.) - Your Digimon may digivolve into a red Digimon card in your hand for its digivolution cost. When it would digivolve by this effect, reduce the cost by 2.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Digivolve into a red Digimon card with cost reduction 2"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = getattr(c, 'card_colors', [])
                return any(col.name == 'Red' for col in colors)
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, cost_reduction=2, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # SecuritySkill effect: play self from security
        effect_sec = ICardEffect()
        effect_sec.set_effect_name("P-103 Security: Play this card")
        effect_sec.set_effect_description("[Security] Play this card without paying the cost.")
        effect_sec.is_security_effect = True

        def condition_sec(context: Dict[str, Any]) -> bool:
            return True
        effect_sec.set_can_use_condition(condition_sec)
        effects.append(effect_sec)

        return effects
