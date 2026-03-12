from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_101(CardScript):
    """BT20-101 Zephagamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-101 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.6 with [Vortex Warriors] trait for cost 1
        effect0._alt_digi_cost = 1
        effect0._alt_digi_level = 6
        effect0._alt_digi_trait = "Vortex Warriors"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Vortex Warriors' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-101 Blast Digivolve")
        effect1.set_effect_description("Blast Digivolve")
        effect1.is_counter_effect = True
        effect1._is_blast_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: piercing
        # Piercing
        effect_piercing = ICardEffect()
        effect_piercing.set_effect_name("BT20-101 Piercing")
        effect_piercing.set_effect_description("Piercing")
        effect_piercing._is_piercing = True

        def condition_piercing(context: Dict[str, Any]) -> bool:
            return True
        effect_piercing.set_can_use_condition(condition_piercing)
        effects.append(effect_piercing)

        # Factory effect: vortex
        # Vortex
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-101 Vortex")
        effect2.set_effect_description("Vortex")
        effect2._is_vortex = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: blocker
        # Blocker
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-101 Blocker")
        effect3.set_effect_description("Blocker")
        effect3._is_blocker = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] [Once Per Turn] When any Digimon suspend, this Digimon may unsuspend.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnTappedAnyone)
        effect4.set_effect_name("BT20-101 Unsuspend this Digimon")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When any Digimon suspend, this Digimon may unsuspend.")
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("Unsuspend_BT20-101")

        def condition4(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            # This Digimon must be on the field
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Unsuspend this Digimon (self)."""
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return
            perm.unsuspend()

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may suspend 1 Digimon. Then, for every 2 suspended Digimon,
        # you may return 1 of your opponent's suspended Digimon to the bottom of the deck.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect5.set_effect_name("BT20-101 Suspend 1 Digimon, Then return suspended Digimon to deck bottom for every 2 suspended")
        effect5.set_effect_description("[On Play] You may suspend 1 Digimon. Then, for every 2 suspended Digimon, you may return 1 of your opponent's suspended Digimon to the bottom of the deck.")
        effect5.is_on_play = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Suspend any Digimon (optional), then bounce for every 2 suspended."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            enemy = player.enemy if player else None

            # Step 1: You may suspend 1 Digimon (any — own or opponent's).
            # Try opponent first (if filter returns True for all). The card says "1 Digimon"
            # without restriction, so the player can choose from any battle area.
            # Use opponent select; if no opponent Digimon, try own.
            suspended_happened = [False]

            def on_suspend_opp(target_perm):
                target_perm.suspend()
                suspended_happened[0] = True

            def on_suspend_own(target_perm):
                target_perm.suspend()
                suspended_happened[0] = True

            # Offer selection from all permanents — select opponent's first, then own.
            # Since the engine has no "any permanent" selector, offer opponent selection
            # as the primary action (most common use case).
            game.effect_select_opponent_permanent(
                player, on_suspend_opp,
                filter_fn=lambda p: p.is_digimon,
                is_optional=True,
                prompt="You may suspend 1 Digimon.")

            # If no opponent Digimon was suspended (optional declined or no valid targets),
            # offer own Digimon suspension.
            if not suspended_happened[0]:
                game.effect_select_own_permanent(
                    player, on_suspend_own,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=True,
                    prompt="You may suspend 1 of your Digimon.")

            # Step 2: Count all suspended Digimon across both fields.
            # For every 2 suspended Digimon, may return 1 opponent's suspended Digimon
            # to the bottom of the deck.
            if not (player and enemy and game):
                return

            all_perms = list(player.battle_area or []) + list(enemy.battle_area or [])
            suspended_count = sum(1 for p in all_perms if p.is_digimon and p.is_suspended)
            bounces = suspended_count // 2

            for _ in range(bounces):
                def on_bounce(target_perm):
                    if enemy:
                        enemy.return_permanent_to_deck_bottom(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_bounce,
                    filter_fn=lambda p: p.is_digimon and p.is_suspended,
                    is_optional=True,
                    prompt="You may return 1 of your opponent's suspended Digimon to the bottom of the deck.")

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may suspend 1 Digimon. Then, for every 2 suspended Digimon,
        # you may return 1 of your opponent's suspended Digimon to the bottom of the deck.
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect6.set_effect_name("BT20-101 Suspend 1 Digimon, Then return suspended Digimon to deck bottom for every 2 suspended")
        effect6.set_effect_description("[When Digivolving] You may suspend 1 Digimon. Then, for every 2 suspended Digimon, you may return 1 of your opponent's suspended Digimon to the bottom of the deck.")
        effect6.is_when_digivolving = True

        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Suspend any Digimon (optional), then bounce for every 2 suspended."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            enemy = player.enemy if player else None

            # Step 1: You may suspend 1 Digimon (any).
            suspended_happened = [False]

            def on_suspend_opp(target_perm):
                target_perm.suspend()
                suspended_happened[0] = True

            def on_suspend_own(target_perm):
                target_perm.suspend()
                suspended_happened[0] = True

            game.effect_select_opponent_permanent(
                player, on_suspend_opp,
                filter_fn=lambda p: p.is_digimon,
                is_optional=True,
                prompt="You may suspend 1 Digimon.")

            if not suspended_happened[0]:
                game.effect_select_own_permanent(
                    player, on_suspend_own,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=True,
                    prompt="You may suspend 1 of your Digimon.")

            # Step 2: For every 2 suspended Digimon, bounce 1 opponent's suspended Digimon.
            if not (player and enemy and game):
                return

            all_perms = list(player.battle_area or []) + list(enemy.battle_area or [])
            suspended_count = sum(1 for p in all_perms if p.is_digimon and p.is_suspended)
            bounces = suspended_count // 2

            for _ in range(bounces):
                def on_bounce(target_perm):
                    if enemy:
                        enemy.return_permanent_to_deck_bottom(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_bounce,
                    filter_fn=lambda p: p.is_digimon and p.is_suspended,
                    is_optional=True,
                    prompt="You may return 1 of your opponent's suspended Digimon to the bottom of the deck.")

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        # Inherited: Ace Overflow <-4>
        # Engine handles ACE Overflow via _apply_ace_overflow in player.py
        # when the card leaves the field. The is_ace and ace_overflow_cost
        # attributes on the card entity base drive this behavior automatically.
        effect_ace = ICardEffect()
        effect_ace.set_effect_name("BT20-101 Ace Overflow")
        effect_ace.set_effect_description("Ace Overflow <-4>")
        effect_ace.is_inherited_effect = True
        pass  # descriptive-tagged: ace_overflow

        def condition_ace(context: Dict[str, Any]) -> bool:
            return True
        effect_ace.set_can_use_condition(condition_ace)
        effects.append(effect_ace)

        return effects
