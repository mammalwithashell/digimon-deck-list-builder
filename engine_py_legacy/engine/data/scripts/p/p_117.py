from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_117(CardScript):
    """P-117 Veemon | Lv.3 (Blue, DP 1000, Cost 3)

    [Your Turn] [Once Per Turn] When this Digimon would digivolve into a
    Digimon card with the [Free] trait, if you have a Tamer, reduce the
    digivolution cost by 1.

    Inherited: [When Attacking] If this Digimon has 2 or more colors, <Draw 1>.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Your Turn] [Once Per Turn] Reduce digivolution cost by 1
        # when digivolving into a [Free] trait card, if you have a Tamer ---
        # Uses WhenWouldDigivolve timing (scanned by Player.digivolve()),
        # NOT BeforePayCost (which is for play cost only).
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.WhenWouldDigivolve)
        effect0.set_effect_name("P-117 Reduce digivolution cost by 1 (Free trait)")
        effect0.set_effect_description(
            "[Your Turn][Once Per Turn] When this Digimon would digivolve into "
            "a card with the [Free] trait, if you have a Tamer, reduce "
            "the digivolution cost by 1."
        )
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("DigivolutionCost-1_P_117")
        effect0.cost_reduction = 1

        def condition0(context: Dict[str, Any]) -> bool:
            # This is a digivolution cost reduction. Context keys from
            # Player.digivolve():
            #   "permanent" = the base permanent being digivolved
            #   "card_source" = the card being digivolved INTO
            #   "player" = the owner
            #
            # Conditions (per C# reference):
            # 1. This card must be on the field (permanent_of_this_card not None)
            # 2. The digivolving permanent must be THIS card's permanent
            # 3. It must be the owner's turn
            # 4. The target card must have the [Free] trait (attribute_eng)
            # 5. The owner must have at least one Tamer in play
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            # Check that THIS permanent is the one digivolving
            digivolving_perm = context.get('permanent')
            if digivolving_perm is not perm:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            if not owner.is_my_turn:
                return False
            # Check that the target card has the [Free] trait
            # Note: "Free" is in attribute_eng, NOT type_eng (card_traits)
            target = context.get('card_source')
            if target is None:
                return False
            target_attrs = (target.c_entity_base.attribute_eng
                            if target and target.c_entity_base else [])
            if 'Free' not in target_attrs:
                return False
            # Check that owner has a Tamer in play
            has_tamer = any(p.is_tamer for p in owner.battle_area)
            if not has_tamer:
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Inherited Effect: [When Attacking] If this Digimon has 2 or more
        # colors, <Draw 1> ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("P-117 Inherited: Draw 1 if 2+ colors")
        effect1.set_effect_description(
            "[When Attacking] If this Digimon has 2 or more colors, "
            "<Draw 1>. (Draw 1 card from your deck.)"
        )
        effect1.is_inherited_effect = True
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            # Check that this Digimon has 2 or more colors
            top = perm.top_card
            if top is None:
                return False
            colors = getattr(top, 'card_colors', None) or []
            if len(colors) < 2:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.draw_cards(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
