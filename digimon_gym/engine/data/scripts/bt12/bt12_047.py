from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT12_047(CardScript):
    """BT12-047 Wormmon | Lv.3 (Green, DP 1000, Cost 3)

    [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with
    [Imperialdramon] in its name or a [Free] trait and 1 Tamer card with
    [Ken Ichijoji] in its name among them to your hand. Place the remaining
    cards at the bottom of your deck in any order.

    Inherited: [End of Your Turn] This Digimon and any of your other Digimon
    may DNA digivolve into a Digimon card in the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] Reveal top 3, add Imperialdramon/Free + Ken Ichijoji ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT12-047 Reveal top 3: add Imperialdramon/Free + Ken Ichijoji")
        effect0.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with "
            "[Imperialdramon] in its name or a [Free] trait and 1 Tamer card with "
            "[Ken Ichijoji] in its name among them to your hand. Place the remaining "
            "cards at the bottom of your deck in any order."
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

            def is_imperialdramon_or_free(c) -> bool:
                if not getattr(c, 'is_digimon', False):
                    return False
                # Check name contains Imperialdramon
                if any('Imperialdramon' in n for n in getattr(c, 'card_names', [])):
                    return True
                # Check [Free] trait — stored in attribute_eng, not card_traits/type_eng
                attrs = c.c_entity_base.attribute_eng if c.c_entity_base else []
                if 'Free' in attrs:
                    return True
                return False

            def is_ken_ichijoji_tamer(c) -> bool:
                if not getattr(c, 'is_tamer', False):
                    return False
                if any('Ken Ichijoji' in n for n in getattr(c, 'card_names', [])):
                    return True
                return False

            game.effect_reveal_and_select_multi(
                player,
                count=3,
                passes=[
                    (is_imperialdramon_or_free, 'hand'),
                    (is_ken_ichijoji_tamer, 'hand'),
                ],
                remaining_placement='deck_bottom',
                is_optional=False,
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Inherited [End of Your Turn] DNA digivolve from hand ---
        effect_inh = ICardEffect()
        effect_inh.set_timing(EffectTiming.OnEndTurn)
        effect_inh.set_effect_name("BT12-047 End of turn DNA digivolve (inherited)")
        effect_inh.set_effect_description(
            "[End of Your Turn] This Digimon and any of your other Digimon "
            "may DNA digivolve into a Digimon card in the hand."
        )
        effect_inh.is_inherited_effect = True
        effect_inh.is_optional = True

        def condition_inh(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            perm = card.permanent_of_this_card()
            if not perm:
                return False
            owner = card.owner
            other_digimon = [p for p in owner.battle_area if p is not perm and p.is_digimon]
            if not other_digimon:
                return False
            hand_digimon = [c for c in owner.hand_cards if getattr(c, 'is_digimon', False)]
            if not hand_digimon:
                return False
            return True

        effect_inh.set_can_use_condition(condition_inh)

        def process_inh(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            game.effect_dna_digivolve_from_hand(
                player,
                filter_fn=lambda c: True,
                is_optional=True,
            )

        effect_inh.set_on_process_callback(process_inh)
        effects.append(effect_inh)

        return effects
