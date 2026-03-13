from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_060(CardScript):
    """BT21-060 Destromon | Lv.5

    Digivolve: from [Vemmon] for cost 6

    [When Digivolving] Until your opponent's turn ends, their effects can't
    trash this Digimon's top stacked cards. Then, to 1 of your opponent's
    Digimon, <De-Digivolve 1> for every 2 [Vemmon] in this Digimon's
    digivolution cards.

    [All Turns] When this Digimon would leave the battle area, you may play
    1 [Vemmon] from its digivolution cards without paying the cost.

    --- Inherited ---
    [Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon
    attacks, by returning 2 [Vemmon] from this Digimon's digivolution cards
    to the bottom of the deck, end that attack.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_vemmon_name(c) -> bool:
            return any('Vemmon' in n for n in getattr(c, 'card_names', []))

        # ─── Effect 0: Alt digivolve from [Vemmon] for cost 6
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-060 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 6
        effect0._alt_digi_name = "Vemmon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Vemmon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ─── Effect 1: [When Digivolving] Immunity from stack trashing + de-digivolve
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT21-060 Can't trash stacked cards, then de-digivolve")
        effect1.set_effect_description("[When Digivolving] Until your opponent's turn ends, their effects can't trash this Digimon's top stacked cards. Then, to 1 of your opponent's Digimon, <De-Digivolve 1> for every 2 [Vemmon] in this Digimon's digivolution cards.")
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Grant immunity from stack trashing, then de-digivolve opponent."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            # Step 1: Until opponent's turn ends, their effects can't trash digi-cards
            from ....interfaces.modifiers import ModifierType
            game.register_modifier(
                perm, ModifierType.IMMUNE_FROM_STACK_TRASHING,
                value_fn=lambda: True, expiry='end_of_opponent_turn')

            # Step 2: Count [Vemmon] in digivolution cards (exclude top card)
            vemmon_count = sum(
                1 for cs in perm.card_sources[:-1]
                if _is_vemmon_name(cs)
            )
            de_digivolve_amount = vemmon_count // 2
            if de_digivolve_amount <= 0:
                return

            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(de_digivolve_amount)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)

            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ─── Effect 2: [All Turns] When this Digimon would leave the battle area,
        #     play 1 [Vemmon] from its digivolution cards without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenRemoveField)
        effect2.set_effect_name("BT21-060 Play 1 [Vemmon] from digi-cards when leaving")
        effect2.set_effect_description("[All Turns] When this Digimon would leave the battle area, you may play 1 [Vemmon] from its digivolution cards without paying the cost.")
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            # Must have at least 1 [Vemmon] in digi-cards (exclude top card)
            has_vemmon = any(_is_vemmon_name(cs) for cs in perm.card_sources[:-1])
            return has_vemmon

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play 1 [Vemmon] from this Digimon's digivolution cards for free."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            # Find a [Vemmon] in digi-cards (exclude top card)
            qualifying = [
                cs for cs in perm.card_sources[:-1]
                if _is_vemmon_name(cs)
            ]
            if not qualifying:
                return

            chosen = qualifying[0]
            perm.card_sources.remove(chosen)
            played = player.play_card_from_source(chosen, pay_cost=False)
            if played:
                game.execute_effects(EffectTiming.OnEnterFieldAnyone,
                    {"played_card": chosen, "played_permanent": played, "event_player": player})

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # ─── Effect 3 (Inherited): [Opponent's Turn] [Once Per Turn] When opponent's
        #     Digimon attacks, return 2 [Vemmon] from digi-cards to deck bottom to end attack.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUseAttack)
        effect3.set_effect_name("BT21-060 Return 2 [Vemmon] from digi-cards to end attack")
        effect3.set_effect_description("[Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon attacks, by returning 2 [Vemmon] from this Digimon's digivolution cards to the bottom of the deck, end that attack.")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("end-attack-BT21-060")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be opponent's turn
            if card and card.owner and card.owner.is_my_turn:
                return False
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            # Must have at least 2 [Vemmon] in digi-cards (exclude top card)
            vemmon_in_stack = [
                cs for cs in perm.card_sources[:-1]
                if _is_vemmon_name(cs)
            ]
            return len(vemmon_in_stack) >= 2

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Return 2 [Vemmon] to deck bottom, then end the attack."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card()
            if perm is None:
                return

            # Return 2 [Vemmon] from digi-cards to deck bottom
            returned = 0
            for cs in list(perm.card_sources[:-1]):
                if returned >= 2:
                    break
                if _is_vemmon_name(cs):
                    perm.card_sources.remove(cs)
                    player.library_cards.append(cs)
                    # Fire OnDigivolutionCardReturnToDeckBottom timing
                    game.execute_effects(
                        EffectTiming.OnDigivolutionCardReturnToDeckBottom,
                        {"permanent": perm, "returned_card": cs})
                    returned += 1

            # End the attack
            game.force_end_attack()

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
