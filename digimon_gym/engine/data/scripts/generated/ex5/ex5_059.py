from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_059(CardScript):
    """EX5-059 Dobermon (X Antibody) | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX5-059 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 0
        effect0._alt_digi_cost = 0

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your Digimon gains <Retaliation> (When this Digimon is deleted after losing a battle, delete the Digimon it was battling), until the end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX5-059 Your 1 Digimon gains Retaliation")
        effect1.set_effect_description("[On Play] 1 of your Digimon gains <Retaliation> (When this Digimon is deleted after losing a battle, delete the Digimon it was battling), until the end of your opponent's turn.")
        effect1.is_on_play = True
        effect1._is_retaliation = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain Keyword Retaliation"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_retaliation')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] <Draw 1> (Draw 1 card from your deck). Then, trash 1 card in your hand. If [Dobermon] in its name or [X Antibody] is in this Digimon's digivolution cards, activate this Digimon's [On Play] effects.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX5-059 Draw 1, trash 1 card from hand and activate [On Play] effect")
        effect2.set_effect_description("[When Digivolving] <Draw 1> (Draw 1 card from your deck). Then, trash 1 card in your hand. If [Dobermon] in its name or [X Antibody] is in this Digimon's digivolution cards, activate this Digimon's [On Play] effects.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Draw 1, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Dobermon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] [Once Per Turn] When an effect plays one of your Digimon, gain 1 memory.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX5-059 Memory +1")
        effect3.set_effect_description("[Your Turn] [Once Per Turn] When an effect plays one of your Digimon, gain 1 memory.")
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Memory1_EX5_059")
        effect3.is_on_play = True

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
