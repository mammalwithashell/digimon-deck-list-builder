from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_019(CardScript):
    """BT17-019 Gabumon | Lv.3 Blue Rookie

    Alt digivolve: from [Tsunomon] for 0.
    [Start of Your Main Phase] If you have a Tamer with [Matt Ishida] in its
        name, [Draw 1].
    Inherited: [End of Your Turn] This Digimon and another of your Digimon
        may DNA digivolve into a Digimon card in the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from [Tsunomon] for 0 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT17-019 Alt digivolve from Tsunomon")
        effect0.set_effect_description("Alt digivolve: from [Tsunomon] for 0")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Tsunomon"
        effect0._alt_digi_exact = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [Start of Your Main Phase] Draw 1 if [Matt Ishida] tamer ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("BT17-019 Draw 1 with Matt Ishida")
        effect1.set_effect_description(
            "[Start of Your Main Phase] If you have a Tamer with [Matt Ishida] "
            "in its name, [Draw 1]."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must have a Tamer with [Matt Ishida]
            has_matt = any(
                p.is_tamer and p.top_card and
                (p.top_card.contains_card_name('Matt Ishida') or
                 p.top_card.contains_card_name('MattIshida'))
                for p in card.owner.battle_area
            )
            if not has_matt:
                return False
            return len(card.owner.library_cards) >= 1

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.draw()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Inherited [End of Your Turn] DNA digivolve from hand ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndTurn)
        effect2.set_effect_name("BT17-019 Inherited: End of Turn DNA digivolve")
        effect2.set_effect_description(
            "[End of Your Turn] This Digimon and another of your Digimon may "
            "DNA digivolve into a Digimon card in the hand."
        )
        effect2.is_inherited_effect = True
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if len(card.owner.hand_cards) < 1:
                return False
            other_digimon = [
                p for p in card.owner.battle_area
                if p.is_digimon and p is not perm
            ]
            if not other_digimon:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            def dna_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (c.c_entity_base and c.c_entity_base.dna_costs):
                    return False
                return True

            game.effect_dna_digivolve_from_hand(
                player, dna_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
