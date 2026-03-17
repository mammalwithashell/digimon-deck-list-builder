from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_101(CardScript):
    """BT24-101 Jupitermon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-101 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 with [TS] trait for cost 5
        effect0._alt_digi_cost = 5
        effect0._alt_digi_level = 5
        effect0._alt_digi_trait = "TS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Lv.5 named [Aegiochusmon], cost = your security count
        # C# uses CostEquation based on card.Owner.SecurityCards.Count
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-101 Alternate digivolution requirement (Aegiochusmon)")
        effect1.set_effect_description(
            "Alternate digivolution: Lv.5 named [Aegiochusmon], cost = security count")
        effect1._alt_digi_cost = 5  # fallback static cost
        effect1._alt_digi_level = 5
        effect1._alt_digi_name = "Aegiochusmon"
        # Dynamic cost: 1 for each of your security cards
        effect1._alt_digi_cost_fn = lambda: len(card.owner.security_cards) if card and card.owner else 5

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card):
                return False
            return permanent.top_card.level == 5 and any(
                'Aegiochusmon' in n
                for n in getattr(permanent.top_card, 'card_names', []))
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # ----------------------------------------------------------------
        # [On Play] Trash your top security card and 1 of your opponent's
        # Digimon gets -13000 DP until their turn ends.  Then, if you have
        # 1 or fewer security cards, <Recovery +2 (Deck)>.
        # ----------------------------------------------------------------
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT24-101 Trash own security, -13000 DP, conditional Recovery +2")
        effect2.set_effect_description("[On Play] Trash your top security card and 1 of your opponent's Digimon gets -13000 DP. Then, if you have 1 or fewer security cards, Recovery +2.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2_fixed(ctx: Dict[str, Any]):
            """[On Play] Trash own security, -13000 DP opponent (until their turn ends), conditional Recovery +2."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Step 1: Trash YOUR top security card (cost of the effect)
            if player.security_cards:
                top_sec = player.security_cards[0]
                player.trash_security_card(top_sec)
            # Step 2: Player selects 1 of opponent's Digimon to get -13000 DP
            # Card text: "until their turn ends" = end_of_opponent_turn
            def on_dp_target(target_perm):
                from ....interfaces.modifiers import ModifierType
                game.register_modifier(
                    target_perm, ModifierType.CHANGE_DP,
                    value_fn=lambda cur, t, c: cur - 13000,
                    expiry='end_of_opponent_turn')
                # Step 3: If you have 1 or fewer security cards, Recovery +2
                if len(player.security_cards) <= 1:
                    player.recovery(2)

            game.effect_select_opponent_permanent(
                player, on_dp_target,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Select 1 opponent's Digimon to get -13000 DP.")

        effect2.set_on_process_callback(process2_fixed)
        effects.append(effect2)

        # ----------------------------------------------------------------
        # [When Digivolving] Same effect as On Play
        # ----------------------------------------------------------------
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT24-101 Trash own security, -13000 DP, conditional Recovery +2")
        effect3.set_effect_description("[When Digivolving] Trash your top security card and 1 of your opponent's Digimon gets -13000 DP. Then, if you have 1 or fewer security cards, Recovery +2.")
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """[When Digivolving] Trash own security, -13000 DP opponent (until their turn ends), conditional Recovery +2."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Step 1: Trash YOUR top security card (cost of the effect)
            if player.security_cards:
                top_sec = player.security_cards[0]
                player.trash_security_card(top_sec)
            # Step 2: Player selects 1 of opponent's Digimon to get -13000 DP
            # Card text: "until their turn ends" = end_of_opponent_turn
            def on_dp_target(target_perm):
                from ....interfaces.modifiers import ModifierType
                game.register_modifier(
                    target_perm, ModifierType.CHANGE_DP,
                    value_fn=lambda cur, t, c: cur - 13000,
                    expiry='end_of_opponent_turn')
                # Step 3: If you have 1 or fewer security cards, Recovery +2
                if len(player.security_cards) <= 1:
                    player.recovery(2)

            game.effect_select_opponent_permanent(
                player, on_dp_target,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Select 1 opponent's Digimon to get -13000 DP.")

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # ----------------------------------------------------------------
        # [All Turns] [Once Per Turn] When your security stack is removed
        # from, trash your opponent's top security card.
        # ----------------------------------------------------------------
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnLoseSecurity)
        effect4.set_effect_name("BT24-101 Trash Opponent's top security")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When your security stack is removed from, trash your opponent's top security card.")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT24_101_AT_Trash_sec")

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only fire on OnLoseSecurity timing (context contains 'lost_card')
            if 'lost_card' not in context:
                return False
            # Must be the card OWNER's security that was lost
            owner = card.owner if card else None
            if owner is None or owner is not context.get('player'):
                return False
            # Re-check OPT (engine doesn't re-check at resolution time)
            if not effect4.can_activate_this_turn():
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Trash opponent's top security card."""
            player = ctx.get('player')
            if not player:
                return
            enemy = player.enemy if player else None
            if enemy and enemy.security_cards:
                top_sec = enemy.security_cards[0]
                enemy.trash_security_card(top_sec)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # ----------------------------------------------------------------
        # [All Turns] [Once Per Turn] When any of your [TS] trait Digimon
        # or Tamers would leave the battle area, by trashing your top
        # security card, they don't leave.
        # ----------------------------------------------------------------
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.WhenRemoveField)
        effect5.set_effect_name("BT24-101 By trashing top security, [TS] card doesn't leave")
        effect5.set_effect_description("[All Turns] [Once Per Turn] When any of your [TS] trait Digimon or Tamers would leave the battle area, by trashing your top security card, they don't leave.")
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BT24_101_AT_Protect_TS")

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # The permanent being removed must have [TS] trait
            target_perm = context.get('permanent')
            if target_perm is None:
                return False
            if not (target_perm.is_digimon or target_perm.is_tamer):
                return False
            if not target_perm.has_trait('TS'):
                return False
            # The target must belong to the card's owner
            owner = card.owner if card else None
            if owner is None:
                return False
            if target_perm not in owner.battle_area:
                return False
            # Owner must have security to trash
            if not owner.security_cards:
                return False
            # Re-check OPT
            if not effect5.can_activate_this_turn():
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Trash own top security to prevent [TS] permanent from leaving."""
            owner = card.owner if card else None
            game = ctx.get('game')
            if not owner or not game:
                return
            if not owner.security_cards:
                return
            # Trash own top security card as cost
            top_sec = owner.security_cards[0]
            owner.trash_security_card(top_sec)
            # Register destruction prevention modifier
            target_perm = ctx.get('permanent')
            if target_perm and hasattr(game, 'modifiers'):
                from ....interfaces.modifiers import ModifierEntry, ModifierType
                game.modifiers.register(ModifierEntry(
                    modifier_type=ModifierType.CANNOT_BE_DESTROYED,
                    condition=lambda perm, c: perm is target_perm,
                    source_effect=effect5,
                    source_permanent=card.permanent_of_this_card(),
                    expiry='end_of_turn',
                ))

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
