from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_203(CardScript):
    """P-203 Justimon: Accel Arm | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("P-203 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("P-203 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Justimon: Blitz Arm] for cost 1
        effect1._alt_digi_cost = 1
        effect1._alt_digi_name = "Justimon: Blitz Arm"

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Justimon: Blitz Arm') or permanent.contains_card_name('Justimon: Critical Arm'))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] [Once Per Turn] <De-Digivolve 1> 1 of your opponent's Digimon. Then, by trashing 1 Option card in the battle area, this Digimon gains <Piercing> and <Security A. +1> for the turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("P-203 De-Digivolve 1, then by trashing 1 option in battle area, gain Piercing & Sec +1 for the turn")
        effect2.set_effect_description("[On Play] [Once Per Turn] <De-Digivolve 1> 1 of your opponent's Digimon. Then, by trashing 1 Option card in the battle area, this Digimon gains <Piercing> and <Security A. +1> for the turn.")
        effect2.set_hash_string("P_203_OP/WD/WA")
        effect2.is_on_play = True
        effect2._is_piercing = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: De Digivolve, Gain Keyword Piercing, Change Security Attack"""
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
            if perm:
                perm.grant_keyword('_is_piercing')
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] [Once Per Turn] <De-Digivolve 1> 1 of your opponent's Digimon. Then, by trashing 1 Option card in the battle area, this Digimon gains <Piercing> and <Security A. +1> for the turn.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("P-203 De-Digivolve 1, then by trashing 1 option in battle area, gain Piercing & Sec +1 for the turn")
        effect3.set_effect_description("[When Digivolving] [Once Per Turn] <De-Digivolve 1> 1 of your opponent's Digimon. Then, by trashing 1 Option card in the battle area, this Digimon gains <Piercing> and <Security A. +1> for the turn.")
        effect3.set_hash_string("P_203_OP/WD/WA")
        effect3.is_when_digivolving = True
        effect3._is_piercing = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: De Digivolve, Gain Keyword Piercing, Change Security Attack"""
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
            if perm:
                perm.grant_keyword('_is_piercing')
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] [Once Per Turn] <De-Digivolve 1> 1 of your opponent's Digimon. Then, by trashing 1 Option card in the battle area, this Digimon gains <Piercing> and <Security A. +1> for the turn.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUseAttack)
        effect4.set_effect_name("P-203 De-Digivolve 1, then by trashing 1 option in battle area, gain Piercing & Sec +1 for the turn")
        effect4.set_effect_description("[When Attacking] [Once Per Turn] <De-Digivolve 1> 1 of your opponent's Digimon. Then, by trashing 1 Option card in the battle area, this Digimon gains <Piercing> and <Security A. +1> for the turn.")
        effect4.set_hash_string("P_203_OP/WD/WA")
        effect4.is_on_attack = True
        effect4._is_piercing = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: De Digivolve, Gain Keyword Piercing, Change Security Attack"""
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
            if perm:
                perm.grant_keyword('_is_piercing')
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [All Turns] [Once Per Turn] When Option cards in the battle area are trashed, 1 of your opponent's Digimon can't digivolve or attack players until their turn ends.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnDestroyedAnyone)
        effect5.set_effect_name("P-203 1 digimon can't attack or digivolve")
        effect5.set_effect_description("[All Turns] [Once Per Turn] When Option cards in the battle area are trashed, 1 of your opponent's Digimon can't digivolve or attack players until their turn ends.")
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("P_203_AT")
        effect5.is_on_deletion = True
        effect5._is_cannot_attack = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Gain Keyword Cannot Attack, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_cannot_attack')
            # Grant effect immunity via modifier system
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
