from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_023(CardScript):
    """EX10-023 Quartzmon | Lv.7 (Green, Cost 9)

    [Hand] [Counter] <Blast Digivolve>
    [On Play] [When Digivolving] Suspend all other Digimon and Tamers.
    [When Digivolving] [When Attacking] [Once Per Turn] Delete 1 of your
        opponent's suspended Digimon.
    [All Turns] Other than this Digimon, no Digimon or Tamers can unsuspend
        in the unsuspend phase.
    Inherited: Ace Overflow <-5>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Alt digivolve from Ryoma Mogami or Astamon ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-023 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 7
        effect0._alt_digi_name = "Ryoma Mogami"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Ryoma Mogami') or permanent.contains_card_name('Astamon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Blast Digivolve ---
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-023 Blast Digivolve")
        effect1.set_effect_description("Blast Digivolve")
        effect1.is_counter_effect = True
        effect1._is_blast_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Shared suspend logic ---
        def _suspend_all_others(player, game):
            own_perm = card.permanent_of_this_card() if card else None
            if not own_perm:
                return
            for pl in [game.player1, game.player2]:
                for p in pl.battle_area:
                    if p is own_perm:
                        continue
                    if (p.is_digimon or p.is_tamer) and not p.is_suspended:
                        p.suspend()
            # Register CANNOT_UNSUSPEND aura for all other Digimon/Tamers
            game.register_modifier(
                own_perm,
                ModifierType.CANNOT_UNSUSPEND,
                condition=lambda target, ctx: (
                    target is not own_perm
                    and (target.is_digimon or target.is_tamer)
                ),
                expiry='permanent',
            )

        # --- [On Play] Suspend all other Digimon and Tamers ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX10-023 On Play: Suspend all others")
        effect2.set_effect_description(
            "[On Play] Suspend all other Digimon and Tamers."
        )
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                _suspend_all_others(player, game)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- [When Digivolving] Suspend all other Digimon and Tamers ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX10-023 When Digivolving: Suspend all others")
        effect3.set_effect_description(
            "[When Digivolving] Suspend all other Digimon and Tamers."
        )
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(process2)  # same logic
        effects.append(effect3)

        # --- [When Digivolving] [When Attacking] [OPT] Delete suspended opponent Digimon ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("EX10-023 Delete 1 suspended opponent Digimon")
        effect4.set_effect_description(
            "[When Digivolving] [Once Per Turn] Delete 1 of your opponent's "
            "suspended Digimon."
        )
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("EX10_023_DeleteSuspended")
        effect4.is_when_digivolving = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            def target_filter(p):
                return p.is_digimon and p.is_suspended

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            has_target = any(target_filter(p) for p in enemy.battle_area)
            if has_target:
                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # --- [When Attacking] [OPT] Delete suspended opponent Digimon ---
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnUseAttack)
        effect5.set_effect_name("EX10-023 When Attacking: Delete 1 suspended Digimon")
        effect5.set_effect_description(
            "[When Attacking] [Once Per Turn] Delete 1 of your opponent's "
            "suspended Digimon."
        )
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("EX10_023_DeleteSuspended")
        effect5.is_on_attack = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect5.set_can_use_condition(condition5)
        effect5.set_on_process_callback(process4)  # same delete logic
        effects.append(effect5)

        # --- Ace Overflow <-5> ---
        effect6 = ICardEffect()
        effect6.set_effect_name("EX10-023 Ace Overflow -5")
        effect6.set_effect_description("Ace Overflow <-5>")
        effect6.is_inherited_effect = True
        effect6._is_ace_overflow = True
        effect6._ace_overflow_value = -5

        def condition6(context: Dict[str, Any]) -> bool:
            return True
        effect6.set_can_use_condition(condition6)
        effects.append(effect6)

        return effects
