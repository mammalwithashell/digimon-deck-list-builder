from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_059(CardScript):
    """BT23-059 Justimon: Blitz Arm | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-059 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Justimon: Accel Arm] for cost 1
        effect0._alt_digi_cost = 1
        effect0._alt_digi_name = "Justimon: Accel Arm"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Justimon: Accel Arm') or permanent.contains_card_name('Justimon: Critical Arm'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-059 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 for cost 3
        effect1._alt_digi_cost = 3
        effect1._alt_digi_level = 5

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-059 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Delete
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT23-059 By trashing 1 option card, delete 1 Digimon")
        effect3.set_effect_description("Delete")
        effect3.set_hash_string("BT23_059_OP")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Trash 1 Option from battle area, then delete lowest play cost opp Digimon"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: trash 1 Option card in battle area
            def option_filter(p):
                return p.is_option
            def on_option_trashed(opt_perm):
                if opt_perm in player.battle_area:
                    player.battle_area.remove(opt_perm)
                    for cs in opt_perm.card_sources:
                        player.trash_cards.append(cs)
                # Then delete 1 opponent's Digimon with lowest play cost
                enemy = player.enemy if player else None
                if not enemy:
                    return
                costs = [getattr(p.top_card, 'play_cost', 99) or 99
                         for p in enemy.battle_area if p.is_digimon]
                min_cost = min(costs) if costs else 99
                def target_filter(p):
                    return p.is_digimon and (getattr(p.top_card, 'play_cost', 99) or 99) <= min_cost
                def on_delete(target_perm):
                    if enemy:
                        enemy.delete_permanent(target_perm)
                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter, is_optional=False)
            game.effect_select_own_permanent(
                player, on_option_trashed, filter_fn=option_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Delete
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT23-059 By trashing 1 option card, delete 1 Digimon")
        effect4.set_effect_description("Delete")
        effect4.set_hash_string("BT23_059_WD")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Trash 1 Option, delete lowest play cost opp Digimon (WhenDigivolving)"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def option_filter(p):
                return p.is_option
            def on_option_trashed(opt_perm):
                if opt_perm in player.battle_area:
                    player.battle_area.remove(opt_perm)
                    for cs in opt_perm.card_sources:
                        player.trash_cards.append(cs)
                enemy = player.enemy if player else None
                if not enemy:
                    return
                costs = [getattr(p.top_card, 'play_cost', 99) or 99
                         for p in enemy.battle_area if p.is_digimon]
                min_cost = min(costs) if costs else 99
                def target_filter(p):
                    return p.is_digimon and (getattr(p.top_card, 'play_cost', 99) or 99) <= min_cost
                def on_delete(target_perm):
                    if enemy:
                        enemy.delete_permanent(target_perm)
                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter, is_optional=False)
            game.effect_select_own_permanent(
                player, on_option_trashed, filter_fn=option_filter, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnUseAttack
        # Delete
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnUseAttack)
        effect5.set_effect_name("BT23-059 By trashing 1 option card, delete 1 Digimon")
        effect5.set_effect_description("Delete")
        effect5.set_hash_string("BT23_059_WA")
        effect5.is_on_attack = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Trash 1 Option, delete lowest play cost opp Digimon (WhenAttacking)"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def option_filter(p):
                return p.is_option
            def on_option_trashed(opt_perm):
                if opt_perm in player.battle_area:
                    player.battle_area.remove(opt_perm)
                    for cs in opt_perm.card_sources:
                        player.trash_cards.append(cs)
                enemy = player.enemy if player else None
                if not enemy:
                    return
                costs = [getattr(p.top_card, 'play_cost', 99) or 99
                         for p in enemy.battle_area if p.is_digimon]
                min_cost = min(costs) if costs else 99
                def target_filter(p):
                    return p.is_digimon and (getattr(p.top_card, 'play_cost', 99) or 99) <= min_cost
                def on_delete(target_perm):
                    if enemy:
                        enemy.delete_permanent(target_perm)
                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter, is_optional=False)
            game.effect_select_own_permanent(
                player, on_option_trashed, filter_fn=option_filter, is_optional=False)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [All Turns] [Once Per Turn] When Option cards in the battle area are trashed, this Digimon unsuspends. Then, your opponent's Digimon's effects don't affect this Digimon for the turn.
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.OnDestroyedAnyone)
        effect6.set_effect_name("BT23-059 Unsuspend, unaffected by opponents Digimon effects")
        effect6.set_effect_description("[All Turns] [Once Per Turn] When Option cards in the battle area are trashed, this Digimon unsuspends. Then, your opponent's Digimon's effects don't affect this Digimon for the turn.")
        effect6.set_max_count_per_turn(1)
        effect6.set_hash_string("BT23_059_AT")
        effect6.is_on_deletion = True

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Unsuspend self, gain effect immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # Unsuspend this Digimon
            self_perm = card.permanent_of_this_card() if card else None
            if self_perm:
                self_perm.unsuspend()
            # Grant effect immunity from opponent's Digimon effects
            if self_perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    self_perm, ModifierType.CANNOT_BE_SELECTED_BY_EFFECT,
                    expiry='end_of_turn')

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
