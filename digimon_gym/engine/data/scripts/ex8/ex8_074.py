from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_074(CardScript):
    """EX8-074 MedievalGallantmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── BeforePayCost ─────────────────────────────────────────────
        # When this card would be played, by suspending 2 Digimon,
        # reduce the play cost by 4.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("EX8-074 Suspend 2 Digimon to get Play Cost -4")
        effect0.set_effect_description(
            "When this card would be played, by suspending 2 Digimon, "
            "reduce the play cost by 4."
        )
        effect0.set_hash_string("PlayCost-4_EX8_074")
        effect0.cost_reduction = 4

        def condition0(context: Dict[str, Any]) -> bool:
            # Only for THIS card being played (prevent cost-reduction leak)
            if context.get('card_source') is not card:
                return False
            # "by suspending 2 Digimon" — need 2+ unsuspended own Digimon
            if card and card.owner:
                own_digimon = [p for p in card.owner.battle_area
                               if p.is_digimon and not p.is_suspended]
                return len(own_digimon) >= 2
            return False

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Cost: Suspend 2 own unsuspended Digimon."""
            player = ctx.get('player')
            if not player:
                return
            own_digimon = [p for p in player.battle_area
                           if p.is_digimon and not p.is_suspended]
            if len(own_digimon) >= 2:
                own_digimon[0].suspend()
                own_digimon[1].suspend()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ── Alliance ──────────────────────────────────────────────────
        effect1 = ICardEffect()
        effect1.set_effect_name("EX8-074 Alliance")
        effect1.set_effect_description("Alliance")
        effect1.is_on_attack = True
        effect1._is_alliance = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # ── Vortex ────────────────────────────────────────────────────
        effect2 = ICardEffect()
        effect2.set_effect_name("EX8-074 Vortex")
        effect2.set_effect_description("Vortex")
        effect2._is_vortex = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # ── [When Digivolving] ────────────────────────────────────────
        # You may suspend 1 Digimon. Then, you may delete 1 of your
        # opponent's 8000 DP or lower Digimon. For each other suspended
        # Digimon, add 3000 to this DP deletion effect's maximum.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX8-074 Suspend then Delete")
        effect3.set_effect_description(
            "[When Digivolving] You may suspend 1 Digimon. Then, you may "
            "delete 1 of your opponent's 8000 DP or lower Digimon. For each "
            "other suspended Digimon, add 3000 to this DP deletion effect's maximum."
        )
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """You may suspend 1 Digimon, then optionally delete opponent Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: You MAY suspend 1 Digimon (own)
            def on_suspend_done(target_perm):
                target_perm.suspend()
                # Step 2: After suspending, proceed to delete
                _do_delete(player, perm, game)

            def on_suspend_decline():
                # Player chose not to suspend, still proceed to delete step
                _do_delete(player, perm, game)

            own_unsuspended = [p for p in player.battle_area
                               if p.is_digimon and not p.is_suspended]
            if own_unsuspended:
                game.effect_select_own_permanent(
                    player, on_suspend_done,
                    filter_fn=lambda p: p.is_digimon and not p.is_suspended,
                    is_optional=True,
                    prompt="You may suspend 1 Digimon.")
                if game.pending_selection:
                    game.pending_selection.on_decline = on_suspend_decline
            else:
                # No unsuspended Digimon — skip to delete
                _do_delete(player, perm, game)

        def _do_delete(player, owner_perm, game):
            """Step 2: You MAY delete 1 opponent Digimon with DP <= base + scaling."""
            # Count all suspended Digimon on both fields ("other suspended Digimon")
            enemy = player.enemy if player else None
            all_suspended = sum(
                1 for p in list(player.battle_area) + list(enemy.battle_area if enemy else [])
                if p.is_suspended
            )
            max_dp = 8000 + 3000 * all_suspended

            def delete_filter(p):
                return p.is_digimon and (p.dp or 0) <= max_dp

            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=delete_filter,
                is_optional=True,
                prompt=f"You may delete 1 opponent Digimon with {max_dp} DP or less.")

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # ── [All Turns] [Once Per Turn] ───────────────────────────────
        # When Digimon are played, you may activate 1 of this Digimon's
        # [When Digivolving] effects.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("EX8-074 Replay When Digivolving on play")
        effect4.set_effect_description(
            "[All Turns] [Once Per Turn] When Digimon are played, you may "
            "activate 1 of this Digimon's [When Digivolving] effects."
        )
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("PlayActivate_EX8_074")

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Check that a Digimon was played
            played = context.get('played_card')
            if played is None:
                return False
            played_perm = context.get('played_permanent')
            if played_perm and not played_perm.is_digimon:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Re-run the When Digivolving effect (suspend + delete)."""
            process3(ctx)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
