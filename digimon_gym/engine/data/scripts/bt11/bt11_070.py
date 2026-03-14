from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_070(CardScript):
    """BT11-070 Destromon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-070 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 6
        effect0._alt_digi_cost = 6
        effect0._alt_digi_name = "Vemmon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Reveal the top 3 cards of your deck. Place 1 [Vemmon] among them under this Digimon as its bottom digivolution card. Trash the rest. Then, if there are 5 or more [Vemmon] in this Digimon's digivolution cards, delete 1 of your opponent's Tamers.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT11-070 Reveal the top 3 cards of deck and delete 1 Tamer")
        effect1.set_effect_description("[When Digivolving] Reveal the top 3 cards of your deck. Place 1 [Vemmon] among them under this Digimon as its bottom digivolution card. Trash the rest. Then, if there are 5 or more [Vemmon] in this Digimon's digivolution cards, delete 1 of your opponent's Tamers.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """[When Digivolving] Reveal top 3, place 1 [Vemmon] as bottom digi-card, trash rest.
            Then if 5+ [Vemmon] in digi-cards, delete 1 opponent Tamer."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            # Step 1: Reveal top 3, place 1 [Vemmon] as bottom digi-card, trash the rest
            def vemmon_filter(c):
                return any('Vemmon' in n for n in getattr(c, 'card_names', []))

            def on_revealed(selected, remaining):
                # Place the selected [Vemmon] as this Digimon's bottom digi-card
                perm.add_card_source_bottom(selected)
                # Trash the rest (per C#: remainingCardsPlace: Trash)
                for c in remaining:
                    player.trash_cards.append(c)

                # Step 2: If 5+ [Vemmon] in digi-cards, delete 1 opponent Tamer
                vemmon_count = sum(
                    1 for cs in perm.card_sources[:-1]
                    if any('Vemmon' in n for n in getattr(cs, 'card_names', []))
                )
                if vemmon_count >= 5:
                    enemy = player.enemy if player else None
                    if enemy:
                        def tamer_filter(p):
                            return getattr(p, 'is_tamer', False)
                        def on_delete(target_perm):
                            enemy.delete_permanent(target_perm)
                        opp_tamers = [p for p in enemy.battle_area if tamer_filter(p)]
                        if opp_tamers:
                            game.effect_select_opponent_permanent(
                                player, on_delete, filter_fn=tamer_filter, is_optional=False)

            game.effect_reveal_and_select(
                player, 3, vemmon_filter, on_revealed, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnUseAttack
        # [Opponent's Turn][Once Per Turn] When an opponent's Digimon attacks, by placing 2 [Vemmon] from 1 of your [Galacticmon]'s digivolution cards at the bottom of their owners' decks, switch the target of attack to this Digimon.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("BT11-070 Return 2 [Vemmon] from [Galacticmon]'s digivolution cards to the deck bottom to switch attack target to this Digimon")
        effect2.set_effect_description("[Opponent's Turn][Once Per Turn] When an opponent's Digimon attacks, by placing 2 [Vemmon] from 1 of your [Galacticmon]'s digivolution cards at the bottom of their owners' decks, switch the target of attack to this Digimon.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("SwitchAttackTarget_BT11_070")
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be opponent's turn
            if card and card.owner and card.owner.is_my_turn:
                return False
            # Must have a [Galacticmon] on our field with 2+ [Vemmon] in its digi-cards
            owner = card.owner if card else None
            if not owner:
                return False
            has_valid_galacticmon = False
            for p in owner.battle_area:
                if p.is_digimon and p.contains_card_name('Galacticmon'):
                    vemmon_in_stack = sum(
                        1 for cs in p.card_sources[:-1]
                        if any('Vemmon' in n for n in getattr(cs, 'card_names', []))
                    )
                    if vemmon_in_stack >= 2:
                        has_valid_galacticmon = True
                        break
            return has_valid_galacticmon

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Return 2 [Vemmon] from 1 of your [Galacticmon]'s digi-cards to deck bottom,
            then switch attack target to this Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            my_perm = card.permanent_of_this_card() if card else None
            if not my_perm:
                return

            # Find a [Galacticmon] on our field with 2+ [Vemmon] in digi-cards
            galacticmon_perm = None
            for p in player.battle_area:
                if p.is_digimon and p.contains_card_name('Galacticmon'):
                    vemmon_in_stack = [
                        cs for cs in p.card_sources[:-1]
                        if any('Vemmon' in n for n in getattr(cs, 'card_names', []))
                    ]
                    if len(vemmon_in_stack) >= 2:
                        galacticmon_perm = p
                        break

            if not galacticmon_perm:
                return

            # Return 2 [Vemmon] from that Galacticmon's digi-cards to deck bottom
            returned = 0
            for cs in list(galacticmon_perm.card_sources[:-1]):
                if returned >= 2:
                    break
                if any('Vemmon' in n for n in getattr(cs, 'card_names', [])):
                    galacticmon_perm.card_sources.remove(cs)
                    player.library_cards.append(cs)
                    # Fire OnDigivolutionCardReturnToDeckBottom timing
                    game.execute_effects(
                        EffectTiming.OnDigivolutionCardReturnToDeckBottom,
                        {"permanent": galacticmon_perm, "returned_card": cs})
                    returned += 1

            # Switch the attack target to this Digimon
            if returned == 2:
                game.switch_attack_target(my_perm)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
