from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_017(CardScript):
    """BT22-017 Gabumon | Lv.3

    Digivolve: from [Tsunomon] or Lv.2 w/ CS trait for 0.
    [On Play] Reveal the top 3 cards of your deck. Add 1 card with [Omnimon]
        in its text and 1 card with the [CS] trait among them to the hand.
        Return the rest to the bottom of the deck.
    Inherited: [End of Your Turn] This Digimon and any of your other Digimon
        may DNA digivolve into a Digimon card in the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from [Tsunomon] for cost 0 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-017 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 2
        effect0._alt_digi_name = "Tsunomon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Tsunomon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [On Play] Reveal 3, add 1 w/ Omnimon in text + 1 w/ CS trait ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT22-017 Reveal top 3, add Omnimon-text + CS-trait")
        effect1.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 card with "
            "[Omnimon] in its text and 1 card with the [CS] trait among them "
            "to the hand. Return the rest to the bottom of the deck."
        )
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Reveal 3, pick 1 w/ Omnimon text, 1 w/ CS trait, rest to bottom."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def omnimon_text_filter(c):
                text = getattr(c, 'card_text', '') or ''
                return 'Omnimon' in text

            def cs_trait_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return any('CS' in t for t in traits)

            game.effect_reveal_and_select_multi(
                player, 3,
                passes=[
                    (omnimon_text_filter, 'hand'),
                    (cs_trait_filter, 'hand'),
                ],
                remaining_placement='deck_bottom',
                is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Inherited [End of Your Turn] DNA digivolve ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndTurn)
        effect2.set_effect_name("BT22-017 DNA digivolve this Digimon")
        effect2.set_effect_description(
            "[End of Your Turn] This Digimon and any of your other Digimon "
            "may DNA digivolve into a Digimon card in the hand."
        )
        effect2.is_inherited_effect = True
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """DNA digivolve from hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            game.effect_dna_digivolve_from_hand(
                player,
                filter_fn=lambda c: getattr(c, 'is_digimon', False),
                is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
