from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_035(CardScript):
    """BT23-035 Dynasmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-035 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 with [Witchelny] trait for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_trait = "Witchelny"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Witchelny' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: barrier
        # Barrier
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-035 Barrier")
        effect1.set_effect_description("Barrier")
        effect1._is_barrier = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By trashing your top security card, all of your opponent's Digimon get -6000 DP for the turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT23-035 By trashing your top security, -6000 all opponent digimon")
        effect2.set_effect_description("[On Play] By trashing your top security card, all of your opponent's Digimon get -6000 DP for the turn.")
        effect2.is_on_play = True

        effect2.is_optional = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # "By trashing" is a cost — must have security to pay
            if card.owner and len(card.owner.security_cards) > 0:
                return True
            return False

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Trash own top security, then all opponent Digimon get -6000 DP"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not player:
                return
            # Cost: Trash player's own top security card
            if player.security_cards:
                trashed = player.security_cards.pop(0)
                player.trash_cards.append(trashed)
            else:
                return
            # Effect: All opponent Digimon get -6000 DP for the turn
            enemy = player.enemy if player else None
            if enemy:
                for p in enemy.battle_area:
                    if p.is_digimon:
                        p.change_dp(-6000)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By trashing your top security card, all of your opponent's Digimon get -6000 DP for the turn.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT23-035 By trashing your top security, -6000 all opponent digimon")
        effect3.set_effect_description("[When Digivolving] By trashing your top security card, all of your opponent's Digimon get -6000 DP for the turn.")
        effect3.is_when_digivolving = True

        effect3.is_optional = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # "By trashing" is a cost — must have security to pay
            if card.owner and len(card.owner.security_cards) > 0:
                return True
            return False

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Trash own top security, then all opponent Digimon get -6000 DP"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not player:
                return
            # Cost: Trash player's own top security card
            if player.security_cards:
                trashed = player.security_cards.pop(0)
                player.trash_cards.append(trashed)
            else:
                return
            # Effect: All opponent Digimon get -6000 DP for the turn
            enemy = player.enemy if player else None
            if enemy:
                for p in enemy.battle_area:
                    if p.is_digimon:
                        p.change_dp(-6000)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnLoseSecurity
        # When your security stack is removed from, this Digimon gains <Security A. +1> until your turn ends. Then, if you have 3 or fewer security cards, <Recovery +1 (Deck)>
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnLoseSecurity)
        effect4.set_effect_name("BT23-035 Gain Sec +1. then if you are 3- security, Recovery")
        effect4.set_effect_description("When your security stack is removed from, this Digimon gains <Security A. +1> until your turn ends. Then, if you have 3 or fewer security cards, <Recovery +1 (Deck)>")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT23_035_AT")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Security Attack +1 until your turn ends, then conditional Recovery +1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return
            # Grant Security Attack +1 until your turn ends
            from digimon_gym.engine.interfaces.modifiers import ModifierType
            game.register_modifier(
                perm, ModifierType.CHANGE_SECURITY_ATTACK,
                value_fn=lambda current, target, ctx: current + 1,
                source_effect=effect,
                expiry='end_of_turn'
            )
            # Then, if you have 3 or fewer security cards, Recovery +1 (Deck)
            if len(player.security_cards) <= 3:
                player.recovery(1)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
