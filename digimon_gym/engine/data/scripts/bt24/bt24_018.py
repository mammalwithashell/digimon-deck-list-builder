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
        # [When Digivolving] You may trash any 1 of your opponent's security cards.
        # Then, this Digimon may unsuspend.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT24-018 May trash 1 opponent's security. Then, this may unsuspend.")
        effect4.set_effect_description("Destroy Security, Unsuspend")
        effect4.is_when_digivolving = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: You may trash any 1 opponent's security. Then, this Digimon may unsuspend."""
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

            # First optional step: you may trash any 1 opponent's security card
            # C# uses SelectCardEffect with SecurityCards as root — player SELECTS
            # which security card to trash (face-down selection), not just "pop top"
            if enemy and enemy.security_cards:
                def on_trash_choice(choice):
                    if choice == 0:
                        # Player chose to trash — let them select which card
                        def on_security_selected(sec_card):
                            if sec_card and enemy:
                                enemy.trash_security_card(sec_card)
                            _maybe_unsuspend()

                        game.effect_select_opponent_security(
                            player,
                            filter_fn=None,
                            callback=on_security_selected,
                            is_optional=False,
                            prompt="Select 1 of your opponent's security cards to trash.")
                    else:
                        # Player declined the trash
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
        # [All Turns] [Once Per Turn] When your opponent's security stack is
        # removed from, you may delete 1 of their Digimon.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnLoseSecurity)
        effect5.set_effect_name("BT24-018 Delete 1 of your opponent's Digimon?")
        effect5.set_effect_description("Delete")
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BT24_18_AT_Sec_Removed")

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # C# checks: player == card.Owner.Enemy
            # In execute_effects, 'player' = owner of the permanent,
            # 'event_player' = the player from extra_context (who lost security).
            # This must only fire when the OPPONENT's security is removed.
            event_player = context.get('event_player')
            owner = card.owner if card else None
            if not owner or not event_player:
                return False
            if event_player is owner:
                # Own security was removed — do NOT trigger
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Delete 1 of your opponent's Digimon."""
            player = ctx.get('player')
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

        # Timing: EffectTiming.WhenPermanentWouldBeDeleted
        # [All Turns] [Once Per Turn] When any of your [Reptile] or [Dragonkin]
        # trait Digimon would leave the battle area, by deleting 1 of your
        # opponent's lowest DP Digimon, they don't leave.
        #
        # Key differences from original:
        # - Uses WhenPermanentWouldBeDeleted (fires BEFORE deletion, can prevent)
        # - No is_opponent_effect check (card text has no cause restriction)
        # - Includes self (Styracomon has Dragonkin trait)
        # - Target must be lowest DP only (C# uses IsMinDP)
        # - Sets _will_not_be_removed flag to prevent deletion
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.WhenPermanentWouldBeDeleted)
        effect6.set_effect_name("BT24-018 Delete an opponent's lowest DP Digimon to prevent [Reptile] or [Dragonkin] trait Digimon from leaving the battle area")
        effect6.set_effect_description("[All Turns] [Once Per Turn] When any of your [Reptile] or [Dragonkin] trait Digimon would leave the battle area, by deleting 1 of your opponent's lowest DP Digimon, they don't leave.")
        effect6.is_optional = True
        effect6.set_max_count_per_turn(1)
        effect6.set_hash_string("BT24_018_AT_Prevent_Deletion")

        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # The leaving permanent must have Reptile or Dragonkin trait
            leaving_perm = context.get('permanent')
            if leaving_perm is None:
                return False
            # Check traits via permanent's has_trait method
            if not (leaving_perm.has_trait('Reptile') or leaving_perm.has_trait('Dragonkin')):
                return False
            # Must belong to this card's owner
            owner = card.owner if card else None
            if not owner:
                return False
            if leaving_perm not in owner.battle_area:
                return False
            # Must have opponent Digimon to delete (lowest DP check is in process)
            enemy = owner.enemy if owner else None
            if not enemy:
                return False
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
            if not opp_digimon:
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: By deleting 1 opponent's lowest DP Digimon, prevent leaving."""
            player = ctx.get('player')
            game = ctx.get('game')
            leaving_perm = ctx.get('permanent')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # C# uses IsMinDP: only the lowest DP Digimon can be selected
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
            if not opp_digimon:
                return
            min_dp = min(p.dp for p in opp_digimon)

            def lowest_dp_filter(p):
                if not p.is_digimon:
                    return False
                if p.dp is None:
                    return False
                return p.dp == min_dp

            def on_delete(target_perm):
                if enemy:
                    enemy.delete_permanent(target_perm)
                # Cost paid — prevent the leaving permanent from being removed
                if leaving_perm:
                    leaving_perm._will_not_be_removed = True

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=lowest_dp_filter, is_optional=False,
                prompt="Select 1 of your opponent's lowest DP Digimon to delete (prevents leaving).")

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
