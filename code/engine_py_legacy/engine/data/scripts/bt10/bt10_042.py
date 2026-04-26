from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_042(CardScript):
    """BT10-042 Venusmon | Lv.6

    Card text:
      [When Digivolving] All of your opponent's Digimon gain
        <Security A. -1> until the end of your opponent's turn.
      [Opponent's Turn] Your opponent's Digimon with <Security A.> can't
        attack this Digimon and can't activate [When Attacking] and
        [When Digivolving] effects.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ──────────────────────────────────────────────────────────────
        # C1 — [When Digivolving] All opp Digimon gain SA-1 until EoOT
        # ──────────────────────────────────────────────────────────────
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT10-042 Security Attack -1")
        effect0.set_effect_description(
            "[When Digivolving] All of your opponent's Digimon gain "
            "<Security Attack -1> until the end of your opponent's turn. "
            "(This Digimon checks 1 fewer security cards.)"
        )
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Apply SA -1 to every opponent Digimon on the field.

            CRITICAL: each modifier registered MUST have a target-equality
            guard in its condition closure; without it, the engine's
            ``get_int_modifier`` iterates ALL entries of type
            ``CHANGE_SECURITY_ATTACK`` against every permanent and overcounts.
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            from ....interfaces.modifiers import ModifierType
            for opp_perm in list(enemy.battle_area):
                if not opp_perm.is_digimon:
                    continue
                # Target-equality guard: this modifier only applies to
                # ``_target`` — NOT to every permanent queried.
                game.register_modifier(
                    opp_perm,
                    ModifierType.CHANGE_SECURITY_ATTACK,
                    value_fn=lambda cur, t, c: cur - 1,
                    condition=lambda t, c, _target=opp_perm: t is _target,
                    expiry='end_of_opponent_turn',
                )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ──────────────────────────────────────────────────────────────
        # C2 — [Opponent's Turn] Opp Digimon with <Security A.> can't
        # attack THIS Digimon and can't activate [WA] / [WD] effects.
        #
        # Implemented by registering two continuous modifiers on play /
        # when-digivolving:
        #   1. CANNOT_ATTACK_TARGET on Venusmon self — queried by
        #      ``action_mask.build_action_mask`` with
        #      ``{'attacker': attacker}``. Returns True only when the
        #      attacker is an opponent Digimon with any SA change on
        #      opponent's turn.
        #   2. DISABLE_EFFECT (global-style) — queried by
        #      ``_gather_effects`` per-effect with ``{'effect': effect}``.
        #      Returns True only when the queried permanent is an
        #      opponent Digimon with SA changes AND the effect is a
        #      [When Attacking] or [When Digivolving] effect.
        #
        # Both modifiers are registered with the Venusmon permanent as
        # target/source so they clean up automatically when Venusmon
        # leaves the field. ``expiry='permanent'`` ensures they persist
        # while Venusmon is on the field.
        #
        # Both triggers (on-play and when-digivolving) register the same
        # pair of modifiers via a shared factory function.
        # ──────────────────────────────────────────────────────────────

        def _make_static_registration(when_digivolving: bool) -> 'ICardEffect':
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            suffix = "WhenDigivolving" if when_digivolving else "OnPlay"
            effect.set_effect_name(
                f"BT10-042 Opponent's Turn restriction ({suffix})"
            )
            effect.set_effect_description(
                "[Opponent's Turn] Your opponent's Digimon with <Security A.> "
                "can't attack this Digimon and can't activate [When Attacking] "
                "and [When Digivolving] effects."
            )
            if when_digivolving:
                effect.is_when_digivolving = True
            else:
                effect.is_on_play = True

            def static_condition(context: Dict[str, Any]) -> bool:
                return bool(card and card.permanent_of_this_card() is not None)
            effect.set_can_use_condition(static_condition)

            def static_process(ctx: Dict[str, Any]):
                """Register the two continuous restriction modifiers."""
                player = ctx.get('player')
                game = ctx.get('game')
                perm = ctx.get('permanent')
                if not (player and game and perm):
                    return
                from ....interfaces.modifiers import ModifierType

                _venusmon_perm = perm
                _venusmon_owner = player

                def _is_opp_turn() -> bool:
                    # "Opponent's turn" = NOT Venusmon controller's turn.
                    return not _venusmon_owner.is_my_turn

                def _venusmon_on_field() -> bool:
                    return (
                        _venusmon_perm in _venusmon_owner.battle_area
                        and _venusmon_perm.top_card is not None
                    )

                def _has_sa_changes(target_perm) -> bool:
                    """True iff this permanent has any Security Attack change.

                    Mirrors DCGO ``HasSecurityAttackChanges``:
                      ``SecurityAttackChanges.Count >= 1``.

                    We treat ANY nonzero computed SA modifier (from either the
                    modifier registry or an inherent ``_security_attack_modifier``
                    on a cached effect) as "has changes".
                    """
                    if target_perm is None or not target_perm.is_digimon:
                        return False
                    try:
                        return target_perm.security_attack_modifier() != 0
                    except Exception:
                        return False

                # ── Modifier 1: CANNOT_ATTACK_TARGET on Venusmon self ──
                # Condition: only fires when (a) target in the query is
                # Venusmon itself, (b) the attacker (from ctx) is an
                # opponent Digimon with SA changes, (c) it's opp's turn,
                # (d) Venusmon is still on the field.
                def cannot_attack_cond(target, q_ctx, _vp=_venusmon_perm,
                                       _owner=_venusmon_owner):
                    if target is not _vp:
                        return False
                    if not _venusmon_on_field():
                        return False
                    if _owner.is_my_turn:
                        return False
                    attacker = q_ctx.get('attacker') if q_ctx else None
                    if attacker is None:
                        return False
                    if attacker is _vp:
                        return False
                    enemy = _owner.enemy
                    if enemy is None or attacker not in enemy.battle_area:
                        return False
                    if not attacker.is_digimon:
                        return False
                    return _has_sa_changes(attacker)

                game.register_modifier(
                    _venusmon_perm,
                    ModifierType.CANNOT_ATTACK_TARGET,
                    condition=cannot_attack_cond,
                    source_effect=effect,
                    expiry='permanent',
                )

                # ── Modifier 2: DISABLE_EFFECT (WA/WD only) ──
                # Registered on Venusmon self for cleanup tie-in, but the
                # condition inspects the `target` perm passed in the query
                # (which is the effect-source permanent, NOT Venusmon).
                def disable_effect_cond(target, q_ctx, _vp=_venusmon_perm,
                                        _owner=_venusmon_owner):
                    if not _venusmon_on_field():
                        return False
                    if _owner.is_my_turn:
                        return False
                    if target is None or target is _vp:
                        return False
                    enemy = _owner.enemy
                    if enemy is None or target not in enemy.battle_area:
                        return False
                    if not target.is_digimon:
                        return False
                    if not _has_sa_changes(target):
                        return False
                    effect_obj = q_ctx.get('effect') if q_ctx else None
                    if effect_obj is None:
                        return False
                    return bool(
                        getattr(effect_obj, 'is_when_digivolving', False)
                        or getattr(effect_obj, 'is_on_attack', False)
                    )

                game.register_modifier(
                    _venusmon_perm,
                    ModifierType.DISABLE_EFFECT,
                    condition=disable_effect_cond,
                    source_effect=effect,
                    expiry='permanent',
                )

            effect.set_on_process_callback(static_process)
            return effect

        effects.append(_make_static_registration(when_digivolving=False))
        effects.append(_make_static_registration(when_digivolving=True))

        return effects
