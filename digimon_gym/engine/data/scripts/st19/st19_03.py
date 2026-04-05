from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST19_03(CardScript):
    """ST19-03 Shoemon | Lv.3 (Yellow, Puppet/LIBERATOR)

    [On Play] Reveal the top 3 cards of your deck. Add 1 card with the
    [Puppet] trait and 1 card with the [LIBERATOR] trait among them to the
    hand. Return the rest to the bottom of the deck.

    Inherited Effect:
    [Your Turn] All of your opponent's security Digimon get -3000 DP.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] Reveal top 3, add 1 Puppet + 1 LIBERATOR to hand ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("ST19-03 Reveal top 3, add Puppet + LIBERATOR")
        effect0.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 card with "
            "the [Puppet] trait and 1 card with the [LIBERATOR] trait among "
            "them to the hand. Return the rest to the bottom of the deck."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            return len(owner.library_cards) >= 1

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Reveal top 3 and select 1 Puppet + 1 LIBERATOR to add to hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def puppet_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return any('Puppet' in t for t in traits)

            def liberator_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return any('LIBERATOR' in t or 'Liberator' in t for t in traits)

            game.effect_reveal_and_select_multi(
                player, 3,
                [(puppet_filter, 'hand'), (liberator_filter, 'hand')],
                remaining_placement='deck_bottom',
                is_optional=True,
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1 (Inherited): [Your Turn] Opponent's security Digimon get -3000 DP ---
        # Continuous/static inherited effect (EffectTiming.NoTiming per C#).
        # Engine applies dp_modifier from active inherited effects during
        # security battle DP calculation.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.NoTiming)
        effect1.set_effect_name("ST19-03 Opponent's Security Digimon -3000 DP")
        effect1.set_effect_description(
            "[Your Turn] All of your opponent's security Digimon get -3000 DP."
        )
        effect1.is_inherited_effect = True
        effect1.dp_modifier = -3000
        effect1._applies_to_opponent_security_digimon = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            return owner.is_my_turn

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            pass  # Declarative DP modifier for security Digimon

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
