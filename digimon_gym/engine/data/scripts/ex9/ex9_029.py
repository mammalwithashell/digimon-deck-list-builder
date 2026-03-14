from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_029(CardScript):
    """EX9-029 Unimon | Lv.4 | Mythical Beast/DM/Ver.3
    <Training>
    [When Digivolving][When Attacking][Once Per Turn] By placing 1 hand card as
    bottom digi card, if security <= face-down digi cards, Recovery +1 (Deck).
    Inherited: [When Attacking][Once Per Turn] -2000 DP to 1 opponent Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Training>
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-029 Training")
        effect0.set_effect_description("<Training>")
        effect0._is_training = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [When Digivolving][Once Per Turn] Place hand, conditional Recovery
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX9-029 When Digivolving: Place hand, Recovery")
        effect1.set_effect_description(
            "[When Digivolving][Once Per Turn] Place 1 hand card as bottom digi "
            "card, if security <= face-down digi cards, Recovery +1."
        )
        effect1.is_when_digivolving = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("place_recovery_EX9_029")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def _place_and_recover(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            if not player.hand_cards:
                return

            def on_hand_selected(selected_card):
                if selected_card is None:
                    return
                if selected_card in player.hand_cards:
                    player.hand_cards.remove(selected_card)
                    perm.add_card_source_bottom(selected_card)
                face_down_count = len(perm.digivolution_cards)
                if len(player.security_cards) <= face_down_count:
                    if player.library_cards:
                        top_card = player.library_cards.pop(0)
                        player.security_cards.append(top_card)

            game.effect_select_hand_card(
                player, lambda c: True, on_hand_selected, is_optional=True,
                prompt="Select 1 card to place face down as bottom digivolution card.")

        effect1.set_on_process_callback(_place_and_recover)
        effects.append(effect1)

        # [When Attacking][Once Per Turn] Same effect
        effect1b = ICardEffect()
        effect1b.set_timing(EffectTiming.OnTappedAnyone)
        effect1b.set_effect_name("EX9-029 When Attacking: Place hand, Recovery")
        effect1b.set_effect_description(
            "[When Attacking][Once Per Turn] Place 1 hand card as bottom digi "
            "card, if security <= face-down digi cards, Recovery +1."
        )
        effect1b.is_on_attack = True
        effect1b.set_max_count_per_turn(1)
        effect1b.set_hash_string("place_recovery_EX9_029")

        def condition1b(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            return True
        effect1b.set_can_use_condition(condition1b)
        effect1b.set_on_process_callback(_place_and_recover)
        effects.append(effect1b)

        # Inherited: [When Attacking][Once Per Turn] -2000 DP to 1 opponent
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnTappedAnyone)
        effect2.set_effect_name("EX9-029 Inherited: When Attacking -2000 DP")
        effect2.set_effect_description(
            "Inherited: [When Attacking][Once Per Turn] -2000 DP to 1 opponent Digimon."
        )
        effect2.is_inherited_effect = True
        effect2.is_on_attack = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("dp_minus_EX9_029_inh")

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon

            def on_reduce(target_perm):
                target_perm.dp_modifier -= 2000

            game.effect_select_opponent_permanent(
                player, on_reduce, filter_fn=target_filter, is_optional=False)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
