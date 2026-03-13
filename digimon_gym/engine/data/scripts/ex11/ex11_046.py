from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_046(CardScript):
    """EX11-046 Galacticmon | Lv.6

    Digivolve: from [Snatchmon] for cost 9
    Digivolve: from [Galacticmon] for cost 5
    Assembly: 8 cards with [Vemmon] in text, reduce cost by 6

    [On Play] [When Digivolving] Choose 1 of your opponent's highest play cost
    Digimon and delete all of their other Digimon. Then, if this Digimon has 4
    or more [Vemmon] in its digivolution cards, until your opponent's turn ends,
    it gains <Blocker> and isn't affected by their effects.

    [End of Opponent's Turn] This Digimon may digivolve into [Galacticmon] in
    the hand or trash, ignoring digivolution requirements and without paying
    the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _count_vemmon_in_digi_cards(perm) -> int:
            """Count [Vemmon] in digivolution cards (exclude top card)."""
            if perm is None or len(perm.card_sources) <= 1:
                return 0
            return sum(
                1 for cs in perm.card_sources[:-1]
                if any('Vemmon' in n for n in getattr(cs, 'card_names', []))
            )

        # ─── Effect 0: Alt digivolve from [Snatchmon] for cost 9
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-046 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 9
        effect0._alt_digi_name = "Snatchmon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Snatchmon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ─── Effect 1: Alt digivolve from [Galacticmon] for cost 5
        effect1 = ICardEffect()
        effect1.set_effect_name("EX11-046 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        effect1._alt_digi_cost = 5
        effect1._alt_digi_name = "Galacticmon"

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Galacticmon'))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # ─── Shared On Play / When Digivolving logic
        def _shared_process(ctx: Dict[str, Any]):
            """Choose 1 highest-cost opponent Digimon, delete all others.
            Then if 4+ Vemmon in digi-cards, gain Blocker + immunity until
            opponent's turn ends."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Step 1: Find opponent's highest play cost Digimon
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if not opp_digimon:
                return

            max_cost = max(
                (p.top_card.get_cost_itself if p.top_card else 0)
                for p in opp_digimon
            )
            highest_cost_digimon = [
                p for p in opp_digimon
                if (p.top_card.get_cost_itself if p.top_card else 0) == max_cost
            ]

            # If there are multiple Digimon tied for highest cost, player chooses 1
            # to KEEP; all others get deleted.
            def highest_filter(p):
                return p.is_digimon and p in highest_cost_digimon

            def on_chosen(chosen_perm):
                # Delete all OTHER opponent Digimon (not the chosen one)
                to_delete = [
                    p for p in list(enemy.battle_area)
                    if p.is_digimon and p is not chosen_perm
                ]
                for target in to_delete:
                    enemy.delete_permanent(target)

            if len(highest_cost_digimon) == 1:
                # Only one at highest cost — auto-choose it, delete all others
                on_chosen(highest_cost_digimon[0])
            else:
                # Multiple tied for highest — player selects which to keep
                game.effect_select_opponent_permanent(
                    player, on_chosen, filter_fn=highest_filter, is_optional=False)

            # Step 2: If 4+ Vemmon in digi-cards, gain Blocker + immunity
            if perm and _count_vemmon_in_digi_cards(perm) >= 4:
                from ....interfaces.modifiers import ModifierType
                game.register_modifier(
                    perm, ModifierType.GRANT_BLOCKER,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')
                game.register_modifier(
                    perm, ModifierType.CANNOT_BE_AFFECTED,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')

        # ─── Effect 2: [On Play]
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX11-046 Choose 1 highest cost, delete rest, conditional Blocker+immunity")
        effect2.set_effect_description("[On Play] Choose 1 of your opponent's highest play cost Digimon and delete all of their other Digimon. Then, if this Digimon has 4 or more [Vemmon] in its digivolution cards, until your opponent's turn ends, it gains <Blocker> and isn't affected by their effects.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_shared_process)
        effects.append(effect2)

        # ─── Effect 3: [When Digivolving]
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX11-046 Choose 1 highest cost, delete rest, conditional Blocker+immunity")
        effect3.set_effect_description("[When Digivolving] Choose 1 of your opponent's highest play cost Digimon and delete all of their other Digimon. Then, if this Digimon has 4 or more [Vemmon] in its digivolution cards, until your opponent's turn ends, it gains <Blocker> and isn't affected by their effects.")
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_shared_process)
        effects.append(effect3)

        # ─── Effect 4: [End of Opponent's Turn] Digivolve into [Galacticmon]
        #     from hand or trash, ignoring requirements, without paying cost.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEndTurn)
        effect4.set_effect_name("EX11-046 Digivolve into Galacticmon from hand or trash")
        effect4.set_effect_description("[End of Opponent's Turn] This Digimon may digivolve into [Galacticmon] in the hand or trash, ignoring digivolution requirements and without paying the cost.")
        effect4.is_optional = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be opponent's turn
            if card and card.owner and card.owner.is_my_turn:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Digivolve into [Galacticmon] from hand or trash, free, ignore requirements."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def galacticmon_filter(c):
                return any('Galacticmon' in n for n in getattr(c, 'card_names', []))

            # Check hand first
            hand_candidates = [c for c in player.hand_cards if galacticmon_filter(c)]
            # Check trash
            trash_candidates = [c for c in player.trash_cards if galacticmon_filter(c)]

            if hand_candidates:
                # Digivolve from hand, free, ignore requirements
                game.effect_digivolve_from_hand(
                    player, perm, galacticmon_filter,
                    cost_override=0, ignore_requirements=True, is_optional=True)
            elif trash_candidates:
                # Digivolve from trash — move card to hand first, then digivolve
                chosen = trash_candidates[0]
                player.trash_cards.remove(chosen)
                player.hand_cards.append(chosen)
                game.effect_digivolve_from_hand(
                    player, perm, galacticmon_filter,
                    cost_override=0, ignore_requirements=True, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
