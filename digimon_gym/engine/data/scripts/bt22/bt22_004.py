from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_004(CardScript):
    """BT22-004 Wanyamon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAddDigivolutionCards
        # [Your Turn] [Once Per Turn] When effects place [CS] trait Digimon cards in this Digimon's digivolution cards, it may digivolve into a [CS] trait Digimon card in the hand with the digivolution cost reduced by 1.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnAddDigivolutionCards)
        effect0.set_effect_name("BT22-004 Digivolve into a [CS] digimon")
        effect0.set_effect_description("[Your Turn] [Once Per Turn] When effects place [CS] trait Digimon cards in this Digimon's digivolution cards, it may digivolve into a [CS] trait Digimon card in the hand with the digivolution cost reduced by 1.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("BT22_004_Digivolve")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Wanyamon must be in a stack on a permanent (field or breeding)
            this_perm = card.permanent_of_this_card() if card else None
            if this_perm is None:
                return False
            # [Your Turn] gate
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Event identity: the permanent whose stack received the card
            # must be this Wanyamon's permanent. DCGO:
            #   permanentCondition: permanent => permanent == card.PermanentOfThisCard()
            event_permanent = context.get('event_permanent')
            if event_permanent is not this_perm:
                return False
            # The added card must be a [CS] trait Digimon. DCGO:
            #   cardCondition: cardSource => cardSource.IsDigimon && cardSource.HasCSTraits
            added_card = context.get('added_card')
            if added_card is None:
                return False
            if not getattr(added_card, 'is_digimon', False):
                return False
            traits = getattr(added_card, 'card_traits', []) or []
            if not any(t == 'CS' for t in traits):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Digivolve into a [CS] trait Digimon from hand with
            the digivolution cost reduced by 1."""
            player = ctx.get('player')
            game = ctx.get('game')
            # Use Wanyamon's own permanent (not ctx['permanent'], which could
            # be overwritten by future context changes). Matches DCGO:
            #   targetPermanent: card.PermanentOfThisCard().TopCard.PermanentOfThisCard()
            perm = card.permanent_of_this_card() if card else None
            if not (player and perm and game):
                return
            def cs_digimon_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any(t == 'CS' for t in traits)
            game.effect_digivolve_from_hand(
                player, perm, cs_digimon_filter,
                cost_reduction=1, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
