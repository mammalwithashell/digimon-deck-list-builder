from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_018(CardScript):
    """BT24-018 Styracomon | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-018 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Lamiamon] for cost 6 while you have [Owen Dreadnought]
        effect0._alt_digi_cost = 6
        effect0._alt_digi_name = "Lamiamon"

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner):
                return False
            # Check that Owen Dreadnought is on the player's field
            has_owen = any(
                p.contains_card_name('Owen Dreadnought')
                for p in card.owner.battle_area
            )
            if not has_owen:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: progress
        # Progress
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-018 Progress")
        effect1.set_effect_description("Progress")
        effect1._is_progress = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-018 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: piercing
        # Piercing
        effect_piercing = ICardEffect()
        effect_piercing.set_effect_name("BT24-018 Piercing")
        effect_piercing.set_effect_description("Piercing")
        effect_piercing._is_piercing = True

        def condition_piercing(context: Dict[str, Any]) -> bool:
            return True
        effect_piercing.set_can_use_condition(condition_piercing)
        effects.append(effect_piercing)

        # Factory effect: armor_purge
        # Armor Purge
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-018 Armor Purge")
        effect3.set_effect_description("Armor Purge")
        effect3._is_armor_purge = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] May trash 1 opponent's top security. Then, this may unsuspend.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT24-018 May trash 1 opponent's security. Then, this may unsuspend.")
        effect4.set_effect_description("Destroy Security, Unsuspend")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: You may trash 1 opponent's security. Then, this Digimon may unsuspend."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return
            enemy = player.enemy if player else None

            def _maybe_unsuspend():
                """Second optional step: this Digimon may unsuspend."""
                if perm and perm.is_suspended:
                    def on_unsuspend_choice(choice):
                        if choice == 0 and perm:
                            perm.unsuspend()
                    game.effect_choose_branch(
                        player, 2, on_unsuspend_choice,
                        prompt="This Digimon may unsuspend.",
                        branch_labels=["Yes, unsuspend", "No, decline"])
                # If not suspended, nothing to do

            # First optional step: you may trash 1 opponent's security card
            if enemy and enemy.security_cards:
                def on_trash_choice(choice):
                    if choice == 0 and enemy and enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)
                    _maybe_unsuspend()
                game.effect_choose_branch(
                    player, 2, on_trash_choice,
                    prompt="You may trash 1 of your opponent's security cards.",
                    branch_labels=["Yes, trash security", "No, decline"])
            else:
                _maybe_unsuspend()

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnLoseSecurity
        # Delete
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnLoseSecurity)
        effect5.set_effect_name("BT24-018 Delete 1 of your opponent's Digimon?")
        effect5.set_effect_description("Delete")
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BT24_18_AT_Sec_Removed")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=True)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When any of your other Digimon with [Reptile] or [Dragonkin] trait
        # would leave the battle area by opponent's effects, by deleting 1 of opponent's Digimon, they don't leave.
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.WhenRemoveField)
        effect6.set_effect_name("BT24-018 Delete an opponent's Digimon to prevent [Reptile] or [Dragonkin] trait Digimon from leaving the battle area")
        effect6.set_effect_description("[All Turns] [Once Per Turn] By deleting 1 of your opponent's Digimon, your other [Reptile]/[Dragonkin] Digimon don't leave the battle area by opponent's effects.")
        effect6.is_optional = True
        effect6.set_max_count_per_turn(1)
        effect6.set_hash_string("BT24_018_AT_Prevent_Deletion")

        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be triggered by opponent's effect
            if not context.get('is_opponent_effect', False):
                return False
            # The leaving permanent must have Reptile or Dragonkin trait
            leaving_perm = context.get('permanent')
            if leaving_perm is None:
                return False
            traits = getattr(leaving_perm.top_card, 'card_traits', []) or [] if leaving_perm.top_card else []
            if not any('Reptile' in t or 'Dragonkin' in t for t in traits):
                return False
            # The leaving permanent must NOT be this Styracomon (must be an "other" Digimon)
            styracomon_perm = card.permanent_of_this_card()
            if leaving_perm is styracomon_perm:
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: By deleting 1 opponent's Digimon, prevent leaving_perm from leaving."""
            player = ctx.get('player')
            game = ctx.get('game')
            leaving_perm = ctx.get('permanent')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            # "By deleting 1 of your opponent's Digimon" — player selects which to delete
            def on_delete(target_perm):
                if enemy:
                    enemy.delete_permanent(target_perm)
                # Cost paid — prevent the leaving permanent from leaving
                if leaving_perm:
                    from digimon_gym.engine.interfaces.modifiers import ModifierType
                    game.register_modifier(
                        leaving_perm, ModifierType.CANNOT_BE_REMOVED,
                        value_fn=lambda: True, expiry='end_of_turn'
                    )
            def target_filter(p):
                return p.is_digimon
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False,
                prompt="Select 1 of your opponent's Digimon to delete (cost to prevent leaving).")

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
