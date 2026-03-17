from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_032(CardScript):
    """BT23-032 Shakkoumon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-032 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.4 for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Jogress Condition
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-032 Jogress Condition")
        effect1.set_effect_description("Jogress Condition")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Until your opponent's turn ends, give 1 of their Digimon '[Start of Your Main Phase] This Digimon attacks.'. Then, if DNA digivolving, <De-Digivolve 1> 1 of your opponent's Digimon.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT23-032 1 digimon gains [This digimon attacks at start of main phase]. then if DNA digivolved, <De-Digivolve 1>")
        effect2.set_effect_description("[When Digivolving] Until your opponent's turn ends, give 1 of their Digimon '[Start of Your Main Phase] This Digimon attacks.'. Then, if DNA digivolving, <De-Digivolve 1> 1 of your opponent's Digimon.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Give 1 opponent Digimon forced attack; if DNA, de-digivolve 1."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            from ....interfaces.modifiers import ModifierType

            def on_select_force(target_perm):
                # Register FORCE_ATTACK modifier until opponent's turn ends
                # Condition ensures it only matches the specific target permanent
                game.register_modifier(
                    target_perm, ModifierType.FORCE_ATTACK,
                    condition=lambda perm, ctx: perm is target_perm,
                    expiry='end_of_opponent_turn')

                # If DNA digivolving, de-digivolve 1 from an opponent Digimon
                is_dna = ctx.get('is_dna_digivolve', False)
                if is_dna:
                    def on_de_digivolve(dedigivolve_target):
                        removed = dedigivolve_target.de_digivolve(1)
                        enemy = player.enemy if player else None
                        if enemy:
                            enemy.trash_cards.extend(removed)
                    game.effect_select_opponent_permanent(
                        player, on_de_digivolve,
                        filter_fn=lambda p: p.is_digimon,
                        is_optional=False,
                        prompt="Select 1 opponent Digimon to De-Digivolve 1.")

            game.effect_select_opponent_permanent(
                player, on_select_force,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Select 1 opponent Digimon to gain forced attack.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # [On Leaving] Play 1 Lv4- Yellow/Black [CS] trait Digimon from its digivolution cards free
        def _make_leave_effect(is_inherited):
            from ....data.enums import CardColor
            eff = ICardEffect()
            eff.set_timing(EffectTiming.WhenRemoveField)
            eff.set_effect_name("BT23-032 Play Lv4- CS Digimon from digi cards")
            eff.set_effect_description(
                "[On Leaving] You may play 1 level 4 or lower yellow or black Digimon card "
                "with the [CS] trait from its digivolution cards without paying the cost."
            )
            if is_inherited:
                eff.is_inherited_effect = True
            eff.is_optional = True
            eff.set_hash_string("BT23_032_AT" + ("_ESS" if is_inherited else ""))

            def cond(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
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
                    if not (CardColor.Yellow in colors or CardColor.Black in colors):
                        return False
                    traits = c.card_traits or []
                    if not any('CS' in t for t in traits):
                        return False
                    return True

                top = perm.top_card
                candidates = [cs for cs in perm.card_sources
                              if cs is not top and play_filter(cs)]
                if not candidates:
                    return

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
