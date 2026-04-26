from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_074(CardScript):
    """EX5-074 Fanglongmon | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX5-074 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 4
        effect0._alt_digi_cost = 4
        effect0._alt_digi_level = 6
        effect0._alt_digi_trait = "Four Sovereigns"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By returning up to 4 cards with the [Deva]/[Four Sovereigns] trait from your trash to the bottom of the deck, for each one, all your opponent's Digimon get -4000 DP for the turn.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX5-074 Return cards from trash to the bottom of deck to reduce opponent's Digimon's DP")
        effect1.set_effect_description("[On Play] By returning up to 4 cards with the [Deva]/[Four Sovereigns] trait from your trash to the bottom of the deck, for each one, all your opponent's Digimon get -4000 DP for the turn.")
        effect1.is_optional = True
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Return To Deck"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (any('Deva' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('Four Sovereigns' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('FourSovereigns' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return True
            def on_return(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)
            game.effect_select_opponent_permanent(
                player, on_return, filter_fn=target_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] By returning up to 4 cards with the [Deva]/[Four Sovereigns] trait from your trash to the bottom of the deck, for each one, all your opponent's Digimon get -4000 DP for the turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("EX5-074 Return cards from trash to the bottom of deck to reduce opponent's Digimon's DP")
        effect2.set_effect_description("[When Attacking] By returning up to 4 cards with the [Deva]/[Four Sovereigns] trait from your trash to the bottom of the deck, for each one, all your opponent's Digimon get -4000 DP for the turn.")
        effect2.is_optional = True
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Return To Deck"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (any('Deva' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('Four Sovereigns' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('FourSovereigns' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return True
            def on_return(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)
            game.effect_select_opponent_permanent(
                player, on_return, filter_fn=target_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] For each of your Digimon with the [Four Sovereigns] trait, trash the top card of your opponent's security stack.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUseAttack)
        effect3.set_effect_name("EX5-074 Trash the top cards of opponent's security")
        effect3.set_effect_description("[When Attacking] For each of your Digimon with the [Four Sovereigns] trait, trash the top card of your opponent's security stack.")
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.None
        # Effect Immunity
        effect4 = ICardEffect()
        effect4.set_effect_name("EX5-074 Isn't affected by opponent's Digimon's effects")
        effect4.set_effect_description("Effect Immunity")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant effect immunity via modifier system
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
