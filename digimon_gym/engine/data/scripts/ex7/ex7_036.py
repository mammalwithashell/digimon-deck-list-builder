from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX7_036(CardScript):
    """EX7-036 Zephagamon | Lv.6, Green, Magic Knight/Vortex Warriors/LIBERATOR

    <Security A. +1> <Vortex>
    [When Digivolving] [When Attacking] Suspend 1 Digimon. If this effect
        suspended your Digimon, return 1 of your opponent's suspended Digimon
        to the bottom of the deck.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Keyword: <Security Attack +1> ---
        effect_sa = ICardEffect()
        effect_sa.set_effect_name("EX7-036 Security Attack +1")
        effect_sa.set_effect_description("<Security A. +1>")
        effect_sa._security_attack_modifier = 1

        def cond_sa(context: Dict[str, Any]) -> bool:
            return True
        effect_sa.set_can_use_condition(cond_sa)
        effects.append(effect_sa)

        # --- Keyword: <Vortex> ---
        effect_vortex = ICardEffect()
        effect_vortex.set_effect_name("EX7-036 Vortex")
        effect_vortex.set_effect_description("<Vortex>")
        effect_vortex._is_vortex = True

        def cond_vortex(context: Dict[str, Any]) -> bool:
            return True
        effect_vortex.set_can_use_condition(cond_vortex)
        effects.append(effect_vortex)

        # --- Shared process: Suspend 1 Digimon; if own => return 1 opponent's
        #     suspended Digimon to deck bottom ---
        def _make_suspend_and_bounce(is_when_digivolving: bool, is_on_attack: bool):
            effect = ICardEffect()
            effect.set_timing(
                EffectTiming.OnEnterFieldAnyone if is_when_digivolving else EffectTiming.OnUseAttack
            )
            tag = "When Digivolving" if is_when_digivolving else "When Attacking"
            effect.set_effect_name(f"EX7-036 Suspend + bounce opp suspended [{tag}]")
            effect.set_effect_description(
                f"[{tag}] Suspend 1 Digimon. If this effect suspended your Digimon, "
                "return 1 of your opponent's suspended Digimon to the bottom of the deck."
            )
            if is_when_digivolving:
                effect.is_when_digivolving = True
            if is_on_attack:
                effect.is_on_attack = True

            def cond(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True
            effect.set_can_use_condition(cond)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                suspended_own = [False]

                def _try_return_opponent_suspended():
                    if not suspended_own[0]:
                        return
                    # Return 1 of opponent's suspended Digimon to deck bottom
                    def on_return(target_perm):
                        if player.enemy:
                            player.enemy.return_permanent_to_deck_bottom(target_perm)

                    opp_suspended = [
                        p for p in (player.enemy.battle_area if player.enemy else [])
                        if p.is_digimon and p.is_suspended
                    ]
                    if opp_suspended:
                        game.effect_select_opponent_permanent(
                            player,
                            on_return,
                            filter_fn=lambda p: p.is_digimon and p.is_suspended,
                            is_optional=False,
                            prompt="Return 1 of your opponent's suspended Digimon to the bottom of the deck.",
                        )

                def on_own_suspended(target_perm):
                    suspended_own[0] = True
                    target_perm.suspend()
                    _try_return_opponent_suspended()

                # Must suspend 1 Digimon (mandatory, not optional)
                own_suspendable = [
                    p for p in player.battle_area
                    if p.is_digimon and not p.is_suspended
                ]
                opp_suspendable = [
                    p for p in player.enemy.battle_area
                    if p.is_digimon and not p.is_suspended
                ] if player.enemy else []

                if own_suspendable:
                    # Offer own Digimon selection (mandatory) — suspending own triggers bonus
                    game.effect_select_own_permanent(
                        player,
                        on_own_suspended,
                        filter_fn=lambda p: p.is_digimon and not p.is_suspended,
                        is_optional=False,
                        prompt="Suspend 1 of your Digimon (triggers return of opponent's suspended Digimon).",
                    )
                elif opp_suspendable:
                    # No own Digimon to suspend; suspend opponent's (no bonus triggered)
                    game.effect_select_opponent_permanent(
                        player,
                        lambda p: p.suspend(),
                        filter_fn=lambda p: p.is_digimon and not p.is_suspended,
                        is_optional=False,
                        prompt="Suspend 1 of your opponent's Digimon.",
                    )

            effect.set_on_process_callback(process)
            return effect

        effects.append(_make_suspend_and_bounce(is_when_digivolving=True, is_on_attack=False))
        effects.append(_make_suspend_and_bounce(is_when_digivolving=False, is_on_attack=True))

        return effects
