from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_043(CardScript):
    """EX8-043 MetalTyrannomon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX8-043 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.4 with [Dinosaur] trait for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4
        effect0._alt_digi_trait = "Dinosaur"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Dinosaur' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may suspend 1 Digimon. Then, if this Digimon is suspended, <De-Digivolve 1> 1 of your opponent's Digimon , and this Digimon isn't returned to hand or deck by an opponent's effect, and isn't affected by <De-Digivolve> effects until the end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX8-043 Suspend 1 Digimon")
        effect1.set_effect_description("[On Play] You may suspend 1 Digimon. Then, if this Digimon is suspended, <De-Digivolve 1> 1 of your opponent's Digimon , and this Digimon isn't returned to hand or deck by an opponent's effect, and isn't affected by <De-Digivolve> effects until the end of your opponent's turn.")
        effect1.is_on_play = True
        effect1._is_cannot_return_to_hand = True
        effect1._is_cannot_return_to_deck = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: De Digivolve, Suspend, Gain Keyword Cannot Return To Hand, Gain Keyword Cannot Return To Deck, Grant Bounce Immunity"""
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
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)
            if perm:
                perm.grant_keyword('_is_cannot_return_to_hand')
                perm.grant_keyword('_is_cannot_return_to_deck')
            # Prevent return to hand/deck via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_RETURNED, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may suspend 1 Digimon. Then, if this Digimon is suspended, <De-Digivolve 1> 1 of your opponent's Digimon , and this Digimon isn't returned to hand or deck by an opponent's effect, and isn't affected by <De-Digivolve> effects until the end of your opponent's turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX8-043 Suspend 1 Digimon")
        effect2.set_effect_description("[When Digivolving] You may suspend 1 Digimon. Then, if this Digimon is suspended, <De-Digivolve 1> 1 of your opponent's Digimon , and this Digimon isn't returned to hand or deck by an opponent's effect, and isn't affected by <De-Digivolve> effects until the end of your opponent's turn.")
        effect2.is_when_digivolving = True
        effect2._is_cannot_return_to_hand = True
        effect2._is_cannot_return_to_deck = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: De Digivolve, Suspend, Gain Keyword Cannot Return To Hand, Gain Keyword Cannot Return To Deck, Grant Bounce Immunity"""
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
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)
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

        # Timing: EffectTiming.OnEndBattle
        # [All Turns] (Once Per Turn) When this Digimon deletes an opponent's Digimon in battle, trash the top card of your opponent's security stack.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX8-043 Trash the top card of opponent's security")
        effect3.set_effect_description("[All Turns] (Once Per Turn) When this Digimon deletes an opponent's Digimon in battle, trash the top card of your opponent's security stack.")
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("TrashSecurity_EX8_043")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
