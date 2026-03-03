from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_036(CardScript):
    """EX10-036 Magneticdramon | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-036 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 6
        effect0._alt_digi_name = "Close"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Close') or permanent.contains_card_name('Proganomon'))):
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-036 Fragment")
        effect1.set_effect_description("Fragment")
        effect1._is_fragment = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        def build_unsuspend_effect(is_when_digivolving: bool = False, is_on_attack: bool = False):
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone if is_when_digivolving else EffectTiming.OnAllyAttack)
            effect.set_effect_name("EX10-036 Place 3 cards from trash as bottom sources")
            effect.set_effect_description(
                "[When Digivolving] [Once Per Turn] By placing 3 [Mineral] or [Rock] trait cards from your trash as this Digimon's bottom digivolution cards, it unsuspends."
                if is_when_digivolving else
                "[When Attacking] [Once Per Turn] By placing 3 [Mineral] or [Rock] trait cards from your trash as this Digimon's bottom digivolution cards, it unsuspends."
            )
            effect.is_optional = True
            effect.set_max_count_per_turn(1)
            effect.set_hash_string("WDWA_EX10_036")
            effect.is_when_digivolving = is_when_digivolving
            effect.is_on_attack = is_on_attack

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                perm = ctx.get('permanent')
                if perm:
                    perm.unsuspend()

            effect.set_on_process_callback(process)
            return effect

        effects.append(build_unsuspend_effect(is_when_digivolving=True))
        effects.append(build_unsuspend_effect(is_on_attack=True))

        def build_delete_effect(is_when_digivolving: bool = False, is_on_attack: bool = False):
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone if is_when_digivolving else EffectTiming.OnAllyAttack)
            effect.set_effect_name("EX10-036 By trashing 3 sources, Delete 1 digimon and trash top security")
            effect.set_effect_description(
                "[When Digivolving] By trashing 3 [Mineral] or [Rock] trait cards from any of your Digimon's digivolution cards, delete 1 of your opponent's Digimon and trash their top security card."
                if is_when_digivolving else
                "[When Attacking] By trashing 3 [Mineral] or [Rock] trait cards from any of your Digimon's digivolution cards, delete 1 of your opponent's Digimon and trash their top security card."
            )
            effect.is_optional = True
            effect.is_when_digivolving = is_when_digivolving
            effect.is_on_attack = is_on_attack

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and perm and game):
                    return

                def target_filter(p):
                    return p.is_digimon

                def on_delete(target_perm):
                    enemy = player.enemy if player else None
                    if enemy:
                        enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter, is_optional=True)
                if not perm.has_no_digivolution_cards:
                    trashed = perm.trash_digivolution_cards(3)
                    player.trash_cards.extend(trashed)
                enemy = player.enemy if player else None
                if enemy and enemy.security_cards:
                    trashed = enemy.security_cards.pop(0)
                    enemy.trash_cards.append(trashed)

            effect.set_on_process_callback(process)
            return effect

        effects.append(build_delete_effect(is_when_digivolving=True))
        effects.append(build_delete_effect(is_on_attack=True))

        return effects
