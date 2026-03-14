from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT18_015(CardScript):
    """BT18-015 Kimeramon | Lv.5 Red/Purple Composite

    Alt digi: Lv.4 [Composite] trait, cost 3.
    [When Digivolving] [When Attacking] By deleting 1 of your Digimon, delete
    1 of your opponent's Digimon with the lowest DP.
    [On Deletion] You may DNA digivolve 1 of your [Machinedramon] in play and
    1 [Kimeramon] in the trash into [Millenniummon] in the hand.
    Inherited: <Security Attack +1>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Alt-Digi: From Lv.4 Composite trait, cost 3 ---
        effect_alt = ICardEffect()
        effect_alt.set_effect_name("BT18-015 Alt digi: Lv.4 Composite cost 3")
        effect_alt.set_effect_description("Alternate digivolution: Lv.4 with [Composite] trait, cost 3")
        effect_alt._alt_digi_cost = 3
        effect_alt._alt_digi_level = 4
        effect_alt._alt_digi_trait = "Composite"

        def condition_alt(context: Dict[str, Any]) -> bool:
            return True
        effect_alt.set_can_use_condition(condition_alt)
        effects.append(effect_alt)

        # --- Shared process for [When Digivolving] and [When Attacking] ---
        def _delete_own_then_delete_lowest(ctx: Dict[str, Any]):
            """Delete own Digimon, then delete opponent's lowest DP Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            def own_filter(p):
                return p.is_digimon

            def on_own_delete(target_perm):
                player.delete_permanent(target_perm)
                # Then delete 1 opponent's Digimon with lowest DP
                opp_digimon = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if not opp_digimon:
                    return
                lowest_dp = min(p.dp for p in opp_digimon)
                lowest = [p for p in opp_digimon if p.dp == lowest_dp]
                if len(lowest) == 1:
                    enemy.delete_permanent(lowest[0])
                elif len(lowest) > 1:
                    # Let agent select which lowest DP Digimon to delete
                    def lowest_filter(p):
                        return p in lowest
                    def on_lowest_selected(sel_perm):
                        enemy.delete_permanent(sel_perm)
                    game.effect_select_opponent_permanent(
                        player, on_lowest_selected, filter_fn=lowest_filter,
                        is_optional=False,
                        prompt="Delete 1 opponent Digimon with the lowest DP.")

            game.effect_select_own_permanent(
                player, on_own_delete, filter_fn=own_filter, is_optional=True,
                prompt="Delete 1 of your Digimon.")

        # --- Effect 0: [When Digivolving] ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT18-015 When Digivolving: Delete own, delete opponent's lowest DP")
        effect0.set_effect_description(
            "[When Digivolving] By deleting 1 of your Digimon, delete 1 of "
            "your opponent's Digimon with the lowest DP."
        )
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_delete_own_then_delete_lowest)
        effects.append(effect0)

        # --- Effect 0b: [When Attacking] ---
        effect0b = ICardEffect()
        effect0b.set_timing(EffectTiming.OnTappedAnyone)
        effect0b.set_effect_name("BT18-015 When Attacking: Delete own, delete opponent's lowest DP")
        effect0b.set_effect_description(
            "[When Attacking] By deleting 1 of your Digimon, delete 1 of "
            "your opponent's Digimon with the lowest DP."
        )
        effect0b.is_on_attack = True

        def condition0b(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0b.set_can_use_condition(condition0b)
        effect0b.set_on_process_callback(_delete_own_then_delete_lowest)
        effects.append(effect0b)

        # --- Effect 1: [On Deletion] DNA digivolve Machinedramon + Kimeramon
        #     (trash) into Millenniummon (hand) ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("BT18-015 On Deletion: DNA digivolve into Millenniummon")
        effect1.set_effect_description(
            "[On Deletion] You may DNA digivolve 1 of your [Machinedramon] "
            "in play and 1 [Kimeramon] in the trash into [Millenniummon] in the hand."
        )
        effect1.is_on_deletion = True
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner):
                return False
            player = card.owner
            has_machinedramon = any(
                p.is_digimon and p.contains_card_name('Machinedramon')
                for p in player.battle_area
            )
            has_millenniummon_in_hand = any(
                c.contains_card_name('Millenniummon') and getattr(c, 'is_digimon', False)
                for c in player.hand_cards
            )
            has_kimeramon_in_trash = any(
                c.contains_card_name('Kimeramon')
                for c in player.trash_cards
            )
            return has_machinedramon and has_millenniummon_in_hand and has_kimeramon_in_trash
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """DNA digivolve: Machinedramon (field) + Kimeramon (trash) -> Millenniummon (hand)"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Use effect_dna_digivolve_from_hand which handles the multi-step
            # selection: pick Millenniummon from hand, then pick field targets.
            # The engine's DNA digivolve already validates jogress conditions.
            def millenniummon_filter(c):
                return c.contains_card_name('Millenniummon') and getattr(c, 'is_digimon', False)

            game.effect_dna_digivolve_from_hand(
                player, millenniummon_filter, is_optional=True,
                prompt="Select [Millenniummon] from hand to DNA digivolve.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Inherited <Security Attack +1> ---
        effect2 = ICardEffect()
        effect2.set_effect_name("BT18-015 Security Attack +1 (Inherited)")
        effect2.set_effect_description("Inherited: <Security A. +1>")
        effect2.is_inherited_effect = True
        effect2._security_attack_modifier = 1

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
