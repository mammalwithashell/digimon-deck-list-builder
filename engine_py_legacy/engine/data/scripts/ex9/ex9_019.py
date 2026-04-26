from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_019(CardScript):
    """EX9-019 WereGarurumon: Sagittarius Mode | Lv.5 Blue/Purple

    Alt-digi 1: [WereGarurumon] exact name: Cost 1
    Alt-digi 2a: Lv.4 w/[Garurumon] in name: Cost 3
    Alt-digi 2b: Lv.4 w/[ADVENTURE] trait: Cost 3

    [On Play] [When Digivolving] 1 of your opponent's Digimon or Tamers can't
        suspend until their turn ends.

    [Your Turn] When any of your Digimon or Tamers with [Greymon] or
        [Matt Ishida] in its name are played or any of your Digimon digivolve
        into a Digimon with [Greymon] in its name, this Digimon may digivolve
        into a Digimon card with [Garurumon] in its name in the hand without
        paying the cost.

    Inherited: [When Attacking] [Once Per Turn] <De-Digivolve 1> 1 of your
        opponent's Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Alt-digi 1: from [WereGarurumon] exact name for cost 1 ──────────
        effect_alt1 = ICardEffect()
        effect_alt1.set_effect_name("EX9-019 Alt digi (WereGarurumon exact)")
        effect_alt1.set_effect_description("Alt digi: from [WereGarurumon] for cost 1")
        effect_alt1._alt_digi_cost = 1
        effect_alt1._alt_digi_name = "WereGarurumon"
        effect_alt1._alt_digi_exact = True

        def condition_alt1(context: Dict[str, Any]) -> bool:
            return True
        effect_alt1.set_can_use_condition(condition_alt1)
        effects.append(effect_alt1)

        # ── Alt-digi 2a: Lv.4 w/[Garurumon] in name for cost 3 ─────────────
        effect_alt2a = ICardEffect()
        effect_alt2a.set_effect_name("EX9-019 Alt digi (Lv.4 Garurumon name)")
        effect_alt2a.set_effect_description("Alt digi: Lv.4 w/[Garurumon] in name for cost 3")
        effect_alt2a._alt_digi_cost = 3
        effect_alt2a._alt_digi_level = 4
        effect_alt2a._alt_digi_name = "Garurumon"

        def condition_alt2a(context: Dict[str, Any]) -> bool:
            return True
        effect_alt2a.set_can_use_condition(condition_alt2a)
        effects.append(effect_alt2a)

        # ── Alt-digi 2b: Lv.4 w/[ADVENTURE] trait for cost 3 ────────────────
        effect_alt2b = ICardEffect()
        effect_alt2b.set_effect_name("EX9-019 Alt digi (Lv.4 ADVENTURE trait)")
        effect_alt2b.set_effect_description("Alt digi: Lv.4 w/[ADVENTURE] trait for cost 3")
        effect_alt2b._alt_digi_cost = 3
        effect_alt2b._alt_digi_level = 4
        effect_alt2b._alt_digi_trait = "ADVENTURE"

        def condition_alt2b(context: Dict[str, Any]) -> bool:
            return True
        effect_alt2b.set_can_use_condition(condition_alt2b)
        effects.append(effect_alt2b)

        # ── Shared process: CANNOT_SUSPEND 1 opponent Digimon/Tamer ──────��──
        def _cannot_suspend_process(ctx: Dict[str, Any]):
            """Select 1 opponent Digimon/Tamer -> CANNOT_SUSPEND until their turn ends."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            def target_filter(p):
                return p.is_digimon or p.is_tamer

            if not any(target_filter(p) for p in enemy.battle_area):
                return

            def on_target(target_perm):
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    target_perm, ModifierType.CANNOT_SUSPEND,
                    condition=lambda target, ctx, fp=target_perm: target is fp,
                    expiry='end_of_opponent_turn')
                game.logger.log(
                    f"[Effect] {game._perm_ref(target_perm)} can't suspend "
                    f"until opponent's turn ends")

            game.effect_select_opponent_permanent(
                player, on_target,
                filter_fn=target_filter,
                is_optional=False,
                prompt="Select 1 opponent Digimon/Tamer that can't suspend.")

        # ── [On Play] CANNOT_SUSPEND ────────────────────────────────────────
        effect_op = ICardEffect()
        effect_op.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_op.set_effect_name("EX9-019 CANNOT_SUSPEND (On Play)")
        effect_op.set_effect_description(
            "[On Play] 1 of your opponent's Digimon or Tamers can't suspend "
            "until their turn ends."
        )
        effect_op.is_on_play = True

        def condition_op(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_op.set_can_use_condition(condition_op)
        effect_op.set_on_process_callback(_cannot_suspend_process)
        effects.append(effect_op)

        # ── [When Digivolving] CANNOT_SUSPEND ───────────────────────────────
        effect_wd = ICardEffect()
        effect_wd.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_wd.set_effect_name("EX9-019 CANNOT_SUSPEND (When Digivolving)")
        effect_wd.set_effect_description(
            "[When Digivolving] 1 of your opponent's Digimon or Tamers can't "
            "suspend until their turn ends."
        )
        effect_wd.is_when_digivolving = True

        def condition_wd(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_wd.set_can_use_condition(condition_wd)
        effect_wd.set_on_process_callback(_cannot_suspend_process)
        effects.append(effect_wd)

        # ── [Your Turn] Reactive: Greymon/Matt Ishida play or Greymon digi ──
        # When any of your Digimon or Tamers with [Greymon] or [Matt Ishida]
        # in name are played, OR any of your Digimon digivolve into [Greymon],
        # this Digimon may digivolve into [Garurumon] from hand free.
        effect_reactive = ICardEffect()
        effect_reactive.set_effect_name("EX9-019 Reactive digivolve into Garurumon")
        effect_reactive.set_effect_description(
            "[Your Turn] When Greymon/Matt Ishida played or digivolve into "
            "Greymon, this may digivolve into [Garurumon] from hand free."
        )
        effect_reactive.is_optional = True
        effect_reactive._is_play_observer = True
        effect_reactive._is_digivolve_observer = True

        def condition_reactive(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            # Play trigger: Digimon or Tamer with [Greymon] or [Matt Ishida] in name
            played_perm = context.get('played_permanent')
            if played_perm is not None:
                if played_perm.top_card:
                    if (played_perm.is_digimon or played_perm.is_tamer):
                        if (played_perm.contains_card_name('Greymon') or
                                played_perm.contains_card_name('Matt Ishida')):
                            return True

            # Digivolve trigger: Digimon digivolves into [Greymon] in name
            digi_perm = context.get('digivolved_permanent')
            if digi_perm is not None:
                if digi_perm.is_digimon and digi_perm.top_card:
                    if digi_perm.contains_card_name('Greymon'):
                        return True

            return False

        effect_reactive.set_can_use_condition(condition_reactive)

        def process_reactive(ctx: Dict[str, Any]):
            """Digivolve this Digimon into a [Garurumon]-name card from hand free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            my_perm = card.permanent_of_this_card() if card else None
            if not my_perm:
                return

            def garurumon_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                names = getattr(c, 'card_names', []) or []
                return any('Garurumon' in n for n in names)

            game.effect_digivolve_from_hand(
                player, my_perm, garurumon_filter,
                cost_override=0, is_optional=True,
                prompt="Select a Digimon card with [Garurumon] in its name "
                       "to digivolve this Digimon into (free).")

        effect_reactive.set_on_process_callback(process_reactive)
        effects.append(effect_reactive)

        # ── Inherited: [When Attacking] [Once Per Turn] De-Digivolve 1 ──────
        effect_inh = ICardEffect()
        effect_inh.set_timing(EffectTiming.OnUseAttack)
        effect_inh.set_effect_name("EX9-019 Inherited: De-Digivolve 1")
        effect_inh.set_effect_description(
            "[When Attacking] [Once Per Turn] <De-Digivolve 1> 1 of your "
            "opponent's Digimon."
        )
        effect_inh.is_inherited_effect = True
        effect_inh.is_on_attack = True
        effect_inh.is_optional = True
        effect_inh.set_max_count_per_turn(1)
        effect_inh.set_hash_string("WA_EX9-019")

        def condition_inh(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be the attacking permanent
            perm = card.permanent_of_this_card()
            ctx_perm = context.get('attacker') or context.get('permanent')
            if perm and ctx_perm and ctx_perm is not perm:
                return False
            # Must have an opponent Digimon to target
            owner = card.owner if card else None
            if not owner:
                return False
            enemy = owner.enemy
            if not enemy:
                return False
            return any(p.is_digimon for p in enemy.battle_area)

        effect_inh.set_can_use_condition(condition_inh)

        def process_inh(ctx: Dict[str, Any]):
            """<De-Digivolve 1> 1 opponent Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            def on_select(target_perm):
                removed = target_perm.de_digivolve(1)
                for c in removed:
                    enemy.trash_cards.append(c)
                game.logger.log(
                    f"[Effect] <De-Digivolve 1> {game._perm_ref(target_perm)}")

            game.effect_select_opponent_permanent(
                player, on_select,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Select 1 of your opponent's Digimon to De-Digivolve 1.")

        effect_inh.set_on_process_callback(process_inh)
        effects.append(effect_inh)

        return effects
