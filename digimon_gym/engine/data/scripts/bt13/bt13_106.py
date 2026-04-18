from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_106(CardScript):
    """BT13-106 Odin's Breath | Option | Yellow | Cost 5

    Card text:
      When an effect trashes this card from the security stack, activate this
      card's [Main] effect.
      [Main] 1 of your opponent's Digimon gets -3000 DP until the end of your
      opponent's turn. Then, if there're 6 or fewer total cards in both
      players' security stacks, all of your opponent's Digimon gain
      <Security A. -1> (This Digimon checks 1 fewer security card.) until the
      end of your opponent's turn.

      [Security] Activate this card's [Main] effects.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _activate_main_body(ctx: Dict[str, Any]) -> None:
            """Shared [Main] body used by OptionSkill, OnDiscardSecurity, and
            SecuritySkill dispatch paths. Faithfully implements:

              1. Player chooses 1 opponent Digimon — target gets -3000 DP
                 until end of opponent's turn (target-equality guard to prevent
                 CHANGE_DP leaks across permanents).
              2. Then, if total security in both stacks <= 6, every opponent
                 Digimon gains <Security A. -1> until end of opponent's turn
                 (per-target modifier with equality guard).
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if enemy is None:
                return

            from ....interfaces.modifiers import ModifierType

            # Step 1: Select 1 opponent Digimon to receive -3000 DP.
            def on_dp_target(target_perm):
                if target_perm is None:
                    return
                # Target-equality guard: the modifier must only apply to
                # this specific permanent. Without this guard, CHANGE_DP
                # leaks to every permanent when queried.
                game.register_modifier(
                    target_perm,
                    ModifierType.CHANGE_DP,
                    value_fn=lambda cur, t, c: cur - 3000,
                    condition=lambda t, c, _tp=target_perm: t is _tp,
                    expiry='end_of_opponent_turn',
                )

                # Step 2: Then, conditional SA -1 to all opponent Digimon.
                total_security = (
                    len(player.security_cards) + len(enemy.security_cards)
                )
                if total_security <= 6:
                    for opp_perm in list(enemy.battle_area):
                        if not opp_perm.is_digimon:
                            continue
                        # Each SA -1 modifier is bound to its own target via
                        # an equality guard to prevent cross-target leakage.
                        game.register_modifier(
                            opp_perm,
                            ModifierType.CHANGE_SECURITY_ATTACK,
                            value_fn=lambda cur, t, c: cur - 1,
                            condition=lambda t, c, _tp=opp_perm: t is _tp,
                            expiry='end_of_opponent_turn',
                        )

            has_target = any(
                p.is_digimon for p in enemy.battle_area
            )
            if not has_target:
                return  # No Digimon on opponent's field — effect fizzles
            game.effect_select_opponent_permanent(
                player, on_dp_target,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Select 1 of your opponent's Digimon to get -3000 DP.",
            )

        # ── Effect 0: OnDiscardSecurity — self-trashed-from-security trigger ──
        # When an effect trashes THIS card from the security stack, activate
        # this card's [Main] effect.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDiscardSecurity)
        effect0.set_effect_name("BT13-106 Activate [Main] on self-trash from security")
        effect0.set_effect_description(
            "When an effect trashes this card from the security stack, "
            "activate this card's [Main] effect."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            # Only fires when THIS card is the one being trashed from security.
            # The _collect_triggered_effects pipeline injects extra_context
            # under the key 'discarded_card' (set by trash_security_card).
            discarded = context.get('discarded_card')
            if discarded is not None and discarded is not card:
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_activate_main_body)
        effects.append(effect0)

        # ── Effect 1: OptionSkill — the [Main] effect itself ──────────────────
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("BT13-106 [Main] -3000 DP + conditional SA -1")
        effect1.set_effect_description(
            "[Main] 1 of your opponent's Digimon gets -3000 DP until the end "
            "of your opponent's turn. Then, if there're 6 or fewer total "
            "cards in both players' security stacks, all of your opponent's "
            "Digimon gain <Security A. -1> until the end of your opponent's "
            "turn."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — engine validates use via play gating.
            return True

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_activate_main_body)
        effects.append(effect1)

        # ── Effect 2: SecuritySkill — [Security] Activate this card's [Main] ──
        # Replaces the old cosmetic `is_security_effect=True` factory effect
        # that had no process callback. SecuritySkill dispatch in
        # Player.security_attack() will invoke this process.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT13-106 [Security] Activate [Main]")
        effect2.set_effect_description(
            "[Security] Activate this card's [Main] effects."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_activate_main_body)
        effects.append(effect2)

        return effects
