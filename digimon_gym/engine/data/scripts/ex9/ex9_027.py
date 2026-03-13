from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_027(CardScript):
    """EX9-027 Kokeshimon | Lv.4 Yellow | Puppet

    Alt digivolve: from Lv.3 [Puppet] for cost 2.

    [When Digivolving] By trashing 1 card in your hand, 1 of your opponent's
        Digimon gets -4000 DP for the turn.

    [On Deletion] By trashing 1 card in your hand, 1 of your opponent's
        Digimon gets -4000 DP for the turn.

    --- Inherited ---
    [Opponent's Turn][Once Per Turn] When an opponent's Digimon attacks, by
        deleting 1 of your other Digimon, end the attack.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from Lv.3 [Puppet] for cost 2 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-027 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3
        effect0._alt_digi_trait = "Puppet"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Shared helper: trash 1 from hand, give opponent Digimon -4000 DP ---
        def _trash_and_minus_dp(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            if not player.hand_cards:
                return

            def on_trash(trashed_card):
                player.hand_cards.remove(trashed_card)
                player.trash_cards.append(trashed_card)

                # Give 1 opponent's Digimon -4000 DP for the turn
                enemy = player.enemy if player else None
                if not enemy:
                    return
                opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
                if not opp_digimon:
                    return

                def opp_digi_filter(p):
                    return p.is_digimon

                def on_select(target_perm):
                    target_perm.change_dp(-4000)

                game.effect_select_opponent_permanent(
                    player, on_select, filter_fn=opp_digi_filter,
                    is_optional=False,
                    prompt="Select 1 of your opponent's Digimon to give -4000 DP."
                )

            game.effect_select_hand_card(
                player, lambda c: True, on_trash, is_optional=True,
                prompt="Select 1 card from your hand to trash."
            )

        # --- Effect 1: [When Digivolving] Trash 1, give -4000 DP ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX9-027 When Digivolving: -4000 DP")
        effect1.set_effect_description(
            "[When Digivolving] By trashing 1 card in your hand, 1 of your "
            "opponent's Digimon gets -4000 DP for the turn."
        )
        effect1.is_when_digivolving = True
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            return len(player.hand_cards) > 0
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_trash_and_minus_dp)
        effects.append(effect1)

        # --- Effect 2: [On Deletion] Trash 1, give -4000 DP ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDestroyedAnyone)
        effect2.set_effect_name("EX9-027 On Deletion: -4000 DP")
        effect2.set_effect_description(
            "[On Deletion] By trashing 1 card in your hand, 1 of your "
            "opponent's Digimon gets -4000 DP for the turn."
        )
        effect2.is_on_deletion = True
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            player = card.owner if card else None
            if not player:
                return False
            return len(player.hand_cards) > 0
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_trash_and_minus_dp)
        effects.append(effect2)

        # --- Effect 3 (Inherited): [Opponent's Turn][Once Per Turn] End attack ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUseAttack)
        effect3.set_effect_name("EX9-027 End attack by deleting own Digimon")
        effect3.set_effect_description(
            "[Opponent's Turn][Once Per Turn] When an opponent's Digimon attacks, "
            "by deleting 1 of your other Digimon, end the attack."
        )
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("StopAttack_EX9-027")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and card.owner.is_my_turn:
                return False
            player = card.owner if card else None
            if not player:
                return False
            my_perm = card.permanent_of_this_card()
            has_other = any(
                p.is_digimon and p is not my_perm
                for p in player.battle_area
            )
            return has_other
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            my_perm = card.permanent_of_this_card() if card else None

            def own_digimon_filter(p):
                return p.is_digimon and p is not my_perm

            def on_delete(target_perm):
                player.delete_permanent(target_perm)
                game.force_end_attack()

            game.effect_select_own_permanent(
                player, on_delete, filter_fn=own_digimon_filter,
                is_optional=False,
                prompt="Select 1 of your other Digimon to delete to end the attack."
            )

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
