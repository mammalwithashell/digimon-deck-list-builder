from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST10_04(CardScript):
    """ST10-04 Gatomon | Lv.4 Yellow/Purple Digimon (Holy Beast)

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

        # --- Effect 0: [On Play] Reveal top 3, add 1 yellow Digimon + 1 purple Digimon ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("ST10-04 Reveal top 3: add yellow + purple Digimon")
        effect0.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 yellow "
            "Digimon card and 1 purple Digimon card among them to your hand. "
            "Place the remaining cards at the bottom of your deck in any order."
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
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            from ....data.enums import CardColor

            def is_yellow_digimon(c) -> bool:
                if not getattr(c, 'is_digimon', False):
                    return False
                return CardColor.Yellow in (getattr(c, 'card_colors', []) or [])

            def is_purple_digimon(c) -> bool:
                if not getattr(c, 'is_digimon', False):
                    return False
                return CardColor.Purple in (getattr(c, 'card_colors', []) or [])

            game.effect_reveal_and_select_multi(
                player,
                count=3,
                passes=[
                    (is_yellow_digimon, 'hand'),
                    (is_purple_digimon, 'hand'),
                ],
                remaining_placement='deck_bottom',
                is_optional=False,
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Your Turn] Reduce digivolve cost by 2 when digivolving
        # this Digimon into a card with the [Archangel] or [Fallen Angel] trait ---
        #
        # Player.digivolve() scans WhenWouldDigivolve effects on every
        # CardSource in permanent.digivolution_cards. Because Gatomon will be
        # the current top_card at the moment of digivolution, a non-inherited
        # WhenWouldDigivolve effect on this card fires exactly at the right
        # moment. The engine reads effect.cost_reduction when the condition
        # passes and applies it to the base memory cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.WhenWouldDigivolve)
        effect1.set_effect_name("ST10-04 Digivolve cost -2 (Archangel/Fallen Angel)")
        effect1.set_effect_description(
            "[Your Turn] When this Digimon would digivolve into a card with "
            "the [Archangel] or [Fallen Angel] trait, reduce the memory cost "
            "of the digivolution by 2."
        )
        effect1.cost_reduction = 2

        def condition1(context: Dict[str, Any]) -> bool:
            # Field presence — Gatomon must actually be on the battle area
            if card and card.permanent_of_this_card() is None:
                return False
            # [Your Turn] — only during the owner's turn
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Only apply when THIS Gatomon is the permanent being digivolved.
            # Player.digivolve() sets context['permanent'] to the target perm.
            target_perm = context.get('permanent')
            if target_perm is not None and target_perm is not card.permanent_of_this_card():
                return False
            # Check the NEW card (card_source in the digivolve context) has
            # the [Archangel] or [Fallen Angel] trait.
            new_card = context.get('card_source')
            if new_card is None:
                return False
            traits = getattr(new_card, 'card_traits', []) or []
            if 'Archangel' in traits:
                return True
            if 'Fallen Angel' in traits:
                return True
            # DCGO also checks the unsplit variant 'FallenAngel'
            if 'FallenAngel' in traits:
                return True
            return False

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
            # Must be on the field (inherited: card is a source under a perm)
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            # [Your Turn] gating
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            owner = card.owner
            # Need at least 1 other Digimon on field to DNA with
            other_digimon = [
                p for p in owner.battle_area
                if p is not perm and p.is_digimon
            ]
            if not other_digimon:
                return False
            # Need at least 1 Digimon card in hand to DNA digivolve into
            has_hand_digimon = any(
                getattr(c, 'is_digimon', False)
                and getattr(c, 'c_entity_base', None) is not None
                and getattr(c.c_entity_base, 'dna_costs', None)
                for c in owner.hand_cards
            )
            return has_hand_digimon

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            game.effect_dna_digivolve_from_hand(
                player,
                filter_fn=lambda c: True,
                is_optional=True,
                prompt="Select a card from hand to DNA digivolve (end of turn).",
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
