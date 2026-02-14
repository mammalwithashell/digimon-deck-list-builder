from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_069(CardScript):
    """BT20-069 Punkmon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-069 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [Evil] trait for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_trait = "Evil"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Evil' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Trash 1 card in your hand. Then, until the end of your opponent's turn, 1 of your Digimon gains <Blocker> and <Retaliation>.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-069 Trash 1 card then, activate effects.")
        effect1.set_effect_description("[On Play] Trash 1 card in your hand. Then, until the end of your opponent's turn, 1 of your Digimon gains <Blocker> and <Retaliation>.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
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

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Trash 1 card in your hand. Then, until the end of your opponent's turn, 1 of your Digimon gains <Blocker> and <Retaliation>.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-069 Trash 1 card then, activate effects.")
        effect2.set_effect_description("[When Digivolving] Trash 1 card in your hand. Then, until the end of your opponent's turn, 1 of your Digimon gains <Blocker> and <Retaliation>.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
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

        # Factory effect: dp_modifier
        # DP modifier
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-069 DP modifier")
        effect3.set_effect_description("DP modifier")
        effect3.is_inherited_effect = True
        effect3.dp_modifier = 2000

        def condition3(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
