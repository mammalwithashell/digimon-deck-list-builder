from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


def _digi_stack_color_count(card: 'CardSource') -> int:
    """Count unique colors across all cards in the digi-stack."""
    perm = card.permanent_of_this_card() if card else None
    if perm is None:
        return 0
    colors = set()
    for cs in perm.card_sources:
        for c in getattr(cs, 'card_colors', []) or []:
            colors.add(c)
    return len(colors)


def _is_dragon_mode(cs: 'CardSource') -> bool:
    """Check if a CardSource is named Imperialdramon: Dragon Mode."""
    names = getattr(cs, 'card_names', []) or []
    for n in names:
        low = n.lower()
        if ('imperialdramon: dragon mode' in low or
                'imperialdramon:dragonmode' in low or
                'imperialdramon dragon mode' in low or
                'imperialdramondragonmode' in low):
            return True
    return False


class BT12_031(CardScript):
    """BT12-031 Imperialdramon: Fighter Mode | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Alt-Digi: From Imperialdramon: Dragon Mode, cost 2 ──────────
        effect_alt = ICardEffect()
        effect_alt.set_timing(EffectTiming.NoTiming)
        effect_alt.set_effect_name("BT12-031 Alt-digi from Imperialdramon: Dragon Mode")
        effect_alt.set_effect_description(
            "Digivolve: From Imperialdramon: Dragon Mode, Cost 2"
        )
        effect_alt._alt_digi_cost = 2
        effect_alt._alt_digi_name = "Imperialdramon: Dragon Mode"
        effects.append(effect_alt)

        # ── [When Digivolving] ──────────────────────────────────────────
        # Suspend all opponent's Digimon with no digi-cards.
        # Then, return 1 opponent's suspended Digimon to hand.
        # By returning 1 [Imperialdramon: Dragon Mode] from this Digimon's
        # digi-stack to hand, place all opponent's suspended Digimon to
        # deck bottom instead.
        effect_wd = ICardEffect()
        effect_wd.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_wd.set_effect_name("BT12-031 When Digivolving suspend + bounce/deck bottom")
        effect_wd.set_effect_description(
            "[When Digivolving] Suspend all of your opponent's Digimon with no "
            "digivolution cards. Then, return 1 of your opponent's suspended "
            "Digimon to its owner's hand. By returning 1 [Imperialdramon: Dragon "
            "Mode] card from this Digimon's digivolution cards to its owner's "
            "hand, place all of your opponent's suspended Digimon at the bottom "
            "of their owners' decks instead."
        )
        effect_wd.is_when_digivolving = True

        def condition_wd(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect_wd.set_can_use_condition(condition_wd)

        def _bounce_one_suspended(player, game, enemy):
            """Bounce 1 of opponent's suspended Digimon to hand."""
            has_target = any(
                p.is_digimon and p.is_suspended
                for p in enemy.battle_area
            )
            if not has_target:
                return

            def on_bounce(target_perm):
                enemy.bounce_permanent_to_hand(target_perm)

            game.effect_select_opponent_permanent(
                player, on_bounce,
                filter_fn=lambda p: p.is_digimon and p.is_suspended,
                is_optional=False
            )

        def process_wd(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return

            # Step 1: Suspend all opponent Digimon with no digi-cards
            for opp_perm in list(enemy.battle_area):
                if opp_perm.is_digimon and opp_perm.has_no_digivolution_cards:
                    if not opp_perm.is_suspended:
                        opp_perm.suspend()

            # Re-check perm still on field
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return

            # Step 2: Check if we can return an Imperialdramon: Dragon Mode
            # from this Digimon's digi-stack (excluding top card).
            # This is optional (canNoSelect = true in C#).
            dragon_mode_cards = [
                cs for cs in perm.card_sources[:-1]
                if _is_dragon_mode(cs)
            ]

            if dragon_mode_cards:
                # Offer the choice: return Dragon Mode (deck bottom all)
                # or don't return (bounce 1 to hand)
                def on_branch(choice: int):
                    if choice == 0:
                        # Return the first Dragon Mode card from digi-stack to hand
                        dm_card = dragon_mode_cards[0]
                        cur_perm = card.permanent_of_this_card() if card else None
                        if cur_perm and dm_card in cur_perm.card_sources:
                            cur_perm.card_sources.remove(dm_card)
                            player.hand_cards.append(dm_card)

                            # Place ALL opponent's suspended Digimon to deck bottom
                            suspended_opp = [
                                p for p in list(enemy.battle_area)
                                if p.is_digimon and p.is_suspended
                            ]
                            for sp in suspended_opp:
                                if sp in enemy.battle_area:
                                    enemy.return_permanent_to_deck_bottom(sp)
                        else:
                            # Dragon Mode card no longer in stack, fallback to bounce
                            _bounce_one_suspended(player, game, enemy)
                    else:
                        # Don't return Dragon Mode — bounce 1 suspended opp to hand
                        _bounce_one_suspended(player, game, enemy)

                game.effect_choose_branch(
                    player, 2,
                    callback=on_branch,
                    branch_labels=[
                        "Return Dragon Mode to hand (deck bottom all suspended)",
                        "Don't return (bounce 1 suspended to hand)"
                    ]
                )
            else:
                # No Dragon Mode in digi-stack, just bounce 1 suspended opp to hand
                _bounce_one_suspended(player, game, enemy)

        effect_wd.set_on_process_callback(process_wd)
        effects.append(effect_wd)

        # ── Static: +1000 DP per unique color in digi-stack ─────────────
        # The C# uses ChangeSelfDPStaticEffect with a dynamic Func<int>.
        # Python dp_modifier is a fixed int, so we register a CHANGE_DP modifier
        # with a dynamic value_fn on play/digivolve. expiry='permanent' keeps it
        # as long as this Digimon is on field, and value_fn re-evaluates each time.
        for is_digivolve in (True, False):
            effect_dp = ICardEffect()
            effect_dp.set_timing(EffectTiming.OnEnterFieldAnyone)
            suffix = "digivolve" if is_digivolve else "play"
            effect_dp.set_effect_name(
                f"BT12-031 register +1000 DP per digi-stack color ({suffix})"
            )
            effect_dp.set_effect_description(
                "This Digimon gets +1000 DP for each of its digivolution cards' colors."
            )
            if is_digivolve:
                effect_dp.is_when_digivolving = True
            else:
                effect_dp.is_on_play = True

            def make_dp_condition():
                def condition_dp(context: Dict[str, Any]) -> bool:
                    if card and card.permanent_of_this_card() is None:
                        return False
                    return True
                return condition_dp

            effect_dp.set_can_use_condition(make_dp_condition())

            def make_dp_process():
                def process_dp(ctx: Dict[str, Any]):
                    game = ctx.get('game')
                    if not game:
                        return
                    perm = card.permanent_of_this_card() if card else None
                    if perm is None:
                        return
                    from digimon_gym.engine.interfaces.modifiers import ModifierType
                    game.register_modifier(
                        ModifierType.CHANGE_DP, perm,
                        value_fn=lambda: 1000 * _digi_stack_color_count(card),
                        expiry='permanent'
                    )
                return process_dp

            effect_dp.set_on_process_callback(make_dp_process())
            effects.append(effect_dp)

        # ── Static: SA+1 when 2+ colors in digi-stack ──────────────────
        effect_sa = ICardEffect()
        effect_sa.set_timing(EffectTiming.NoTiming)
        effect_sa.set_effect_name("BT12-031 SA+1 when 2+ digi-stack colors")
        effect_sa.set_effect_description(
            "If this Digimon has 2 or more colors among its digivolution cards, "
            "it gains <Security Attack +1>."
        )
        effect_sa.is_declarative = True
        effect_sa._security_attack_modifier = 1

        def condition_sa(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return _digi_stack_color_count(card) >= 2

        effect_sa.set_can_use_condition(condition_sa)
        effects.append(effect_sa)

        # ── Static: Blocker when 2+ colors in digi-stack ────────────────
        effect_blocker = ICardEffect()
        effect_blocker.set_timing(EffectTiming.NoTiming)
        effect_blocker.set_effect_name("BT12-031 Blocker when 2+ digi-stack colors")
        effect_blocker.set_effect_description(
            "If this Digimon has 2 or more colors among its digivolution cards, "
            "it gains <Blocker>."
        )
        effect_blocker.is_declarative = True
        effect_blocker._is_blocker = True

        def condition_blocker(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return _digi_stack_color_count(card) >= 2

        effect_blocker.set_can_use_condition(condition_blocker)
        effects.append(effect_blocker)

        return effects
