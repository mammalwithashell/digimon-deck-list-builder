from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_027(CardScript):
    """BT23-027 Angemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req (Patamon variant)
        # Alternate digivolution requirement: from [Patamon] for cost 2
        # C# PermanentCondition = TopCard.EqualsCardName("Patamon")
        #                         || (IsLevel3 && HasCSTraits)
        # Split into two _alt_digi effects (engine validator does not call
        # can_use_condition for alt-digi; all constraints go on _alt_digi_*).
        effect0a = ICardEffect()
        effect0a.set_effect_name("BT23-027 Alt digi from Patamon")
        effect0a.set_effect_description("Alternate digivolution: from [Patamon] for cost 2")
        effect0a._alt_digi_cost = 2
        effect0a._alt_digi_name = "Patamon"

        def condition0a(context: Dict[str, Any]) -> bool:
            return True
        effect0a.set_can_use_condition(condition0a)
        effects.append(effect0a)

        # Factory effect: alt_digivolve_req (Lv.3 w/CS trait variant)
        # Alternate digivolution requirement: from Lv.3 w/[CS] trait for cost 2
        effect0b = ICardEffect()
        effect0b.set_effect_name("BT23-027 Alt digi from Lv.3 CS")
        effect0b.set_effect_description("Alternate digivolution: from Lv.3 w/[CS] trait for cost 2")
        effect0b._alt_digi_cost = 2
        effect0b._alt_digi_level = 3
        effect0b._alt_digi_trait = "CS"

        def condition0b(context: Dict[str, Any]) -> bool:
            return True
        effect0b.set_can_use_condition(condition0b)
        effects.append(effect0b)

        # Factory effect: barrier
        # Barrier
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-027 Barrier")
        effect1.set_effect_description("Barrier")
        effect1._is_barrier = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] <Draw 1>. Then if it's your turn, 2 of your Digimon may DNA digivolve into [Shakkoumon] in the hand.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT23-027 Draw 1. then if its your turn, you may DNA into [Shakkoumon] from hand")
        effect2.set_effect_description("[On Play] <Draw 1>. Then if it's your turn, 2 of your Digimon may DNA digivolve into [Shakkoumon] in the hand.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Draw 1, then DNA digivolve into Shakkoumon"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)
            if not (player and game):
                return
            if player.is_my_turn:
                def shakkoumon_filter(c):
                    return any('Shakkoumon' in n for n in getattr(c, 'card_names', []))
                game.effect_dna_digivolve_from_hand(
                    player, shakkoumon_filter, is_optional=True,
                    prompt="Select Shakkoumon from hand for DNA digivolve.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] <Draw 1>. Then if it's your turn, 2 of your Digimon may DNA digivolve into [Shakkoumon] in the hand.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT23-027 Draw 1. then if its your turn, you may DNA into [Shakkoumon] from hand")
        effect3.set_effect_description("[When Digivolving] <Draw 1>. Then if it's your turn, 2 of your Digimon may DNA digivolve into [Shakkoumon] in the hand.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Draw 1, then DNA digivolve into Shakkoumon"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)
            if not (player and game):
                return
            if player.is_my_turn:
                def shakkoumon_filter(c):
                    return any('Shakkoumon' in n for n in getattr(c, 'card_names', []))
                game.effect_dna_digivolve_from_hand(
                    player, shakkoumon_filter, is_optional=True,
                    prompt="Select Shakkoumon from hand for DNA digivolve.")

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Factory effect: barrier
        # Barrier
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-027 Barrier")
        effect4.set_effect_description("Barrier")
        effect4.is_inherited_effect = True
        effect4._is_barrier = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
