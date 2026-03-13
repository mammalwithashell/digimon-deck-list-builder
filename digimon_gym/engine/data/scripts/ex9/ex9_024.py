from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_024(CardScript):
    """EX9-024 Hanimon | Lv.3 Yellow | Puppet

    Alt digivolve: from Kyaromon for cost 0.

    [On Play] By trashing 1 card in your hand, you may return 1 Digimon card
        with the [Puppet] trait from your trash to the hand.

    --- Inherited ---
    [Opponent's Turn][Once Per Turn] When an opponent's Digimon attacks, by
        deleting 1 of your other Digimon, end the attack.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from Kyaromon for cost 0 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-024 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Kyaromon"
        effect0._alt_digi_level = 2

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [On Play] Trash 1 hand card, return 1 [Puppet] Digimon from trash to hand ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX9-024 Trash hand card, return Puppet from trash")
        effect1.set_effect_description(
            "[On Play] By trashing 1 card in your hand, you may return 1 Digimon "
            "card with the [Puppet] trait from your trash to the hand."
        )
        effect1.is_on_play = True
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            if len(player.hand_cards) < 1:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            if not player.hand_cards:
                return

            def _puppet_digimon_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Puppet' in t for t in traits)

            # Trash 1 card from hand
            def on_trash(trashed_card):
                player.hand_cards.remove(trashed_card)
                player.trash_cards.append(trashed_card)

                # Then return 1 [Puppet] Digimon from trash to hand
                qualifying = [c for c in player.trash_cards if _puppet_digimon_filter(c)]
                if not qualifying:
                    return

                # Use simplified selection: pick first qualifying
                chosen = qualifying[0]
                player.trash_cards.remove(chosen)
                player.hand_cards.append(chosen)

            game.effect_select_hand_card(
                player, lambda c: True, on_trash, is_optional=True,
                prompt="Select 1 card from your hand to trash."
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2 (Inherited): [Opponent's Turn][Once Per Turn] End attack ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("EX9-024 End attack by deleting own Digimon")
        effect2.set_effect_description(
            "[Opponent's Turn][Once Per Turn] When an opponent's Digimon attacks, "
            "by deleting 1 of your other Digimon, end the attack."
        )
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("StopAttack_EX9-024")

        def condition2(context: Dict[str, Any]) -> bool:
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
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
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

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
