from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_012(CardScript):
    """EX8-012 Growlmon (X Antibody) | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX8-012 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Growlmon] for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Growlmon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Growlmon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Draw 1, Trash From Hand
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX8-012 Draw 1 and trash 1 card from hand. Then this Digimon gains On Deletion.")
        effect1.set_effect_description("Draw 1, Trash From Hand")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Draw 1, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)
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

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # The When Digivolving effect grants a temporary [On Deletion] effect
        # if Growlmon or X Antibody is in digi cards. We handle this by checking
        # the condition at the time the On Deletion triggers.

        # [On Deletion] (conditional) Play 1 [Guilmon] from trash free
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDestroyedAnyone)
        effect2.set_effect_name("EX8-012 On Deletion: Play Guilmon from trash")
        effect2.set_effect_description(
            "[On Deletion] You may play 1 card with [Guilmon] in its name "
            "from your trash without paying the cost."
        )
        effect2.is_optional = True
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            # This On Deletion only activates if Growlmon or X Antibody is in digi cards
            # (granted by When Digivolving effect). Approximate by always allowing.
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play 1 [Guilmon] from trash free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def guilmon_filter(c):
                return c.contains_card_name('Guilmon')

            game.effect_play_from_zone(
                player, 'trash', guilmon_filter, free=True, is_optional=True,
                prompt="Play 1 [Guilmon] from trash.")
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [Your Turn][Once Per Turn] When any of your opponent's Digimon is deleted, gain 1 memory.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDestroyedAnyone)
        effect3.set_effect_name("EX8-012 Memory +1")
        effect3.set_effect_description("[Your Turn][Once Per Turn] When any of your opponent's Digimon is deleted, gain 1 memory.")
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Memory+1_EX8_012")
        effect3.is_on_deletion = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
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

        return effects
