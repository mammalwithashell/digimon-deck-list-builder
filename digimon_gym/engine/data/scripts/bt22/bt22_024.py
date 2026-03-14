from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_024(CardScript):
    """BT22-024 MarineBullmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: decode
        # Decode
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-024 Decode")
        effect0.set_effect_description("Decode")
        effect0._is_decode = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDeclaration
        # [Hand] [Main] If you have [Yao Qinglan], by placing 1 [Shellmon] from your trash as any of your [Sangomon]'s bottom digivolution card, it digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDeclaration)
        effect1._is_field_main = True
        effect1.set_effect_name("BT22-024 Place 1 [Shellmon] from trash under 1 [Sangomon], to digivolve for 3")
        effect1.set_effect_description("[Hand] [Main] If you have [Yao Qinglan], by placing 1 [Shellmon] from your trash as any of your [Sangomon]'s bottom digivolution card, it digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Yao Qinglan') or permanent.contains_card_name('Sangomon'))):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                if not (any('Sea Animal' in _t or 'Aqua' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEndAttack
        # [End of Attack] [Once Per Turn] You may play 1 level 4 or lower Digimon card with [Aqua] or [Sea Animal] in any of its traits from this Digimon's digivolution cards without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndAttack)
        effect2.set_effect_name("BT22-024 Play 1 level 4 or lower [Aqua]/[Sea Animal] digimon from sources")
        effect2.set_effect_description("[End of Attack] [Once Per Turn] You may play 1 level 4 or lower Digimon card with [Aqua] or [Sea Animal] in any of its traits from this Digimon's digivolution cards without paying the cost.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT22_024_OnEndAttack")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                if not (any('Sea Animal' in _t or 'Aqua' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
