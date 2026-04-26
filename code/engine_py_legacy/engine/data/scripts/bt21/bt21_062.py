from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_062(CardScript):
    """BT21-062 Galacticmon | Lv.6

    Digivolve: from [Snatchmon] for cost 9

    [When Digivolving] By placing 4 cards with [Vemmon] in their texts from
    your trash as this Digimon's bottom digivolution cards, you may use 1
    [Ragnarok Cannon] from your hand or trash without paying the cost.

    [Start of Your Main Phase] Delete 1 of your opponent's Digimon.

    [All Turns] When this Digimon would leave the battle area, by placing 4
    [Vemmon] from this Digimon's digivolution cards at the bottom of their
    owners' decks, prevent it from leaving play.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _has_vemmon_text(c) -> bool:
            return 'Vemmon' in getattr(c, 'card_text', '')

        def _is_vemmon_name(c) -> bool:
            return any('Vemmon' in n for n in getattr(c, 'card_names', []))

        # ─── Effect 0: Alt digivolve from [Snatchmon] for cost 9
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-062 Alternate digivolution requirement")
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

        # ─── Effect 1: [When Digivolving] Place 4 Vemmon-text from trash as bottom
        #     digi-cards, then use 1 Ragnarok Cannon from hand/trash for free.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT21-062 Place 4 Vemmon-text from trash, use Ragnarok Cannon")
        effect1.set_effect_description("[When Digivolving] By placing 4 cards with [Vemmon] in their texts from your trash as this Digimon's bottom digivolution cards, you may use 1 [Ragnarok Cannon] from your hand or trash without paying the cost.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must have 4+ Vemmon-text cards in trash
            owner = card.owner if card else None
            if not owner:
                return False
            vemmon_text_in_trash = sum(1 for c in owner.trash_cards if _has_vemmon_text(c))
            if vemmon_text_in_trash < 4:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Place 4 Vemmon-text from trash as bottom digi-cards, then use Ragnarok Cannon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            # Step 1: Place 4 cards with [Vemmon] in text from trash as bottom digi-cards
            placed = 0
            for c in list(player.trash_cards):
                if placed >= 4:
                    break
                if _has_vemmon_text(c):
                    player.trash_cards.remove(c)
                    perm.add_card_source_bottom(c)
                    placed += 1

            if placed < 4:
                return  # Couldn't place enough

            # Step 2: Use 1 [Ragnarok Cannon] from hand or trash for free
            def ragnarok_filter(c):
                return any('Ragnarok Cannon' in n for n in getattr(c, 'card_names', []))

            game.effect_play_from_zone(
                player, 'hand_or_trash', ragnarok_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ─── Effect 2: [Start of Your Main Phase] Delete 1 opponent's Digimon.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnStartMainPhase)
        effect2.set_effect_name("BT21-062 Delete 1 opponent Digimon")
        effect2.set_effect_description("[Start of Your Main Phase] Delete 1 of your opponent's Digimon.")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Delete 1 of opponent's Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon

            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # ─── Effect 3: [All Turns] When this Digimon would leave, return 4 [Vemmon]
        #     from digi-cards to deck bottom to prevent leaving.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.WhenRemoveField)
        effect3.set_effect_name("BT21-062 Prevent leaving by returning 4 Vemmon to deck bottom")
        effect3.set_effect_description("[All Turns] When this Digimon would leave the battle area, by placing 4 [Vemmon] from this Digimon's digivolution cards at the bottom of their owners' decks, prevent it from leaving play.")
        effect3.is_optional = True
        effect3.set_hash_string("Substitute_BT21_062")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            # Must have 4+ [Vemmon] (by name) in digi-cards (exclude top card)
            vemmon_in_stack = sum(
                1 for cs in perm.card_sources[:-1]
                if _is_vemmon_name(cs)
            )
            return vemmon_in_stack >= 4

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Return 4 [Vemmon] from digi-cards to deck bottom to prevent leaving."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm):
                return

            returned = 0
            for cs in list(perm.card_sources[:-1]):
                if returned >= 4:
                    break
                if _is_vemmon_name(cs):
                    perm.card_sources.remove(cs)
                    player.library_cards.append(cs)
                    # Fire OnDigivolutionCardReturnToDeckBottom timing
                    if game:
                        game.execute_effects(
                            EffectTiming.OnDigivolutionCardReturnToDeckBottom,
                            {"permanent": perm, "returned_card": cs})
                    returned += 1

            # Prevention of removal is signaled by WhenRemoveField hook processing

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
