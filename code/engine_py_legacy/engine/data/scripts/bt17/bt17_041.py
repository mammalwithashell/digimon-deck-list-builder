from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_041(CardScript):
    """BT17-041 ShineGreymon: Burst Mode | Lv.7 (Yellow, Vaccine, Light Dragon)

    [Hand] [Counter] <Blast Digivolve> (Your Digimon may digivolve into
        this card without paying the cost.)
    [On Play] [When Digivolving] You may play 1 Tamer card from your hand
        without paying the cost. Then, for the turn, 1 of your opponent's
        Digimon gets -5000 DP for each of your Tamers.
    [When Attacking] By suspending up to 2 of your yellow Tamers, for every
        Tamer this effect suspended, this Digimon gains <Security A. +1>
        for the turn.
    Ace Overflow <-5>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        from ....data.enums import CardColor
        effects = []

        # --- Effect 0: Blast Digivolve ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT17-041 Blast Digivolve")
        effect0.set_effect_description(
            "[Hand] [Counter] <Blast Digivolve>")
        effect0.is_counter_effect = True
        effect0._is_blast_digivolve = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1 & 2: [On Play] [When Digivolving] Play Tamer free, then -5000 DP per Tamer ---
        for is_play in [True, False]:
            effect_n = ICardEffect()
            effect_n.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect_n.set_effect_name("BT17-041 Play Tamer, opponent -5000 DP per Tamer")
            effect_n.set_effect_description(
                "[On Play] [When Digivolving] You may play 1 Tamer card from "
                "your hand without paying the cost. Then, for the turn, 1 of "
                "your opponent's Digimon gets -5000 DP for each of your Tamers."
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

            def make_process():
                def process_n(ctx: Dict[str, Any]):
                    player = ctx.get('player')
                    game = ctx.get('game')
                    if not (player and game):
                        return

                    # Play 1 Tamer from hand without paying cost
                    def tamer_filter(c):
                        return getattr(c, 'is_tamer', False)

                    game.effect_play_from_zone(
                        player, 'hand', tamer_filter, free=True, is_optional=True)

                    # Then, 1 of opponent's Digimon gets -5000 DP for each of your Tamers
                    enemy = player.enemy
                    if not enemy:
                        return

                    tamer_count = sum(1 for p in player.battle_area if p.is_tamer)
                    if tamer_count <= 0:
                        return

                    total_dp_reduction = -5000 * tamer_count

                    opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
                    if opp_digimon:
                        def on_dp_target(target_perm):
                            target_perm.change_dp(total_dp_reduction)

                        game.effect_select_opponent_permanent(
                            player, on_dp_target,
                            filter_fn=lambda p: p.is_digimon,
                            is_optional=False,
                            prompt=f"Select 1 opponent Digimon to get "
                                   f"{total_dp_reduction} DP for the turn.")

                return process_n
            effect_n.set_on_process_callback(make_process())
            effects.append(effect_n)

        # --- Effect 3: [When Attacking] Suspend up to 2 yellow Tamers for SA +1 each ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUseAttack)
        effect3.set_effect_name("BT17-041 When Attacking: Suspend Tamers for SA+1")
        effect3.set_effect_description(
            "[When Attacking] By suspending up to 2 of your yellow Tamers, "
            "for every Tamer this effect suspended, this Digimon gains "
            "<Security A. +1> for the turn."
        )
        effect3.is_optional = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            # Must have unsuspended yellow tamers
            owner = card.owner if card else None
            if not owner:
                return False
            yellow_tamers = [
                p for p in owner.battle_area
                if p.is_tamer and not p.is_suspended
                and p.top_card
                and CardColor.Yellow in (getattr(p.top_card, 'card_colors', []) or [])
            ]
            return len(yellow_tamers) > 0
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            # Find unsuspended yellow tamers
            yellow_tamers = [
                p for p in player.battle_area
                if p.is_tamer and not p.is_suspended
                and p.top_card
                and CardColor.Yellow in (getattr(p.top_card, 'card_colors', []) or [])
            ]

            # Suspend up to 2
            suspended_count = 0
            for tamer in yellow_tamers[:2]:
                tamer.suspend()
                suspended_count += 1

            # Gain SA +1 per suspended tamer
            if suspended_count > 0:
                perm._temp_sa_modifier += suspended_count

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # --- Effect 4: Ace Overflow <-5> ---
        # Overflow is a continuous effect: when this card moves from field/under
        # to another area, lose 5 memory. Handled as inherited effect description.
        # The engine typically handles Overflow via the card data / keyword system.
        # We mark it here for completeness.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnRemovedField)
        effect4.set_effect_name("BT17-041 Ace Overflow <-5>")
        effect4.set_effect_description(
            "Ace Overflow <-5> (As this card moves from the field or under a "
            "card to an area other than those, lose 5 memory.)"
        )
        effect4.is_inherited_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player:
                player.lose_memory(5)
        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
