from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST18_10(CardScript):
    """ST18-10 GrandGalemon | Lv.5, Green, Bird Dragon/Vortex Warriors/LIBERATOR

    [On Play] [When Digivolving] Suspend 1 Digimon. If this effect suspends
        your Digimon, you may play 1 Digimon card with [Bird] or [Avian] in
        any of its traits with 3000 DP or less from your hand without paying
        the cost.
    [Inherited] [Your Turn] [Once Per Turn] When this Digimon attacks your
        opponent's Digimon, you may unsuspend this Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Shared helper: suspend 1 Digimon; if own => play Bird/Avian <=3000DP ---
        def _make_suspend_and_play(is_on_play: bool):
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            tag = "On Play" if is_on_play else "When Digivolving"
            effect.set_effect_name(f"ST18-10 Suspend + play Bird [{tag}]")
            effect.set_effect_description(
                f"[{tag}] Suspend 1 Digimon. If this effect suspends your Digimon, "
                "you may play 1 Digimon card with [Bird] or [Avian] in any of its "
                "traits with 3000 DP or less from your hand without paying the cost."
            )
            if is_on_play:
                effect.is_on_play = True
            else:
                effect.is_when_digivolving = True

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

                def _try_play_from_hand():
                    def bird_filter(c):
                        if not getattr(c, 'is_digimon', False):
                            return False
                        dp = getattr(c, 'base_dp', None) or 0
                        if dp > 3000:
                            return False
                        traits = getattr(c, 'card_traits', []) or []
                        return any('Bird' in t or 'Avian' in t for t in traits)
                    game.effect_play_from_zone(
                        player, 'hand', bird_filter,
                        free=True, is_optional=True,
                        prompt="You may play 1 Digimon with [Bird]/[Avian] and 3000 DP or less for free.",
                    )

                def on_own_suspended(target_perm):
                    suspended_own[0] = True
                    target_perm.suspend()
                    _try_play_from_hand()

                # Prefer own Digimon (triggers the play bonus); fall back to opponent
                own_suspendable = [
                    p for p in player.battle_area
                    if p.is_digimon and not p.is_suspended
                ]
                opp_suspendable = [
                    p for p in player.enemy.battle_area
                    if p.is_digimon and not p.is_suspended
                ] if player.enemy else []

                if own_suspendable:
                    game.effect_select_own_permanent(
                        player,
                        on_own_suspended,
                        filter_fn=lambda p: p.is_digimon and not p.is_suspended,
                        is_optional=False,
                        prompt="Suspend 1 of your Digimon (enables play from hand).",
                    )
                elif opp_suspendable:
                    game.effect_select_opponent_permanent(
                        player,
                        lambda p: p.suspend(),
                        filter_fn=lambda p: p.is_digimon and not p.is_suspended,
                        is_optional=False,
                        prompt="Suspend 1 of your opponent's Digimon.",
                    )

            effect.set_on_process_callback(process)
            return effect

        effects.append(_make_suspend_and_play(is_on_play=True))
        effects.append(_make_suspend_and_play(is_on_play=False))

        # --- [Inherited] [Your Turn] [Once Per Turn] When this Digimon attacks
        #     opponent's Digimon, you may unsuspend this Digimon ---
        effect_inh = ICardEffect()
        effect_inh.set_timing(EffectTiming.OnUseAttack)
        effect_inh.set_effect_name("ST18-10 Unsuspend when attacking opponent's Digimon (Inherited)")
        effect_inh.set_effect_description(
            "[Your Turn] [Once Per Turn] When this Digimon attacks your "
            "opponent's Digimon, you may unsuspend this Digimon."
        )
        effect_inh.is_inherited_effect = True
        effect_inh.is_on_attack = True
        effect_inh.is_optional = True
        effect_inh.set_max_count_per_turn(1)
        effect_inh.set_hash_string("ST18_10_inh_unsuspend")

        def cond_inh(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not (owner and owner.is_my_turn):
                return False
            perm_now = card.permanent_of_this_card()
            if perm_now is None:
                return False
            game_obj = owner.game if owner else None
            if game_obj and game_obj.pending_attack:
                pa = game_obj.pending_attack
                if pa.attacker is not perm_now:
                    return False
                target = pa.original_target
                from digimon_gym.engine.core.player import Player as _Player
                if isinstance(target, _Player):
                    return False
                if not getattr(target, 'is_digimon', False):
                    return False
            return True
        effect_inh.set_can_use_condition(cond_inh)

        def process_inh(ctx: Dict[str, Any]):
            perm_now = card.permanent_of_this_card() if card else None
            if perm_now:
                perm_now.unsuspend()
        effect_inh.set_on_process_callback(process_inh)
        effects.append(effect_inh)

        return effects
