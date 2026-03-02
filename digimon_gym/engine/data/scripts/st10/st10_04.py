from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST10_04(CardScript):
    """ST10-04 Gatomon | Lv.4 Yellow/Purple Digimon

    [On Play] Reveal the top 3 cards of your deck. Add 1 yellow Digimon card
        and 1 purple Digimon card among them to your hand. Place the remaining
        cards at the bottom of your deck in any order.
    [Your Turn] When this Digimon would digivolve into a card with the
        [Archangel] or [Fallen Angel] trait, reduce the memory cost of the
        digivolution by 2.
    Inherited Effect [End of Your Turn] This Digimon and any of your other
        Digimon may DNA digivolve into a Digimon card in the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] Reveal top 3, add 1 yellow + 1 purple Digimon ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("ST10-04 Reveal top 3 add yellow+purple")
        effect0.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 yellow "
            "Digimon card and 1 purple Digimon card among them to your hand. "
            "Place the remaining cards at the bottom of your deck in any order."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Reveal top 3, add 1 yellow Digimon and 1 purple Digimon to hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Reveal top 3
            revealed = []
            for _ in range(min(3, len(player.library_cards))):
                revealed.append(player.library_cards.pop(0))

            added_yellow = False
            added_purple = False
            to_hand = []
            to_bottom = []

            for c in revealed:
                colors = getattr(c, 'card_colors', []) or []
                color_names = [col.name for col in colors]
                is_digimon = getattr(c, 'is_digimon', False)
                if is_digimon and not added_yellow and 'Yellow' in color_names:
                    to_hand.append(c)
                    added_yellow = True
                elif is_digimon and not added_purple and 'Purple' in color_names:
                    to_hand.append(c)
                    added_purple = True
                else:
                    to_bottom.append(c)

            for c in to_hand:
                player.hand_cards.append(c)
            for c in to_bottom:
                player.library_cards.append(c)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Your Turn] Reduce digivolve cost by 2 for Archangel/Fallen Angel ---
        # This is a continuous effect that reduces digivolution cost.
        # Implemented as a cost_reduction marker; the engine checks for
        # cost_reduction on effects when calculating digivolution costs.
        effect1 = ICardEffect()
        effect1.set_effect_name("ST10-04 Archangel/Fallen Angel evo cost -2")
        effect1.set_effect_description(
            "[Your Turn] When this Digimon would digivolve into a card with "
            "the [Archangel] or [Fallen Angel] trait, reduce the digivolution "
            "cost by 2."
        )
        effect1.cost_reduction = 2

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check if the digivolving target has Archangel or Fallen Angel trait
            target_card = context.get('digivolving_card')
            if target_card:
                type_eng = getattr(target_card, 'type_eng', []) or []
                if any('Archangel' in t or 'Fallen Angel' in t for t in type_eng):
                    return True
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: Inherited [End of Your Turn] DNA digivolve from hand ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndTurn)
        effect2.set_effect_name("ST10-04 Inherited: End of turn DNA digivolve")
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
            """Trigger DNA digivolve from hand at end of turn."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def dna_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                entity = getattr(c, 'c_entity_base', None)
                if entity and entity.dna_costs:
                    return True
                return False
            game.effect_dna_digivolve_from_hand(player, dna_filter, is_optional=True)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
