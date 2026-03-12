from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT12_057(CardScript):
    """BT12-057 Quartzmon | Lv.7 (Green, Unidentified, DP 15000, Cost 16)

    [When Digivolving] Suspend all other Digimon and all Tamers. Then, for
        every 2 suspended Digimon and/or Tamers, gain 1 memory.
    [All Turns] All other Digimon and Tamers don't unsuspend.
    [When Attacking] Suspend 1 of your opponent's Digimon or Tamers. Then,
        trash the top card of your opponent's security stack for every 5
        suspended Digimon and Tamers.

    Engine notes:
    - "All other don't unsuspend" is now implemented via CANNOT_UNSUSPEND modifier
      registered at digivolve time. The modifier's condition excludes Quartzmon itself,
      so it acts as an aura affecting all current AND future permanents.
    - Auto-cleaned when Quartzmon leaves the field via cleanup_modifiers_for_permanent.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _get_all_digimon_tamers(game, exclude_perm=None):
            """Return list of all Digimon/Tamer permanents on both fields."""
            result = []
            for pl in [game.player1, game.player2]:
                for p in pl.battle_area:
                    if p is exclude_perm:
                        continue
                    if p.is_digimon or p.is_tamer:
                        result.append(p)
            return result

        # --- Effect 0: [When Digivolving] Suspend all other Digimon/Tamers,
        #     then +1 memory per 2 suspended, then register CANNOT_UNSUSPEND aura ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name(
            "BT12-057 Suspend all other Digimon/Tamers, gain memory"
        )
        effect0.set_effect_description(
            "[When Digivolving] Suspend all other Digimon and all Tamers. "
            "Then, for every 2 suspended Digimon and/or Tamers, gain 1 memory."
        )
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            owner_perm = card.permanent_of_this_card() if card else None

            # Suspend all other Digimon and all Tamers (both fields)
            for p in _get_all_digimon_tamers(game, exclude_perm=owner_perm):
                if not p.is_suspended:
                    p.suspend()

            # Count ALL currently suspended Digimon and Tamers (both fields, excl. Quartzmon)
            total_suspended = sum(
                1 for p in _get_all_digimon_tamers(game, exclude_perm=owner_perm)
                if p.is_suspended
            )

            # Gain 1 memory per 2 suspended
            memory_gain = total_suspended // 2
            if memory_gain > 0:
                player.add_memory(memory_gain)
                game.logger.log(
                    f"[BT12-057] Suspended {total_suspended} Digimon/Tamers, "
                    f"gained {memory_gain} memory."
                )

            # Register CANNOT_UNSUSPEND aura — affects all other Digimon/Tamers
            # (including those entering the field later)
            if owner_perm:
                game.register_modifier(
                    owner_perm,
                    ModifierType.CANNOT_UNSUSPEND,
                    condition=lambda target, ctx: (
                        target is not owner_perm
                        and (target.is_digimon or target.is_tamer)
                    ),
                    source_effect=effect0,
                    expiry='permanent',
                )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [When Attacking] Suspend 1 opponent Digimon/Tamer,
        #     trash security per 5 suspended ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name(
            "BT12-057 Suspend opponent Digimon/Tamer, trash security per 5 suspended"
        )
        effect1.set_effect_description(
            "[When Attacking] Suspend 1 of your opponent's Digimon or Tamers. "
            "Then, trash the top card of your opponent's security stack for "
            "every 5 suspended Digimon and Tamers."
        )
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            enemy = player.enemy
            if not enemy:
                return

            def count_and_trash_security():
                # Count all suspended Digimon and Tamers on both fields
                total_suspended = sum(
                    1 for pl in [game.player1, game.player2]
                    for p in pl.battle_area
                    if (p.is_digimon or p.is_tamer) and p.is_suspended
                )
                security_trashes = total_suspended // 5
                game.logger.log(
                    f"[BT12-057] {total_suspended} suspended → trash "
                    f"{security_trashes} security card(s)."
                )
                for _ in range(security_trashes):
                    if not enemy.security_cards:
                        break
                    top_sec = enemy.security_cards[-1]
                    enemy.trash_security_card(top_sec)

            # Select an opponent Digimon or Tamer to suspend
            has_target = any(
                (p.is_digimon or p.is_tamer) for p in enemy.battle_area
            )
            if has_target:
                def on_suspend_target(target_perm):
                    target_perm.suspend()
                    count_and_trash_security()

                game.effect_select_opponent_permanent(
                    player, on_suspend_target,
                    filter_fn=lambda p: p.is_digimon or p.is_tamer,
                    is_optional=False,
                    prompt="Select 1 of your opponent's Digimon or Tamers to suspend.",
                )
            else:
                # No valid target — still trash security based on current suspended count
                count_and_trash_security()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
