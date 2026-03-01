from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_077(CardScript):
    """BT10-077 MadLeomon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAddHand
        # [Opponent's Turn][Once Per Turn] When an effect adds cards to your opponent's hand, by trashing 1 of this Digimon's digivolution cards, your opponent trashes cards in their hand equal to the number of cards added to their hand by the effect.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnAddHand)
        effect0.set_effect_name("BT10-077 Opponent discarads hand")
        effect0.set_effect_description("[Opponent's Turn][Once Per Turn] When an effect adds cards to your opponent's hand, by trashing 1 of this Digimon's digivolution cards, your opponent trashes cards in their hand equal to the number of cards added to their hand by the effect.")
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Discard_BT10_077")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash From Hand, Trash Digivolution Cards"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: save
        # Save
        effect1 = ICardEffect()
        effect1.set_effect_name("BT10-077 Save")
        effect1.set_effect_description("Save")
        effect1.is_on_deletion = True
        effect1._is_save = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDigivolutionCardDiscarded
        # [Opponent's Turn] When an effect trashes this digivolution card, gain 1 memory.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDigivolutionCardDiscarded)
        effect2.set_effect_name("BT10-077 Memory +1")
        effect2.set_effect_description("[Opponent's Turn] When an effect trashes this digivolution card, gain 1 memory.")
        effect2.is_inherited_effect = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
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

        # Timing: EffectTiming.None
        # Effect
        effect3 = ICardEffect()
        effect3.set_effect_name("BT10-077 Effect")
        effect3.set_effect_description("Effect")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
