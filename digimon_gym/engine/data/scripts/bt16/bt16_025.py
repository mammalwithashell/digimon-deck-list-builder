from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_025(CardScript):
    """BT16-025 Paildramon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Partition (non-inherited) ────────────────────────────────────
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-025 Partition")
        effect0.set_effect_description("Partition")
        effect0._is_partition = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ── Partition (inherited) ────────────────────────────────────────
        effect0i = ICardEffect()
        effect0i.set_effect_name("BT16-025 Partition")
        effect0i.set_effect_description("Partition")
        effect0i.is_inherited_effect = True
        effect0i._is_partition = True

        def condition0i(context: Dict[str, Any]) -> bool:
            return True
        effect0i.set_can_use_condition(condition0i)
        effects.append(effect0i)

        # ── DNA Digivolve condition ──────────────────────────────────────
        effect1 = ICardEffect()
        effect1.set_effect_name("BT16-025 Jogress Condition")
        effect1.set_effect_description("Jogress Condition")

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # ── [When Digivolving] ───────────────────────────────────────────
        # Suspend all of your opponent's Digimon with as many or fewer
        # digivolution cards as this Digimon. Then, if DNA digivolving,
        # none of your opponent's Digimon can unsuspend until the end of
        # their turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT16-025 Suspend opponent Digimon + DNA cannot unsuspend")
        effect2.set_effect_description(
            "[When Digivolving] Suspend all of your opponent's Digimon with "
            "as many or fewer digivolution cards as this Digimon. Then, if "
            "DNA digivolving, none of your opponent's Digimon can unsuspend "
            "until the end of their turn."
        )
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Digivolution cards = cards below the top card
            # card_sources includes the top card, so digi card count =
            # len(card_sources) - 1
            my_digi_count = len(perm.card_sources) - 1

            for opp_perm in list(enemy.battle_area):
                if not opp_perm.is_digimon:
                    continue
                opp_digi_count = len(opp_perm.card_sources) - 1
                if opp_digi_count <= my_digi_count:
                    opp_perm.suspend()

            # If DNA digivolving, ALL opponent Digimon cannot unsuspend
            # until end of opponent's turn
            is_dna = ctx.get('is_dna_digivolve', False)
            if not is_dna:
                return

            from digimon_gym.engine.interfaces.modifiers import ModifierType

            for opp_perm in list(enemy.battle_area):
                if opp_perm.is_digimon:
                    _ref = opp_perm  # capture for closure
                    game.register_modifier(
                        opp_perm, ModifierType.CANNOT_UNSUSPEND,
                        condition=lambda p, ctx, _r=_ref: p is _r,
                        value_fn=lambda: True,
                        expiry='end_of_opponent_turn'
                    )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # ── [When Attacking] [Once Per Turn] ─────────────────────────────
        # Suspend 1 of your opponent's unsuspended Digimon. If this effect
        # didn't suspend, unsuspend this Digimon.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUseAttack)
        effect3.set_effect_name("BT16-025 Suspend 1 opp Digimon or unsuspend self")
        effect3.set_effect_description(
            "[When Attacking] [Once Per Turn] Suspend 1 of your opponent's "
            "unsuspended Digimon. If this effect didn't suspend, unsuspend "
            "this Digimon."
        )
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("WhenAttacking_BT16_025")
        effect3.is_on_attack = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return
            enemy = player.enemy
            if not enemy:
                return

            def target_filter(p):
                return p.is_digimon and not p.is_suspended

            has_targets = any(
                target_filter(p) for p in enemy.battle_area
            )

            if has_targets:
                def on_suspend(target_perm):
                    target_perm.suspend()
                    # Successfully suspended a target — do NOT unsuspend self

                game.effect_select_opponent_permanent(
                    player, on_suspend,
                    filter_fn=target_filter,
                    is_optional=False
                )
            else:
                # No valid targets to suspend — unsuspend this Digimon
                perm.unsuspend()

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
