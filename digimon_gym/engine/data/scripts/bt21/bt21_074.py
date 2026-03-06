from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_074(CardScript):
    """BT21-074 Satellamon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-074 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Three Musketeers' in text):
                    return False
            else:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT21-074 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [Sup.] trait for cost 4
        effect1._alt_digi_cost = 3
        effect1._alt_digi_trait = "Sup."

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Sup.' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By placing 1 [Appmon] or [Three Musketeers] trait card from your hand or trash as any of your Digimon's bottom digivolution cards, until your opponent's turn ends, their effects can't return that Digimon to hand or decks or affect it with <De-Digivolve> effects.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT21-074 Tuck to get protections")
        effect2.set_effect_description("[On Play] By placing 1 [Appmon] or [Three Musketeers] trait card from your hand or trash as any of your Digimon's bottom digivolution cards, until your opponent's turn ends, their effects can't return that Digimon to hand or decks or affect it with <De-Digivolve> effects.")
        effect2.is_on_play = True
        effect2._is_cannot_return_to_hand = True
        effect2._is_cannot_return_to_deck = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain Keyword Cannot Return To Hand, Gain Keyword Cannot Return To Deck, Grant Bounce Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_cannot_return_to_hand')
                perm.grant_keyword('_is_cannot_return_to_deck')
            # Prevent return to hand/deck via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_RETURNED, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By placing 1 [Appmon] or [Three Musketeers] trait card from your hand or trash as any of your Digimon's bottom digivolution cards, until your opponent's turn ends, their effects can't return that Digimon to hand or decks or affect it with <De-Digivolve> effects.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT21-074 Tuck to get protections")
        effect3.set_effect_description("[When Digivolving] By placing 1 [Appmon] or [Three Musketeers] trait card from your hand or trash as any of your Digimon's bottom digivolution cards, until your opponent's turn ends, their effects can't return that Digimon to hand or decks or affect it with <De-Digivolve> effects.")
        effect3.is_when_digivolving = True
        effect3._is_cannot_return_to_hand = True
        effect3._is_cannot_return_to_deck = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Gain Keyword Cannot Return To Hand, Gain Keyword Cannot Return To Deck, Grant Bounce Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_cannot_return_to_hand')
                perm.grant_keyword('_is_cannot_return_to_deck')
            # Prevent return to hand/deck via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_RETURNED, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving][Once Per Turn] By trashing 1 card with the [Appmon] or [Three Musketeers] trait from your Digimon's digivolution cards, <De-Digivolve 1> 1 of your opponent's Digimon.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT21-074 Trash source to de-digivolve")
        effect4.set_effect_description("[When Digivolving][Once Per Turn] By trashing 1 card with the [Appmon] or [Three Musketeers] trait from your Digimon's digivolution cards, <De-Digivolve 1> 1 of your opponent's Digimon.")
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT21_074De-digivolve")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards, De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnAllyAttack
        # [When Digivolving][Once Per Turn] By trashing 1 card with the [Appmon] or [Three Musketeers] trait from your Digimon's digivolution cards, <De-Digivolve 1> 1 of your opponent's Digimon.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnAllyAttack)
        effect5.set_effect_name("BT21-074 Trash source to de-digivolve")
        effect5.set_effect_description("[When Digivolving][Once Per Turn] By trashing 1 card with the [Appmon] or [Three Musketeers] trait from your Digimon's digivolution cards, <De-Digivolve 1> 1 of your opponent's Digimon.")
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BT21_074De-digivolve")
        effect5.is_on_attack = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            # Triggered on attack — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards, De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=True)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.WhenLinked
        # [When Linking] Delete 1 of your opponent's level 4 or lower Digimon.
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.WhenLinked)
        effect6.set_effect_name("BT21-074 Delete level 4")
        effect6.set_effect_description("[When Linking] Delete 1 of your opponent's level 4 or lower Digimon.")

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 4:
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
