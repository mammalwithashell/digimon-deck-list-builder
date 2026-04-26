from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_060(CardScript):
    """BT16-060 Tankdramon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 3 cards of your deck. For each card with the [D-Brigade] or [DigiPolice] trait among them, reduce the play cost of all of your opponent's Digimon by 1 for the turn. Return the revealed cards to the top or bottom of the deck. Then, delete 1 of your opponent's Digimon with a play cost of 4 or less.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-060 Reduce opponent's Digimon cost, then delete a 6 cost or less Digimon")
        effect0.set_effect_description("[On Play] Reveal the top 3 cards of your deck. For each card with the [D-Brigade] or [DigiPolice] trait among them, reduce the play cost of all of your opponent's Digimon by 1 for the turn. Return the revealed cards to the top or bottom of the deck. Then, delete 1 of your opponent's Digimon with a play cost of 4 or less.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (any('D-Brigade' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('DigiPolice' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            if not (player and game):
                return
            def reveal_filter(c):
                if not (any('D-Brigade' in _t or 'DigiPolice' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 4, reveal_filter, on_revealed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Reveal the top 3 cards of your deck. For each card with the [D-Brigade] or [DigiPolice] trait among them, reduce the play cost of all of your opponent's Digimon by 1 for the turn. Return the revealed cards to the top or bottom of the deck. Then, delete 1 of your opponent's Digimon with a play cost of 4 or less.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT16-060 Reduce opponent's Digimon cost, then delete a 4 cost or less Digimon")
        effect1.set_effect_description("[When Digivolving] Reveal the top 3 cards of your deck. For each card with the [D-Brigade] or [DigiPolice] trait among them, reduce the play cost of all of your opponent's Digimon by 1 for the turn. Return the revealed cards to the top or bottom of the deck. Then, delete 1 of your opponent's Digimon with a play cost of 4 or less.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (any('D-Brigade' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('DigiPolice' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            if not (player and game):
                return
            def reveal_filter(c):
                if not (any('D-Brigade' in _t or 'DigiPolice' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 4, reveal_filter, on_revealed, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns] [Once Per Turn] When one of your other Digimon with the [D-Brigade] or [DigiPolice] trait is played, <De-Digivolve 1> 1 of your opponent's Digimon.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT16-060 De-Digivolve 1")
        effect2.set_effect_description("[All Turns] [Once Per Turn] When one of your other Digimon with the [D-Brigade] or [DigiPolice] trait is played, <De-Digivolve 1> 1 of your opponent's Digimon.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("De-Digivolve_BT16_060")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
