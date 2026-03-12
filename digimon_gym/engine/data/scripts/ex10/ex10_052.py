from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_052(CardScript):
    """EX10-052 Lucemon: Chaos Mode | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-052 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.3 from [Lucemon] for cost 5
        effect0._alt_digi_cost = 5
        effect0._alt_digi_level = 3
        effect0._alt_digi_name = "Lucemon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Lucemon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By trashing 1 card in your hand, your opponent may delete 1 of their Digimon or Tamers. If this effect didn't delete, <Recovery +1 (Deck)> (Place the top card of your deck on top of your security stack.)
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX10-052 By trashing 1 card in hand, your opponent may delete a digimon or tamer. if they didnt Recover +1")
        effect1.set_effect_description("[When Digivolving] By trashing 1 card in your hand, your opponent may delete 1 of their Digimon or Tamers. If this effect didn't delete, <Recovery +1 (Deck)> (Place the top card of your deck on top of your security stack.)")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Trash From Hand"""
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
                player, hand_filter, on_trashed, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] By trashing 1 card in your hand, your opponent may delete 1 of their Digimon or Tamers. If this effect didn't delete, <Recovery +1 (Deck)> (Place the top card of your deck on top of your security stack.)
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("EX10-052 By trashing 1 card in hand, your opponent may delete a digimon or tamer. if they didnt Recover +1")
        effect2.set_effect_description("[When Attacking] By trashing 1 card in your hand, your opponent may delete 1 of their Digimon or Tamers. If this effect didn't delete, <Recovery +1 (Deck)> (Place the top card of your deck on top of your security stack.)")
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Trash From Hand"""
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
                player, hand_filter, on_trashed, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When this Digimon would leave the battle area, your opponent may delete 1 of their Digimon or Tamers. If this effect didn't delete, it doesn't leave.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.WhenRemoveField)
        effect3.set_effect_name("EX10-052 Your opponent may delete a digimon or tamer, if they dont this cards doesnt leave")
        effect3.set_effect_description("[All Turns] [Once Per Turn] When this Digimon would leave the battle area, your opponent may delete 1 of their Digimon or Tamers. If this effect didn't delete, it doesn't leave.")
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("EX10_052_RemoveField")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
