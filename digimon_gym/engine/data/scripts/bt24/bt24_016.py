from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_016(CardScript):
    """Auto-transpiled from DCGO BT24_016.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDeclaration
        # [Hand] [Main] If you have [Owen Dreadnought], by placing 1 [Dimetromon] from your trash as any of your [Elizamon]'s bottom digivolution card, it digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-016 Place 1 [Dimetromon] from trash under 1 [Elizamon], to digivolve for 3")
        effect0.set_effect_description("[Hand] [Main] If you have [Owen Dreadnought], by placing 1 [Dimetromon] from your trash as any of your [Elizamon]'s bottom digivolution card, it digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements.")

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            pass  # TODO: digivolve effect needs card selection

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-016 Opponent places 1 card from hand in security bottom. Trash their security top")
        effect1.set_effect_description("Effect")
        effect1.set_hash_string("WAWD_BT24-016")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-016 Opponent places 1 card from hand in security bottom. Trash their security top")
        effect2.set_effect_description("Effect")
        effect2.set_hash_string("WAWD_BT24-016")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnLoseSecurity
        # Play Card, Trash From Hand
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-016 Play 1 [Reptile] or [Dragonkin] from hand")
        effect3.set_effect_description("Play Card, Trash From Hand")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("PlayDigimon_BT24_016")

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
