from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_047(CardScript):
    """BT23-047 Examon | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-047 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.6 for cost 5
        effect0._alt_digi_cost = 5
        effect0._alt_digi_level = 6

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Jogress Condition
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-047 Jogress Condition")
        effect1.set_effect_description("Jogress Condition")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: security_attack_plus
        # Security Attack +1
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-047 Security Attack +1")
        effect2.set_effect_description("Security Attack +1")
        effect2._security_attack_modifier = 1

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        effect2b = ICardEffect()
        effect2b.set_effect_name("BT23-047 Piercing")
        effect2b.set_effect_description("Piercing")
        effect2b._is_piercing = True

        def condition2b(context: Dict[str, Any]) -> bool:
            return True

        effect2b.set_can_use_condition(condition2b)
        effects.append(effect2b)

        # Factory effect: partition
        # Partition
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-047 Partition")
        effect3.set_effect_description("Partition")
        effect3._is_partition = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Suspend, Gain Keyword Cannot Unsuspend Player, Force Attack, Grant Cannot Unsuspend
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT23-047 Suspend 5 digimon/tamer, none can unsuspend. Then you may attack")
        effect4.set_effect_description("Suspend, Gain Keyword Cannot Unsuspend Player, Force Attack, Grant Cannot Unsuspend")
        effect4.is_on_play = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Suspend 5 opponent Digimon/Tamers, none can unsuspend, then may attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if enemy:
                # Auto-suspend up to 5 opponent permanents (Digimon or Tamers)
                targets = [p for p in enemy.battle_area
                           if (p.is_digimon or p.is_tamer) and not p.is_suspended][:5]
                for t in targets:
                    t.suspend()
                # None of opponent's Digimon can unsuspend in their next unsuspend phase
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                for p in enemy.battle_area:
                    if p.is_digimon:
                        game.register_modifier(
                            p, ModifierType.CANNOT_UNSUSPEND,
                            expiry='end_of_opponent_turn')
            # Then, this Digimon may attack
            pass  # descriptive-tagged: force_attack — engine does not yet support optional self-attack

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Suspend, Gain Keyword Cannot Unsuspend Player, Force Attack, Grant Cannot Unsuspend
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect5.set_effect_name("BT23-047 Suspend 5 digimon/tamer, none can unsuspend. Then you may attack")
        effect5.set_effect_description("Suspend, Gain Keyword Cannot Unsuspend Player, Force Attack, Grant Cannot Unsuspend")
        effect5.is_when_digivolving = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Suspend 5 opponent Digimon/Tamers, none can unsuspend, then may attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if enemy:
                # Auto-suspend up to 5 opponent permanents (Digimon or Tamers)
                targets = [p for p in enemy.battle_area
                           if (p.is_digimon or p.is_tamer) and not p.is_suspended][:5]
                for t in targets:
                    t.suspend()
                # None of opponent's Digimon can unsuspend in their next unsuspend phase
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                for p in enemy.battle_area:
                    if p.is_digimon:
                        game.register_modifier(
                            p, ModifierType.CANNOT_UNSUSPEND,
                            expiry='end_of_opponent_turn')
            # Then, this Digimon may attack
            pass  # descriptive-tagged: force_attack — engine does not yet support optional self-attack

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.OnLoseSecurity
        # [Your Turn] [Once Per Turn] When your opponent's security stack is removed from, trash 1 of their Option cards in the battle area. Then, delete 1 of their suspended Digimon or Tamers.
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.OnLoseSecurity)
        effect6.set_effect_name("BT23-047 Trash 1 option card, then delete 1 suspended digimon/tamer")
        effect6.set_effect_description("[Your Turn] [Once Per Turn] When your opponent's security stack is removed from, trash 1 of their Option cards in the battle area. Then, delete 1 of their suspended Digimon or Tamers.")
        effect6.set_max_count_per_turn(1)
        effect6.set_hash_string("YT_BT23_047")

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Trash 1 opponent Option in battle area, then delete 1 suspended Digimon/Tamer"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            # Trash 1 of their Option cards in the battle area
            option_perms = [p for p in enemy.battle_area if p.is_option]
            if option_perms:
                target = option_perms[0]
                if target in enemy.battle_area:
                    enemy.battle_area.remove(target)
                    for cs in target.card_sources:
                        enemy.trash_cards.append(cs)
            # Then, delete 1 of their suspended Digimon or Tamers
            def delete_filter(p):
                return (p.is_digimon or p.is_tamer) and p.is_suspended
            def on_delete(target_perm):
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=delete_filter, is_optional=False)

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
