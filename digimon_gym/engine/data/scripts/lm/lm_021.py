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

        # Shared process: Delete any number of opponent's Digimon whose total DP <= this Digimon's DP
        def _shared_delete_process(ctx: Dict[str, Any]):
            """Delete opponent's Digimon up to this Digimon's DP total (iterative player selection)."""
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

            from ....game.constants import SEL_OPP_FIELD_START
            from ....data.enums import GamePhase

            # Track running total and which permanents have been selected for deletion
            state = {'total_dp': 0, 'to_delete': []}

            def _select_next():
                """Let player iteratively select opponent Digimon to delete."""
                remaining_budget = my_dp - state['total_dp']
                if remaining_budget <= 0:
                    _execute_deletions()
                    return

                opp = player.enemy if player else None
                if not opp:
                    _execute_deletions()
                    return

                # Build valid indices for opponent Digimon within budget
                valid = []
                for i, p in enumerate(opp.battle_area):
                    if not p.is_digimon:
                        continue
                    if p in state['to_delete']:
                        continue
                    if (p.dp or 0) <= remaining_budget:
                        if not game.modifiers.can_be_selected_by_effect(p):
                            continue
                        valid.append(SEL_OPP_FIELD_START + i)

                if not valid:
                    _execute_deletions()
                    return

                def on_selected(action_id):
                    idx = action_id - SEL_OPP_FIELD_START
                    opp2 = player.enemy if player else None
                    if opp2 and 0 <= idx < len(opp2.battle_area):
                        target = opp2.battle_area[idx]
                        state['total_dp'] += (target.dp or 0)
                        state['to_delete'].append(target)
                    _select_next()

                game.request_selection(
                    GamePhase.SelectTarget, player, on_selected,
                    valid, is_optional=True,
                    prompt=f"Select an opponent's Digimon to delete (budget: {remaining_budget} DP remaining).",
                    on_decline=_execute_deletions)

            def _execute_deletions():
                """Delete all selected Digimon."""
                for target in state['to_delete']:
                    if target in enemy.battle_area:
                        enemy.delete_permanent(target)

            _select_next()

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
