from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_069(CardScript):
    """EX10-069 Unique Emblem: Gravel Hearts"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] You may play 1 [Sunarizamon] or [Close] from your hand or trash without paying the cost. Then, place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("EX10-069 Play 1 [Sunarizamon]/[Close] from hand or trash, then place in battle area")
        effect0.set_effect_description("[Main] You may play 1 [Sunarizamon] or [Close] from your hand or trash without paying the cost. Then, place this card in the battle area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not (any('Sunarizamon' in _n or 'Close' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        # Delay
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-069 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            return bool(card and card.permanent_of_this_card() is not None)
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnTappedAnyone
        # [Your Turn] When any of your [Close] suspend, <Delay> (By trashing this card after the placing turn, activate the effect below.)\r\n・1 of your [Rock] or [Mineral] trait Digimon may digivolve into a [Rock] or [Mineral] and [LIBERATOR] trait Digimon card in the hand with the digivolution cost reduced by 3.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnTappedAnyone)
        effect2.set_effect_name("EX10-069 1 of your [Rock] or [Mineral] trait Digimon may digivolve")
        effect2.set_effect_description("[Your Turn] When any of your [Close] suspend, <Delay> (By trashing this card after the placing turn, activate the effect below.)\\r\\n・1 of your [Rock] or [Mineral] trait Digimon may digivolve into a [Rock] or [Mineral] and [LIBERATOR] trait Digimon card in the hand with the digivolution cost reduced by 3.")
        effect2.is_optional = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            event_perm = context.get('event_permanent')
            if not (event_perm and event_perm in card.owner.battle_area):
                return False
            if not event_perm.contains_card_name('Close'):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def base_filter(target_perm):
                return target_perm.has_trait('Rock') or target_perm.has_trait('Mineral')

            def on_target(target_perm):
                def digi_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    traits = set(getattr(c, 'card_traits', []))
                    has_rock_or_mineral = 'Rock' in traits or 'Mineral' in traits
                    return has_rock_or_mineral and 'LIBERATOR' in traits

                game.effect_digivolve_from_hand(
                    player,
                    target_perm,
                    digi_filter,
                    cost_reduction=3,
                    is_optional=True,
                )

            game.effect_select_own_permanent(
                player,
                on_target,
                base_filter,
                is_optional=True,
                prompt="Select 1 of your [Rock] or [Mineral] Digimon to digivolve.",
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: security_play
        # Security: Play this card
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-069 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
