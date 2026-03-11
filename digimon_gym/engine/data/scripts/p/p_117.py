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
        # when digivolving into a [Free] trait Digimon, if you have a Tamer ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("P-117 Reduce digivolution cost by 1 (Free trait)")
        effect0.set_effect_description(
            "[Your Turn][Once Per Turn] When this Digimon would digivolve into "
            "a Digimon card with the [Free] trait, if you have a Tamer, reduce "
            "the digivolution cost by 1."
        )
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("DigivolutionCost-1_P_117")
        effect0.cost_reduction = 1

        def condition0(context: Dict[str, Any]) -> bool:
            # This is a digivolution cost reduction — the card_source key holds
            # the card being digivolved INTO. We apply the reduction only when:
            # 1. This card is on the field as the base being digivolved
            # 2. It is the owner's turn
            # 3. The target digivolution card has the [Free] trait
            # 4. The owner has at least one Tamer in play
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            if not owner.is_my_turn:
                return False
            # Check that the card whose cost is being calculated has [Free] trait
            target = context.get('card_source')
            if target is None:
                return False
            if not getattr(target, 'is_digimon', False):
                return False
            if not any('Free' in t for t in getattr(target, 'card_traits', [])):
                return False
            # Check that owner has a Tamer in play
            has_tamer = any(p.is_tamer for p in owner.battle_area)
            if not has_tamer:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            # Cost reduction handled via cost_reduction property
            pass

        effect0.set_on_process_callback(process0)
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
