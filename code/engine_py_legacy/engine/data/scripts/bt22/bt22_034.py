from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_034(CardScript):
    """BT22-034 Reppamon | Lv.4 Yellow | Holy Beast/CS | 5000 DP | Cost 5

    When effects trash this card from the security stack, you may play this
        card without paying the cost.
    [On Play] [When Digivolving] 1 of your opponent's Digimon gets -3000 DP
        until their turn ends. By trashing your top security card, instead 1
        of your opponent's Digimon gets -6000 DP until their turn ends.

    Inherited:
    [When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -2000
        DP for the turn.

    Alt digivolve: Lv.3 w/ [CS] trait for cost 2.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Effect 0: Alt-digivolve (Lv.3 [CS] trait for cost 2) ────
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-034 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3
        effect0._alt_digi_trait = "CS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ── Effect 1: OnDiscardSecurity self-trash → may play free ──
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDiscardSecurity)
        effect1.set_effect_name("BT22-034 Self-trash from security: play free")
        effect1.set_effect_description(
            "When effects trash this card from the security stack, you may "
            "play this card without paying the cost."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Only fires when THIS specific card is the one being discarded
            # from security and is now sitting in trash.
            discarded = context.get('discarded_card')
            if discarded is not None and discarded is not card:
                return False
            owner = card.owner if card else None
            if owner is None:
                return False
            if card not in owner.trash_cards:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Play THIS card from trash without paying its cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and card):
                return
            if card not in player.trash_cards:
                return
            # Remove from trash and play for free
            player.trash_cards.remove(card)
            played_perm = player.play_card_from_source(card, pay_cost=False)
            if played_perm is None:
                return
            # Fire OnEnterFieldAnyone so Reppamon's own [On Play] effect runs
            game.execute_effects(
                EffectTiming.OnEnterFieldAnyone,
                {
                    "played_card": card,
                    "played_permanent": played_perm,
                    "event_permanent": played_perm,
                    "event_player": player,
                },
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ── Shared [On Play] / [When Digivolving] factory ──────────
        def make_dp_debuff_effect(when_digivolving: bool) -> ICardEffect:
            eff = ICardEffect()
            eff.set_timing(EffectTiming.OnEnterFieldAnyone)
            trigger_tag = "WD" if when_digivolving else "OP"
            eff.set_effect_name(
                f"BT22-034 [{trigger_tag}] -3K DP (or trash top sec for -6K)"
            )
            eff.set_effect_description(
                ("[When Digivolving] " if when_digivolving else "[On Play] ")
                + "1 of your opponent's Digimon gets -3000 DP until their "
                "turn ends. By trashing your top security card, instead 1 "
                "of your opponent's Digimon gets -6000 DP until their turn "
                "ends."
            )
            if when_digivolving:
                eff.is_when_digivolving = True
            else:
                eff.is_on_play = True

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True
            eff.set_can_use_condition(condition)

            def apply_dp_debuff(game, player, dp_amount: int):
                """Select an opponent Digimon and apply -dp_amount until
                end of opponent's turn, with a target-equality guard so the
                modifier only affects the selected permanent."""
                from ....interfaces.modifiers import ModifierType

                enemy = player.enemy if player else None
                if not enemy:
                    return
                if not any(p.is_digimon for p in enemy.battle_area):
                    return

                def on_target(target_perm):
                    if target_perm is None:
                        return
                    _tp = target_perm

                    def value_fn(cur, t, c, _amount=dp_amount):
                        return cur - _amount

                    def target_equality(t, c, _tp=_tp):
                        return t is _tp

                    game.register_modifier(
                        _tp,
                        ModifierType.CHANGE_DP,
                        condition=target_equality,
                        value_fn=value_fn,
                        expiry='end_of_opponent_turn',
                    )

                game.effect_select_opponent_permanent(
                    player, on_target,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=False,
                    prompt=f"Select 1 of your opponent's Digimon to give -{dp_amount} DP.",
                )

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                has_security = bool(player.security_cards)

                def default_branch():
                    apply_dp_debuff(game, player, 3000)

                def trash_sec_branch():
                    # Trash your own top security card, then -6000 DP
                    if player.security_cards:
                        top_sec = player.security_cards[0]
                        player.trash_security_card(top_sec)
                    apply_dp_debuff(game, player, 6000)

                if not has_security:
                    # No choice possible — only default branch applies
                    default_branch()
                    return

                def on_branch(choice: int):
                    if choice == 1:
                        trash_sec_branch()
                    else:
                        default_branch()

                game.effect_choose_branch(
                    player, 2, on_branch,
                    prompt="Trash top security for -6000 DP instead of -3000 DP?",
                    branch_labels=["-3000 DP", "Trash top security, -6000 DP"],
                )

            eff.set_on_process_callback(process)
            return eff

        effects.append(make_dp_debuff_effect(when_digivolving=False))
        effects.append(make_dp_debuff_effect(when_digivolving=True))

        # ── Effect 4: Inherited [When Attacking] OPT -2000 DP ───────
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUseAttack)
        effect4.set_effect_name("BT22-034 Inherited [WA] -2K DP OPT")
        effect4.set_effect_description(
            "[When Attacking] [Once Per Turn] 1 of your opponent's Digimon "
            "gets -2000 DP for the turn."
        )
        effect4.is_inherited_effect = True
        effect4.is_on_attack = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT22_034_WA")

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Engine restricts OnUseAttack to the attacking permanent already.
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            from ....interfaces.modifiers import ModifierType
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            if not any(p.is_digimon for p in enemy.battle_area):
                return

            def on_target(target_perm):
                if target_perm is None:
                    return
                _tp = target_perm

                def value_fn(cur, t, c):
                    return cur - 2000

                def target_equality(t, c, _tp=_tp):
                    return t is _tp

                game.register_modifier(
                    _tp,
                    ModifierType.CHANGE_DP,
                    condition=target_equality,
                    value_fn=value_fn,
                    expiry='end_of_turn',
                )

            game.effect_select_opponent_permanent(
                player, on_target,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Select 1 of your opponent's Digimon to give -2000 DP.",
            )

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
