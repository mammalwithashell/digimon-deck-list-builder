from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_071(CardScript):
    """BT21-071 Scopemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-071 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [Stnd.] trait for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_trait = "Stnd."

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Stnd.' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT21-071 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 0
        effect1._alt_digi_cost = 0

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Three Musketeers' in text):
                    return False
            else:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By placing 1 card with the [Appmon] or [Three Musketeers] trait from your hand or trash as 1 of your Digimon's bottom digivolution card, gain 1 memory.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT21-071 Tuck appmon/muskets for memory")
        effect2.set_effect_description("[On Play] By placing 1 card with the [Appmon] or [Three Musketeers] trait from your hand or trash as 1 of your Digimon's bottom digivolution card, gain 1 memory.")
        effect2.is_optional = True
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By placing 1 card with the [Appmon] or [Three Musketeers] trait from your hand or trash as 1 of your Digimon's bottom digivolution card, gain 1 memory.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT21-071 Tuck appmon/muskets for memory")
        effect3.set_effect_description("[When Digivolving] By placing 1 card with the [Appmon] or [Three Musketeers] trait from your hand or trash as 1 of your Digimon's bottom digivolution card, gain 1 memory.")
        effect3.is_optional = True
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.WhenLinked
        # [When Linking] <Draw 2> and trash 2 cards in your hand.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT21-071 Trash 2 draw 2")
        effect4.set_effect_description("[When Linking] <Draw 2> and trash 2 cards in your hand.")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Draw 2, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(2)
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
