from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_037(CardScript):
    """BT24-037 Silphymon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Lv.4 with [TS] trait for cost 3
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-037 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4
        effect0._alt_digi_trait = "TS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: jogress
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-037 Jogress Condition")
        effect1.set_effect_description("Jogress Condition")
        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # [On Play] [When Digivolving] 1 of your opponent's Digimon gets -5000 DP
        # for the turn. Then, 1 of your Digimon may attack. If DNA digivolving,
        # 1 of your Digimon gains <Security A. +1> and +5000 DP for the turn.
        for is_play in [True, False]:
            effect_n = ICardEffect()
            effect_n.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect_n.set_effect_name("BT24-037 DP -5000, may attack, DNA bonus")
            effect_n.set_effect_description(
                "[On Play] [When Digivolving] 1 of your opponent's Digimon gets -5000 DP "
                "for the turn. Then, 1 of your Digimon may attack. If DNA digivolving, "
                "1 of your Digimon gains <Security A. +1> and +5000 DP for the turn."
            )
            if is_play:
                effect_n.is_on_play = True
            else:
                effect_n.is_when_digivolving = True

            def make_condition():
                def cond(context: Dict[str, Any]) -> bool:
                    if card and card.permanent_of_this_card() is None:
                        return False
                    return True
                return cond

            effect_n.set_can_use_condition(make_condition())

            def make_process(is_play_flag):
                def process_n(ctx: Dict[str, Any]):
                    player = ctx.get('player')
                    game = ctx.get('game')
                    if not (player and game):
                        return
                    enemy = player.enemy if player else None

                    # 1) DP -5000 to 1 opponent Digimon (player chooses)
                    if enemy and enemy.battle_area:
                        def opp_filter(p):
                            return p.is_digimon
                        def on_dp_target(target_perm):
                            target_perm.change_dp(-5000)
                            game.logger.log(
                                f"[Effect] {game._perm_ref(target_perm)} gets -5000 DP")
                            _after_dp(ctx)
                        game.effect_select_opponent_permanent(
                            player, on_dp_target, filter_fn=opp_filter,
                            is_optional=False,
                            prompt="Select 1 of your opponent's Digimon to give -5000 DP.")
                    else:
                        _after_dp(ctx)

                def _after_dp(ctx):
                    player = ctx.get('player')
                    game = ctx.get('game')
                    if not (player and game):
                        return
                    # 2) "Then, 1 of your Digimon may attack"
                    def own_digi_filter(p):
                        return p.is_digimon
                    def on_attack_target(target_perm):
                        from ....interfaces.modifiers import ModifierType
                        game.register_modifier(
                            target_perm, ModifierType.FORCE_ATTACK,
                            value_fn=lambda: True, expiry='end_of_turn')
                        # 3) If DNA digivolving, grant SA+1 and +5000 DP
                        is_dna = ctx.get('is_dna_digivolve', False)
                        if is_dna:
                            def on_dna_target(dna_perm):
                                dna_perm._temp_sa_modifier += 1
                                dna_perm.change_dp(5000)
                                game.logger.log(
                                    f"[Effect] {game._perm_ref(dna_perm)} gains "
                                    f"<Security A. +1> and +5000 DP (DNA bonus)")
                            game.effect_select_own_permanent(
                                player, on_dna_target, filter_fn=own_digi_filter,
                                is_optional=False,
                                prompt="Select 1 of your Digimon to gain <Security A. +1> "
                                       "and +5000 DP (DNA bonus).")
                    game.effect_select_own_permanent(
                        player, on_attack_target, filter_fn=own_digi_filter,
                        is_optional=True,
                        prompt="Select 1 of your Digimon that may attack.")

                return process_n

            effect_n.set_on_process_callback(make_process(is_play))
            effects.append(effect_n)

        # [All Turns] [Once Per Turn] When this Digimon would leave the battle area
        # other than by your effects, you may play 1 level 4 or lower yellow, red
        # or [TS] trait Digimon card from its digivolution cards without paying the cost.
        def _make_leave_effect(is_inherited):
            eff = ICardEffect()
            eff.set_timing(EffectTiming.WhenRemoveField)
            eff.set_effect_name("BT24-037 Play lv4- yellow/red/TS from digi sources")
            eff.set_effect_description(
                "[All Turns] [Once Per Turn] When this Digimon would leave the battle area "
                "other than by your effects, you may play 1 level 4 or lower yellow, red or "
                "[TS] trait Digimon card from its digivolution cards without paying the cost."
            )
            if is_inherited:
                eff.is_inherited_effect = True
            eff.is_optional = True
            eff.set_max_count_per_turn(1)
            eff.set_hash_string("BT24_037_AT" + ("_ESS" if is_inherited else ""))

            def cond(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                # "other than by your effects" — engine gap: WhenRemoveField context
                # does not carry is_own_effect. Best-effort: allow for 'battle',
                # 'rule', 'de_digivolve' causes; block 'cost' (always own);
                # 'effect' is ambiguous (could be own or opponent).
                cause = context.get('removal_cause', 'effect')
                if cause == 'cost':
                    return False
                return True
            eff.set_can_use_condition(cond)

            def proc(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and perm and game):
                    return

                def play_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    if (c.level or 99) > 4:
                        return False
                    colors = c.card_colors or []
                    traits = c.card_traits or []
                    return (CardColor.Yellow in colors
                            or CardColor.Red in colors
                            or any('TS' in t for t in traits))

                top = perm.top_card
                candidates = [cs for cs in perm.card_sources
                              if cs is not top and play_filter(cs)]
                if not candidates:
                    return

                # Player selects which card to play (instead of auto-selecting)
                if len(candidates) == 1:
                    chosen = candidates[0]
                    perm.card_sources.remove(chosen)
                    played = player.play_card_from_source(chosen, pay_cost=False)
                    if played:
                        game.execute_effects(
                            EffectTiming.OnEnterFieldAnyone,
                            {'played_card': chosen, 'played_permanent': played,
                             'event_player': player},
                        )
                else:
                    # Multiple candidates — let player choose via branch selection
                    labels = [
                        f"{getattr(c, 'card_names', ['?'])[0]} (Lv.{c.level})"
                        for c in candidates
                    ]

                    def on_branch(branch_idx):
                        chosen = candidates[branch_idx]
                        perm.card_sources.remove(chosen)
                        played = player.play_card_from_source(chosen, pay_cost=False)
                        if played:
                            game.execute_effects(
                                EffectTiming.OnEnterFieldAnyone,
                                {'played_card': chosen, 'played_permanent': played,
                                 'event_player': player},
                            )

                    game.effect_choose_branch(
                        player, len(candidates), on_branch,
                        prompt="Select 1 Digimon card from digivolution cards to play.",
                        branch_labels=labels)

            eff.set_on_process_callback(proc)
            return eff

        effects.append(_make_leave_effect(is_inherited=False))
        effects.append(_make_leave_effect(is_inherited=True))

        return effects
