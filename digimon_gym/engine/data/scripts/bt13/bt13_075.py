from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_075(CardScript):
    """BT13-075 Alphamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By placing 1 Digimon card with the [X Antibody] or [Royal Knight] trait from your trash as this Digimon's bottom digivolution card, all of your opponent's play cost 10 or higher Digimon can't attack players until the end of their turn.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT13-075 Place 1 card from trash to digivolution cards so that opponent Digimons can't attack")
        effect0.set_effect_description("[On Play] By placing 1 Digimon card with the [X Antibody] or [Royal Knight] trait from your trash as this Digimon's bottom digivolution card, all of your opponent's play cost 10 or higher Digimon can't attack players until the end of their turn.")
        effect0.is_optional = True
        effect0.is_on_play = True
        effect0._is_cannot_attack_player = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Restrict Attack, Gain Keyword Cannot Attack Player, Grant Cannot Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Attack restriction via modifier system
            if not (player and game):
                return
            from digimon_gym.engine.interfaces.modifiers import ModifierType
            def target_filter(p):
                return p.is_digimon
            def on_restrict(target_perm):
                game.register_modifier(
                    ModifierType.CANNOT_ATTACK, target_perm,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')
            game.effect_select_opponent_permanent(
                player, on_restrict, filter_fn=target_filter, is_optional=True)
            if perm:
                perm.grant_keyword('_is_cannot_attack_player')
            # Prevent target from attacking
            if not (player and game):
                return
            from digimon_gym.engine.interfaces.modifiers import ModifierType
            def on_restrict(target_perm):
                game.register_modifier(
                    ModifierType.CANNOT_ATTACK, target_perm,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')
            game.effect_select_opponent_permanent(
                player, on_restrict, filter_fn=lambda p: p.is_digimon, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By placing 1 Digimon card with the [X Antibody] or [Royal Knight] trait from your trash as this Digimon's bottom digivolution card, all of your opponent's play cost 10 or higher Digimon can't attack players until the end of their turn.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT13-075 Place 1 card from trash to digivolution cards so that opponent Digimons can't attack")
        effect1.set_effect_description("[When Digivolving] By placing 1 Digimon card with the [X Antibody] or [Royal Knight] trait from your trash as this Digimon's bottom digivolution card, all of your opponent's play cost 10 or higher Digimon can't attack players until the end of their turn.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True
        effect1._is_cannot_attack_player = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Restrict Attack, Gain Keyword Cannot Attack Player, Grant Cannot Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Attack restriction via modifier system
            if not (player and game):
                return
            from digimon_gym.engine.interfaces.modifiers import ModifierType
            def target_filter(p):
                return p.is_digimon
            def on_restrict(target_perm):
                game.register_modifier(
                    ModifierType.CANNOT_ATTACK, target_perm,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')
            game.effect_select_opponent_permanent(
                player, on_restrict, filter_fn=target_filter, is_optional=True)
            if perm:
                perm.grant_keyword('_is_cannot_attack_player')
            # Prevent target from attacking
            if not (player and game):
                return
            from digimon_gym.engine.interfaces.modifiers import ModifierType
            def on_restrict(target_perm):
                game.register_modifier(
                    ModifierType.CANNOT_ATTACK, target_perm,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')
            game.effect_select_opponent_permanent(
                player, on_restrict, filter_fn=lambda p: p.is_digimon, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns][Once Per Turn] When an effect would remove this Digimon from the battle area, by returning 1 card with the [X Antibody] or [Royal Knight] trait from this Digimon's digivolution cards to the bottom of the deck, prevent that removal.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenRemoveField)
        effect2.set_effect_name("BT13-075 Prevent this Digimon from leaving play")
        effect2.set_effect_description("[All Turns][Once Per Turn] When an effect would remove this Digimon from the battle area, by returning 1 card with the [X Antibody] or [Royal Knight] trait from this Digimon's digivolution cards to the bottom of the deck, prevent that removal.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Substitute_BT13_075")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
