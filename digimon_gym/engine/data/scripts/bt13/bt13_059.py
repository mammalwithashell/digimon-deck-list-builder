from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_059(CardScript):
    """BT13-059 Examon | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Jogress Condition (handled by DNA cost in database, purely visual here)
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-059 Jogress Condition")
        effect0.set_effect_description("Jogress Condition")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Helper for Suspend + Freeze
        def suspend_freeze_logic(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_suspend(target_perm):
                target_perm.suspend()
                # "That Digimon doesn't unsuspend during your opponent's next unsuspend phase."
                # Grant keyword until the end of opponent's next turn?
                # Or just expiry logic.
                # Turn sequence: My Turn (T) -> Opponent Turn (T+1). Unsuspend phase is start of T+1.
                # So it needs to persist through T+1's start.
                # grant_keyword(duration=T+1) means it expires at end of T+1?
                # Usually duration is expiry turn index.
                target_perm.grant_keyword('_is_cannot_unsuspend', game.turn_count + 1)

            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=lambda p: True, is_optional=False)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Suspend 1 of your opponent's Digimon. That Digimon doesn't unsuspend during your opponent's next unsuspend phase.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-059 [On Play] Suspend & Freeze")
        effect1.set_effect_description("[On Play] Suspend 1 of your opponent's Digimon. That Digimon doesn't unsuspend during your opponent's next unsuspend phase.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(suspend_freeze_logic)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] ...
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-059 [When Digivolving] Suspend & Freeze")
        effect2.set_effect_description("[When Digivolving] Suspend 1 of your opponent's Digimon. That Digimon doesn't unsuspend during your opponent's next unsuspend phase.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(suspend_freeze_logic)
        effects.append(effect2)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns][Once Per Turn] When an opponent's Digimon becomes suspended, you may activate 1 of the effects below.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT13-059 Choose 1 effect")
        effect3.set_effect_description("[All Turns][Once Per Turn] When an opponent's Digimon becomes suspended, activate 1: Suspend 1 of opponent's Digimon OR Unsuspend 1 of your Digimon.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("SelectEffects_BT13_059")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Context check: Opponent's Digimon suspended
            suspended = context.get('suspended_permanent')
            if not suspended:
                return False
            if not suspended.is_digimon:
                return False
            player = context.get('player')
            if not player or suspended in player.battle_area:
                return False # Not opponent's
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_branch_selected(branch_idx: int):
                if branch_idx == 0:
                    # Suspend 1 of opponent's Digimon
                    game.effect_select_opponent_permanent(
                        player,
                        lambda p: p.suspend(),
                        filter_fn=lambda p: True,
                        is_optional=False
                    )
                elif branch_idx == 1:
                    # Unsuspend 1 of your Digimon
                    game.effect_select_own_permanent(
                        player,
                        lambda p: p.unsuspend(),
                        filter_fn=lambda p: True,
                        is_optional=False
                    )

            game.effect_choose_branch(player, 2, on_branch_selected)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
