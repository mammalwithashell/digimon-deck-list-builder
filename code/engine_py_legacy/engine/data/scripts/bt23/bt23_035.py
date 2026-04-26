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
        # Alternate digivolution: Lv.5 with [Witchelny] trait (or CS trait) for cost 3
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-035 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_trait = "Witchelny"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not permanent or not permanent.top_card:
                return False
            traits = getattr(permanent.top_card, 'card_traits', []) or []
            level = getattr(permanent.top_card, 'level', None)
            if level != 5:
                return False
            return (
                any('Witchelny' in tr for tr in traits)
                or any('CS' in tr for tr in traits)
            )

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: barrier
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-035 Barrier")
        effect1.set_effect_description("Barrier")
        effect1._is_barrier = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        def _shared_dp_debuff(ctx: Dict[str, Any]):
            """Cost: trash top security. Effect: all opponent Digimon -6000 DP for the turn."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not player:
                return
            if not player.security_cards:
                return
            # Cost: trash the top security card (index 0)
            top_security = player.security_cards[0]
            player.trash_security_card(top_security)
            # Effect: all opponent Digimon get -6000 DP for the turn via modifier
            enemy = player.enemy if player else None
            if enemy and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                for p in enemy.battle_area:
                    if p.is_digimon:
                        game.register_modifier(
                            p, ModifierType.CHANGE_DP,
                            value_fn=lambda: -6000,
                            expiry='end_of_turn',
                        )

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By trashing your top security card, all of your opponent's Digimon get -6000 DP for the turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT23-035 By trashing your top security, -6000 all opponent digimon")
        effect2.set_effect_description("[On Play] By trashing your top security card, all of your opponent's Digimon get -6000 DP for the turn.")
        effect2.is_on_play = True
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # "By trashing" is a cost — must have security to pay
            if card.owner and len(card.owner.security_cards) > 0:
                return True
            return False

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            _shared_dp_debuff(ctx)

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

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # "By trashing" is a cost — must have security to pay
            if card.owner and len(card.owner.security_cards) > 0:
                return True
            return False

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            _shared_dp_debuff(ctx)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns] [Once Per Turn] When your security stack is removed from, this Digimon gains
        # <Security A. +1> until your turn ends. Then, if you have 3 or fewer security cards,
        # <Recovery +1 (Deck)>.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnLoseSecurity)
        effect4.set_effect_name("BT23-035 Gain Sec +1. then if you are 3- security, Recovery")
        effect4.set_effect_description("When your security stack is removed from, this Digimon gains <Security A. +1> until your turn ends. Then, if you have 3 or fewer security cards, <Recovery +1 (Deck)>")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT23_035_AT")

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only fires when YOUR security is removed
            losing_player = context.get('player')
            owner = card.owner if card else None
            if losing_player is not None and owner is not None and losing_player is not owner:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Security Attack +1 until your turn ends, then conditional Recovery +1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            my_perm = card.permanent_of_this_card() if card else None
            if my_perm is None:
                return
            # Grant Security Attack +1 until your turn ends
            from engine_py_legacy.engine.interfaces.modifiers import ModifierType
            game.register_modifier(
                my_perm, ModifierType.CHANGE_SECURITY_ATTACK,
                value_fn=lambda current, target, ctx2: current + 1,
                source_effect=effect4,
                expiry='end_of_turn',
            )
            # Then, if you have 3 or fewer security cards, Recovery +1 (Deck)
            owner = card.owner if card else None
            if owner and len(owner.security_cards) <= 3:
                owner.recovery(1)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
