from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


def _has_gallantmon_or_x_antibody(perm) -> bool:
    """Return True if perm's digivolution stack contains a card named 'Gallantmon'
    (substring match covers all Gallantmon variants) or a card with trait 'X Antibody'."""
    if perm is None:
        return False
    for cs in getattr(perm, 'card_sources', []):
        if cs is perm.top_card:
            # top_card is the card itself, skip — we only check digivolution cards
            continue
        # Name check: contains_card_name does case-insensitive substring match
        if getattr(cs, 'contains_card_name', lambda _: False)('Gallantmon'):
            return True
        # Trait check
        if any('X Antibody' in t for t in getattr(cs, 'card_traits', []) or []):
            return True
    return False


class EX8_073(CardScript):
    """EX8-073 Gallantmon (X Antibody) | Lv.6 | Red/Blue | Cost:12 | DP:12000"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Alternate digivolution: from [Gallantmon] for cost 1 ──────────────
        effect0 = ICardEffect()
        effect0.set_effect_name("EX8-073 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 1
        effect0._alt_digi_name = "Gallantmon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.contains_card_name('Gallantmon')):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ── Helper: shared process for the DP +4000 / opp -4000 effect ────────
        def _make_dp_effect(timing: EffectTiming, is_when_digivolving: bool):
            """[When Digivolving] / [When Attacking] — conditional on source check."""
            eff = ICardEffect()
            eff.set_timing(timing)
            label = "When Digivolving" if is_when_digivolving else "When Attacking"
            eff.set_effect_name(
                "EX8-073 This Digimon gets +4000 DP and gives -4000 DP to an opponent's Digimon"
            )
            eff.set_effect_description(
                f"[{label}] If [Gallantmon]/[X Antibody] is in this Digimon's digivolution cards, "
                "this Digimon gets +4000 DP and 1 of your opponent's Digimon gets -4000 DP until "
                "the end of their turn."
            )
            if is_when_digivolving:
                eff.is_when_digivolving = True
            else:
                eff.is_on_attack = True

            def cond(context: Dict[str, Any]) -> bool:
                perm = card.permanent_of_this_card() if card else None
                if perm is None:
                    return False
                # Source condition: Gallantmon or X Antibody must be in digivolution stack
                if not _has_gallantmon_or_x_antibody(perm):
                    return False
                return True
            eff.set_can_use_condition(cond)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent') or (card.permanent_of_this_card() if card else None)
                game = ctx.get('game')
                if not (player and game and perm):
                    return

                from engine_py_legacy.engine.interfaces.modifiers import ModifierType

                # This Digimon gets +4000 DP until end of opponent's turn
                game.register_modifier(
                    perm,
                    ModifierType.CHANGE_DP,
                    value_fn=lambda base, p, c: base + 4000,
                    expiry='end_of_opponent_turn',
                )

                # Select 1 opponent Digimon to get -4000 DP
                enemy = player.enemy if player else None
                if enemy:
                    opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
                    if opp_digimon:
                        def on_target_selected(target_perm):
                            game.register_modifier(
                                target_perm,
                                ModifierType.CHANGE_DP,
                                value_fn=lambda base, p, c: base - 4000,
                                expiry='end_of_opponent_turn',
                            )

                        game.effect_select_opponent_permanent(
                            player,
                            on_target_selected,
                            filter_fn=lambda p: p.is_digimon,
                            is_optional=False,
                            prompt="Select 1 opponent Digimon to get -4000 DP until end of their turn.",
                        )

            eff.set_on_process_callback(process)
            return eff

        effects.append(_make_dp_effect(EffectTiming.OnEnterFieldAnyone, is_when_digivolving=True))
        effects.append(_make_dp_effect(EffectTiming.OnUseAttack, is_when_digivolving=False))

        # ── Helper: shared process for the delete-or-trash effect ──────────────
        def _make_delete_or_trash_effect(timing: EffectTiming, is_when_digivolving: bool):
            """[When Digivolving] / [End of Attack] [Once Per Turn]
            Delete 1 opponent Digimon ≤10000 DP.
            If this didn't delete → trash top security + unsuspend this Digimon."""
            eff = ICardEffect()
            eff.set_timing(timing)
            label = "When Digivolving" if is_when_digivolving else "End of Attack"
            eff.set_effect_name(
                "EX8-073 Delete 1 Digimon with 10000 DP or less, or trash the opponent's top security and unsuspend"
            )
            eff.set_effect_description(
                f"[{label}] [Once Per Turn] Delete 1 of your opponent's Digimon with 10000 DP or "
                "less. If this didn't delete, trash your opponent's top security card and this "
                "Digimon unsuspends."
            )
            eff.set_max_count_per_turn(1)
            eff.set_hash_string("Delete_EX8_073")
            if is_when_digivolving:
                eff.is_when_digivolving = True

            def cond(context: Dict[str, Any]) -> bool:
                return card.permanent_of_this_card() is not None if card else False
            eff.set_can_use_condition(cond)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                enemy = player.enemy if player else None
                if not enemy:
                    return

                # Find opponent Digimon with ≤10000 DP
                delete_targets = [
                    p for p in enemy.battle_area
                    if p.is_digimon and (p.dp or 0) <= 10000
                ]

                # deleted[0] tracks whether a deletion actually happened
                deleted = [False]

                def on_delete_selected(target_perm):
                    deleted[0] = True
                    enemy.delete_permanent(target_perm)

                def on_decline_or_no_target():
                    # "If this didn't delete" — trash top security and unsuspend this Digimon
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)
                    this_perm = card.permanent_of_this_card() if card else None
                    if this_perm:
                        this_perm.unsuspend()

                if delete_targets:
                    # There are valid targets — player must select one
                    game.effect_select_opponent_permanent(
                        player,
                        on_delete_selected,
                        filter_fn=lambda p: p.is_digimon and (p.dp or 0) <= 10000,
                        is_optional=False,
                        prompt="Delete 1 of your opponent's Digimon with 10000 DP or less.",
                    )
                    # After the selection resolves, if nothing was deleted (e.g. immune),
                    # check deleted flag. Since selection is synchronous in the engine,
                    # on_decline_or_no_target is called only if no target existed.
                    # If selection succeeded, deleted[0] is True and we skip failure branch.
                    if not deleted[0]:
                        on_decline_or_no_target()
                else:
                    # No valid delete targets — failure branch fires immediately
                    on_decline_or_no_target()

            eff.set_on_process_callback(process)
            return eff

        effects.append(_make_delete_or_trash_effect(EffectTiming.OnEnterFieldAnyone, is_when_digivolving=True))
        effects.append(_make_delete_or_trash_effect(EffectTiming.OnEndAttack, is_when_digivolving=False))

        # ── [All Turns] While you have 0 or less memory, this Digimon isn't affected
        #    by the effects of your opponent's Digimon. ─────────────────────────────
        # This is a passive conditional continuous effect registered when the permanent
        # enters the field (OnEnterFieldAnyone, is_when_digivolving=True is NOT correct here —
        # it is a passive that must register once and re-evaluate every query).
        # We register it once on entering field via a NoTiming effect that auto-fires on setup.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect5.is_when_digivolving = True
        effect5.set_effect_name(
            "EX8-073 Not affected by opponent's Digimon's effects while memory <= 0 (register)"
        )
        effect5.set_effect_description(
            "[All Turns] While you have 0 or less memory, this Digimon isn't affected by the "
            "effects of your opponent's Digimon."
        )

        def condition5(context: Dict[str, Any]) -> bool:
            return card.permanent_of_this_card() is not None if card else False
        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            perm = card.permanent_of_this_card() if card else None
            if not (player and game and perm):
                return

            from engine_py_legacy.engine.interfaces.modifiers import ModifierType

            enemy_ref = player.enemy

            def immunity_condition(target_perm, ctx_inner):
                """Active only while this player has 0 or less memory and target is this perm."""
                if target_perm is not perm:
                    return False
                # Memory check
                if not (player and player.memory <= 0):
                    return False
                # Only block effects originating from opponent Digimon
                if not enemy_ref:
                    return False
                source_eff = ctx_inner.get('source_effect') if ctx_inner else None
                if source_eff is None:
                    return False
                for ep in enemy_ref.battle_area:
                    if ep.is_digimon:
                        for cs in ep.card_sources:
                            if source_eff in cs.effect_list(EffectTiming.NoTiming):
                                return True
                return False

            game.register_modifier(
                perm,
                ModifierType.CANNOT_BE_AFFECTED,
                condition=immunity_condition,
                expiry='permanent',
            )

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
