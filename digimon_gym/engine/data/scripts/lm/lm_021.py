from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class LM_021(CardScript):
    """LM-021 Agumon - Bond of Bravery | Lv.7

    [Hand] [Counter] <Blast Digivolve>
    [On Play] [When Digivolving] Delete any number of your opponent's Digimon
    whose total DP adds up to equal or less than this Digimon's DP.
    [When Attacking] [Once Per Turn] If you have a Tamer, trash the top card
    of your opponent's security stack.
    Inherited: Ace Overflow <-5>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Blast Digivolve
        effect_blast = ICardEffect()
        effect_blast.set_effect_name("LM-021 Blast Digivolve")
        effect_blast.set_effect_description("[Hand] [Counter] <Blast Digivolve>")
        effect_blast.is_counter_effect = True
        effect_blast._is_blast_digivolve = True

        def condition_blast(context: Dict[str, Any]) -> bool:
            return True
        effect_blast.set_can_use_condition(condition_blast)
        effects.append(effect_blast)

        # Shared process: Delete opponent's Digimon whose total DP <= this Digimon's DP
        def _shared_delete_process(ctx: Dict[str, Any]):
            """Delete opponent's Digimon up to this Digimon's DP total."""
            player = ctx.get('player')
            game = ctx.get('game')
            perm = ctx.get('permanent')
            if not (player and game and perm):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            my_dp = perm.dp or 0
            if my_dp <= 0:
                return

            # Greedy approach: delete opponent Digimon from lowest DP up
            opp_digimon = sorted(
                [p for p in enemy.battle_area if p.is_digimon],
                key=lambda p: p.dp or 0
            )
            total_dp = 0
            to_delete = []
            for opp in opp_digimon:
                opp_dp = opp.dp or 0
                if total_dp + opp_dp <= my_dp:
                    total_dp += opp_dp
                    to_delete.append(opp)

            for target in to_delete:
                if target in enemy.battle_area:
                    enemy.delete_permanent(target)

        # [On Play]
        effect_op = ICardEffect()
        effect_op.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_op.set_effect_name("LM-021 Delete opponent Digimon up to this DP")
        effect_op.set_effect_description("[On Play] Delete any number of opponent's Digimon whose total DP <= this Digimon's DP.")
        effect_op.is_on_play = True

        def condition_op(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_op.set_can_use_condition(condition_op)
        effect_op.set_on_process_callback(_shared_delete_process)
        effects.append(effect_op)

        # [When Digivolving]
        effect_wd = ICardEffect()
        effect_wd.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_wd.set_effect_name("LM-021 Delete opponent Digimon up to this DP")
        effect_wd.set_effect_description("[When Digivolving] Delete any number of opponent's Digimon whose total DP <= this Digimon's DP.")
        effect_wd.is_when_digivolving = True

        def condition_wd(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_wd.set_can_use_condition(condition_wd)
        effect_wd.set_on_process_callback(_shared_delete_process)
        effects.append(effect_wd)

        # [When Attacking] [Once Per Turn] If you have a Tamer, trash opponent's top security
        effect_atk = ICardEffect()
        effect_atk.set_timing(EffectTiming.OnUseAttack)
        effect_atk.set_effect_name("LM-021 Trash opponent top security")
        effect_atk.set_effect_description("[When Attacking] [Once Per Turn] If you have a Tamer, trash the top card of your opponent's security stack.")
        effect_atk.is_on_attack = True
        effect_atk.set_max_count_per_turn(1)
        effect_atk.set_hash_string("TrashSec_LM_021")

        def condition_atk(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must have a Tamer
            owner = card.owner if card else None
            if not owner:
                return False
            has_tamer = any(getattr(p, 'is_tamer', False) for p in owner.battle_area)
            return has_tamer

        effect_atk.set_can_use_condition(condition_atk)

        def process_atk(ctx: Dict[str, Any]):
            """Trash opponent's top security card."""
            player = ctx.get('player')
            if not player:
                return
            enemy = player.enemy if player else None
            if enemy and enemy.security_cards:
                top_sec = enemy.security_cards.pop()  # top = last
                enemy.trash_cards.append(top_sec)

        effect_atk.set_on_process_callback(process_atk)
        effects.append(effect_atk)

        # Inherited: Ace Overflow <-5>
        effect_ace = ICardEffect()
        effect_ace.set_effect_name("LM-021 Ace Overflow <-5>")
        effect_ace.set_effect_description("Ace Overflow <-5>")
        effect_ace.is_inherited_effect = True
        def condition_ace(context: Dict[str, Any]) -> bool:
            return True
        effect_ace.set_can_use_condition(condition_ace)
        effects.append(effect_ace)

        return effects
