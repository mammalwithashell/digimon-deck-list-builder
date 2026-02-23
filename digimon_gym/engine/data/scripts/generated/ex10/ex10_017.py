from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_017(CardScript):
    """EX10-017 Mienumon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-017 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: jamming
        # Jamming
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-017 Jamming")
        effect1.set_effect_description("Jamming")
        effect1._is_jamming = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: retaliation
        # Retaliation
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-017 Retaliation")
        effect2.set_effect_description("Retaliation")
        effect2._is_retaliation = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.WhenLinked
        # [Your Turn] [Once Per Turn] When this Digimon gets linked, if you have 1 or fewer Tamers, you may play 1 Tamer card with the [Leviathan] trait from your hand without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-017 Play 1 Tamer with [Leviathan] trait")
        effect3.set_effect_description("[Your Turn] [Once Per Turn] When this Digimon gets linked, if you have 1 or fewer Tamers, you may play 1 Tamer card with the [Leviathan] trait from your hand without paying the cost.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("EX10_017_WhenLinked")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] When any of your opponent's Digimon suspend, by trashing 1 of this Digimon's link cards, <Draw 1> (Draw 1 card from your deck.) and gain 1 memory.
        effect4 = ICardEffect()
        effect4.set_effect_name("EX10-017 By trashing 1 link card, draw 1 & gain 1 memory")
        effect4.set_effect_description("[All Turns] When any of your opponent's Digimon suspend, by trashing 1 of this Digimon's link cards, <Draw 1> (Draw 1 card from your deck.) and gain 1 memory.")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Draw 1, Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)
            if player:
                player.add_memory(1)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
