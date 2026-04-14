from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class AD1_014(CardScript):
    """AD1-014 MetalGarurumon | Lv.6 Blue/White

    <Blocker>
    <Evade>
    [On Play] [When Digivolving] [When Attacking] [Once Per Turn] Delete 1 of
    your opponent's level 5 or lower Digimon. Then, for every 2 of your Tamers'
    colors, 1 of your opponent's Digimon or Tamers can't suspend until their
    turn ends.

    [All Turns] [Once Per Turn] When any of your Digimon suspend, this Digimon
    may unsuspend.

    Inherited:
    [When Attacking] [Once Per Turn] If this Digimon has [Garurumon] or
    [Omnimon] in its name, 1 of your opponent's Digimon or Tamers can't suspend
    until their turn ends.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Alt-digi 1: Lv.5 with [Garurumon] in name, cost 3 ---
        alt1 = ICardEffect()
        alt1.set_effect_name("AD1-014 Alt-digi: Garurumon name")
        alt1.set_effect_description("Lv.5 w/[Garurumon] in name: Cost 3")
        alt1._alt_digi_cost = 3
        alt1._alt_digi_level = 5
        alt1._alt_digi_name = "Garurumon"

        def cond_alt1(context: Dict[str, Any]) -> bool:
            return True
        alt1.set_can_use_condition(cond_alt1)
        effects.append(alt1)

        # --- Alt-digi 2: Lv.5 with [ADVENTURE] trait, cost 3 ---
        alt2 = ICardEffect()
        alt2.set_effect_name("AD1-014 Alt-digi: ADVENTURE trait")
        alt2.set_effect_description("Lv.5 w/[ADVENTURE] trait: Cost 3")
        alt2._alt_digi_cost = 3
        alt2._alt_digi_level = 5
        alt2._alt_digi_trait = "ADVENTURE"

        def cond_alt2(context: Dict[str, Any]) -> bool:
            return True
        alt2.set_can_use_condition(cond_alt2)
        effects.append(alt2)

        # --- Alt-digi 3: Lv.5 with [Hero] trait, cost 3 ---
        alt3 = ICardEffect()
        alt3.set_effect_name("AD1-014 Alt-digi: Hero trait")
        alt3.set_effect_description("Lv.5 w/[Hero] trait: Cost 3")
        alt3._alt_digi_cost = 3
        alt3._alt_digi_level = 5
        alt3._alt_digi_trait = "Hero"

        def cond_alt3(context: Dict[str, Any]) -> bool:
            return True
        alt3.set_can_use_condition(cond_alt3)
        effects.append(alt3)

        # --- Blocker ---
        eff_blocker = ICardEffect()
        eff_blocker.set_effect_name("AD1-014 Blocker")
        eff_blocker.set_effect_description("Blocker")
        eff_blocker._is_blocker = True

        def cond_blocker(context: Dict[str, Any]) -> bool:
            return True
        eff_blocker.set_can_use_condition(cond_blocker)
        effects.append(eff_blocker)

        # --- Evade ---
        eff_evade = ICardEffect()
        eff_evade.set_effect_name("AD1-014 Evade")
        eff_evade.set_effect_description("Evade")
        eff_evade._is_evade = True

        def cond_evade(context: Dict[str, Any]) -> bool:
            return True
        eff_evade.set_can_use_condition(cond_evade)
        effects.append(eff_evade)

        # --- Shared [On Play] [When Digivolving] [When Attacking] OPT ---
        # Delete 1 opponent Lv5 or lower Digimon, then freeze N opponent
        # Digimon/Tamers where N = (distinct tamer colors) // 2

        def _make_shared_process():
            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return
                enemy = player.enemy
                if not enemy:
                    return

                # Step 1: Delete 1 opponent Lv5 or lower Digimon (mandatory)
                def delete_filter(p):
                    return p.is_digimon and (p.level or 0) <= 5

                def on_delete(target_perm):
                    enemy.delete_permanent(target_perm)
                    game.logger.log(
                        f"[Effect] AD1-014 deleted {game._perm_ref(target_perm)}"
                    )
                    # Step 2: count distinct tamer colors and freeze
                    _apply_tamer_color_freeze(player, game, enemy)

                if any(delete_filter(p) for p in enemy.battle_area):
                    game.effect_select_opponent_permanent(
                        player, on_delete, filter_fn=delete_filter,
                        is_optional=False,
                        prompt="Select 1 of your opponent's Lv5 or lower Digimon to delete."
                    )
                else:
                    # No valid delete targets -- still apply freeze
                    _apply_tamer_color_freeze(player, game, enemy)

            return process

        def _apply_tamer_color_freeze(player, game, enemy):
            """Count distinct colors across all own Tamers, freeze N opponent permanents."""
            all_colors = set()
            for p in player.battle_area:
                if p.is_tamer and p.top_card:
                    for color in p.top_card.card_colors:
                        all_colors.add(color)
            n_freeze = len(all_colors) // 2
            if n_freeze <= 0:
                return

            _frozen = []

            def _do_freeze(n_remaining):
                if n_remaining <= 0:
                    return

                def freeze_filter(p):
                    return (p.is_digimon or p.is_tamer) and p not in _frozen

                def on_freeze(target_perm):
                    from digimon_gym.engine.interfaces.modifiers import ModifierType
                    game.register_modifier(
                        target_perm, ModifierType.CANNOT_SUSPEND,
                        condition=lambda target, ctx, fp=target_perm: target is fp,
                        expiry='end_of_opponent_turn'
                    )
                    game.logger.log(
                        f"[Effect] AD1-014: {game._perm_ref(target_perm)} "
                        f"can't suspend until opponent's turn ends"
                    )
                    _frozen.append(target_perm)
                    _do_freeze(n_remaining - 1)

                if any(freeze_filter(p) for p in enemy.battle_area):
                    game.effect_select_opponent_permanent(
                        player, on_freeze, filter_fn=freeze_filter,
                        is_optional=False,
                        prompt=f"Select opponent's Digimon/Tamer that can't suspend "
                               f"({len(_frozen) + 1}/{n_freeze})."
                    )

            _do_freeze(n_freeze)

        shared_process = _make_shared_process()

        # [On Play] trigger
        eff_op = ICardEffect()
        eff_op.set_timing(EffectTiming.OnEnterFieldAnyone)
        eff_op.set_effect_name("AD1-014 [On Play] Delete + Freeze")
        eff_op.set_effect_description(
            "[On Play] [Once Per Turn] Delete 1 opponent's Lv5 or lower Digimon. "
            "Then freeze opponent Digimon/Tamers based on Tamer colors."
        )
        eff_op.is_on_play = True
        eff_op.set_max_count_per_turn(1)
        eff_op.set_hash_string("AD1_014_OP_WD_WA")

        def cond_op(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        eff_op.set_can_use_condition(cond_op)
        eff_op.set_on_process_callback(shared_process)
        effects.append(eff_op)

        # [When Digivolving] trigger
        eff_wd = ICardEffect()
        eff_wd.set_timing(EffectTiming.OnEnterFieldAnyone)
        eff_wd.set_effect_name("AD1-014 [When Digivolving] Delete + Freeze")
        eff_wd.set_effect_description(
            "[When Digivolving] [Once Per Turn] Delete 1 opponent's Lv5 or lower Digimon. "
            "Then freeze opponent Digimon/Tamers based on Tamer colors."
        )
        eff_wd.is_when_digivolving = True
        eff_wd.set_max_count_per_turn(1)
        eff_wd.set_hash_string("AD1_014_OP_WD_WA")

        def cond_wd(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        eff_wd.set_can_use_condition(cond_wd)
        eff_wd.set_on_process_callback(shared_process)
        effects.append(eff_wd)

        # [When Attacking] trigger
        eff_wa = ICardEffect()
        eff_wa.set_timing(EffectTiming.OnUseAttack)
        eff_wa.set_effect_name("AD1-014 [When Attacking] Delete + Freeze")
        eff_wa.set_effect_description(
            "[When Attacking] [Once Per Turn] Delete 1 opponent's Lv5 or lower Digimon. "
            "Then freeze opponent Digimon/Tamers based on Tamer colors."
        )
        eff_wa.is_on_attack = True
        eff_wa.set_max_count_per_turn(1)
        eff_wa.set_hash_string("AD1_014_OP_WD_WA")

        def cond_wa(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        eff_wa.set_can_use_condition(cond_wa)
        eff_wa.set_on_process_callback(shared_process)
        effects.append(eff_wa)

        # --- [All Turns] [Once Per Turn] When any of your Digimon suspend,
        #     this Digimon may unsuspend. ---
        eff_unsuspend = ICardEffect()
        eff_unsuspend.set_effect_name(
            "AD1-014 Unsuspend when own Digimon suspends"
        )
        eff_unsuspend.set_effect_description(
            "[All Turns] [Once Per Turn] When any of your Digimon suspend, "
            "this Digimon may unsuspend."
        )
        eff_unsuspend.is_optional = True
        eff_unsuspend._is_suspend_observer = True
        eff_unsuspend.set_max_count_per_turn(1)
        eff_unsuspend.set_hash_string("AD1_014_AT")

        def cond_unsuspend(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if not perm or not perm.is_suspended:
                return False
            # The suspended permanent must be one of our Digimon (not self)
            suspended_perm = context.get('suspended_permanent')
            if not suspended_perm or not suspended_perm.is_digimon:
                return False
            return True
        eff_unsuspend.set_can_use_condition(cond_unsuspend)

        def process_unsuspend(ctx: Dict[str, Any]):
            game = ctx.get('game')
            perm = card.permanent_of_this_card() if card else None
            if perm and game:
                perm.unsuspend()
                game.logger.log(
                    f"[Effect] AD1-014: {game._perm_ref(perm)} unsuspended"
                )
        eff_unsuspend.set_on_process_callback(process_unsuspend)
        effects.append(eff_unsuspend)

        # --- Inherited: [When Attacking] [Once Per Turn] ---
        # If this Digimon has [Garurumon] or [Omnimon] in its name,
        # 1 opponent Digimon or Tamer can't suspend until their turn ends.
        eff_ess = ICardEffect()
        eff_ess.set_timing(EffectTiming.OnUseAttack)
        eff_ess.set_effect_name(
            "AD1-014 Inherited: Freeze 1 opponent if Garurumon/Omnimon"
        )
        eff_ess.set_effect_description(
            "[When Attacking] [Once Per Turn] If this Digimon has [Garurumon] or "
            "[Omnimon] in its name, 1 of your opponent's Digimon or Tamers can't "
            "suspend until their turn ends."
        )
        eff_ess.is_inherited_effect = True
        eff_ess.is_on_attack = True
        eff_ess.set_max_count_per_turn(1)
        eff_ess.set_hash_string("AD1_014_ESS_WA")

        def cond_ess(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if not perm:
                return False
            if not (perm.contains_card_name('Garurumon')
                    or perm.contains_card_name('Omnimon')):
                return False
            return True
        eff_ess.set_can_use_condition(cond_ess)

        def process_ess(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            def freeze_filter(p):
                return p.is_digimon or p.is_tamer

            def on_freeze(target_perm):
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    target_perm, ModifierType.CANNOT_SUSPEND,
                    condition=lambda target, ctx, fp=target_perm: target is fp,
                    expiry='end_of_opponent_turn'
                )
                game.logger.log(
                    f"[Effect] AD1-014 inherited: {game._perm_ref(target_perm)} "
                    f"can't suspend until opponent's turn ends"
                )

            if any(freeze_filter(p) for p in enemy.battle_area):
                game.effect_select_opponent_permanent(
                    player, on_freeze, filter_fn=freeze_filter,
                    is_optional=False,
                    prompt="Select 1 opponent Digimon/Tamer that can't suspend."
                )
        eff_ess.set_on_process_callback(process_ess)
        effects.append(eff_ess)

        return effects
