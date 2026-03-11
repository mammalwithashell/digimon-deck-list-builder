from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_020(CardScript):
    """BT20-020 Imperialdramon: Fighter Mode | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: from [Imperialdramon: Dragon Mode] for cost 2
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-020 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_name = "Imperialdramon: Dragon Mode"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Imperialdramon: Dragon Mode'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: raid
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-020 Raid")
        effect1.set_effect_description("Raid")
        effect1.is_on_attack = True
        effect1._is_raid = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: piercing (Fix #1 - was missing)
        effect_piercing = ICardEffect()
        effect_piercing.set_effect_name("BT20-020 Piercing")
        effect_piercing.set_effect_description("Piercing")
        effect_piercing._is_piercing = True

        def condition_piercing(context: Dict[str, Any]) -> bool:
            return True
        effect_piercing.set_can_use_condition(condition_piercing)
        effects.append(effect_piercing)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Your opponent can't play Digimon or Tamers by effects
        # until the end of their turn. Then, if [Imperialdramon: Dragon Mode] is in
        # this Digimon's digivolution cards, trash your opponent's top security card.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT20-020 Play restriction + conditional security trash")
        effect2.set_effect_description("[When Digivolving] Your opponent can't play Digimon or Tamers by effects until the end of their turn. Then, if [Imperialdramon: Dragon Mode] is in this Digimon's digivolution cards, trash your opponent's top security card.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play restriction + conditional security trash"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            # Fix #2: Register play restriction modifier on opponent
            # "Your opponent can't play Digimon or Tamers by effects until end of their turn"
            # Uses CANNOT_PLAY_CARD modifier with end_of_opponent_turn expiry
            game.register_modifier(
                perm,
                ModifierType.CANNOT_PLAY_CARD,
                condition=lambda target_perm, ctx: True,
                source_effect=effect2,
                expiry='end_of_opponent_turn',
            )

            # Fix #3: Only trash security if "Imperialdramon: Dragon Mode" is in
            # this Digimon's digivolution cards (card_sources[:-1])
            has_dragon_mode = False
            if len(perm.card_sources) > 1:
                for cs in perm.card_sources[:-1]:
                    if cs.contains_card_name('Imperialdramon: Dragon Mode'):
                        has_dragon_mode = True
                        break

            if has_dragon_mode:
                # Fix #4: Use proper engine method instead of raw list manipulation
                enemy = player.enemy if player else None
                if enemy and enemy.security_cards:
                    enemy.trash_security_card(enemy.security_cards[0])

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns][Once Per Turn] When your opponent's security stack is removed
        # from, delete 1 of their Digimon with as much or less DP as this Digimon.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnLoseSecurity)
        effect3.set_effect_name("BT20-020 Delete 1 Digimon with as much or less DP as this Digimon")
        effect3.set_effect_description("[All Turns] [Once Per Turn] When your opponent's security stack is removed from, delete 1 of their Digimon with as much or less DP as this Digimon.")
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("ImperialDramon_BT20_020")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Fix #5: Only trigger when the OPPONENT loses security, not the card owner.
            # OnLoseSecurity fires with extra_context {"player": <player who lost security>}
            # which gets mapped to ctx["event_player"] by execute_effects.
            # ctx["player"] is the effect owner.
            event_player = context.get('event_player')
            owner = context.get('player')
            if event_player and owner and event_player is not owner.enemy:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Delete 1 opponent Digimon with DP <= this Digimon's DP"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            # Fix #6: Filter by DP <= this Digimon's DP
            my_dp = perm.dp
            def target_filter(p):
                if not p.is_digimon:
                    return False
                if my_dp is not None and p.dp is not None and p.dp > my_dp:
                    return False
                return True
            def on_delete(target_perm):
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
