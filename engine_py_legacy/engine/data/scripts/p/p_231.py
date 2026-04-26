from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_231(CardScript):
    """P-231 Unique Emblem: Invincibly Invisible"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Reveal the top 3 cards of your deck. Add 1 [Cyborg] or [Machine] trait Digimon card and 1 [LIBERATOR] trait card among them to the hand. Return the rest to the bottom of the deck. Then, place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("P-231 Reveal 3, add 1 [Cyborg]/[Machine] trait and 1 [Liberator] trait, bot deck the rest, place in battle area.")
        effect0.set_effect_description("[Main] Reveal the top 3 cards of your deck. Add 1 [Cyborg] or [Machine] trait Digimon card and 1 [LIBERATOR] trait card among them to the hand. Return the rest to the bottom of the deck. Then, place this card in the battle area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Add To Hand, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if not (player and game):
                return
            def reveal_filter_0(c):
                if not (any('Cyborg' in _t or 'Machine' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def reveal_filter_1(c):
                if not (any('LIBERATOR' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_reveal_and_select_multi(
                player, 3, [(reveal_filter_0, 'hand'), (reveal_filter_1, 'hand')],
                remaining_placement='deck_bottom', is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        # Delay
        effect1 = ICardEffect()
        effect1.set_effect_name("P-231 Delay")
        effect1.set_effect_description("Delay")
        effect1.is_on_play = True
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Altea'))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] When any of your [Altea]s are played, <Delay>.\r\n 1 of your Digimon may digivolve into a level 6 or lower [LIBERATOR] trait card in the hand with the digivolution cost reduced by 3.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("P-231 Digivolve into a level 6 or lower [LIBERATOR] Digimon for 3 less.")
        effect2.set_effect_description("[Your Turn] When any of your [Altea]s are played, <Delay>.\\r\\n 1 of your Digimon may digivolve into a level 6 or lower [LIBERATOR] trait card in the hand with the digivolution cost reduced by 3.")
        effect2.is_optional = True
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Altea'))):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not (any('Cyborg' in _t or 'Machine' in _t or 'LIBERATOR' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: security_play
        # Security: Play this card
        effect3 = ICardEffect()
        effect3.set_effect_name("P-231 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
