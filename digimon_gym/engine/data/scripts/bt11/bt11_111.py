from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_111(CardScript):
    """BT11-111 Galacticmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_vemmon_name(c) -> bool:
            return any('Vemmon' in n for n in getattr(c, 'card_names', []))

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-111 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 9
        effect0._alt_digi_cost = 9
        effect0._alt_digi_name = "Snatchmon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ─── Effect 1: [When Digivolving] You may place up to 4 [Vemmon] from
        # your trash under this Digimon as its bottom digivolution cards. Then,
        # if there are 8 or more [Vemmon] in this Digimon's digivolution cards,
        # delete 1 of your opponent's Digimon.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT11-111 Place up to 4 [Vemmon] from trash to digivolution cards and delete 1 Digimon")
        effect1.set_effect_description(
            "[When Digivolving] You may place up to 4 [Vemmon] from your trash "
            "under this Digimon as its bottom digivolution cards. Then, if there "
            "are 8 or more [Vemmon] in this Digimon's digivolution cards, delete "
            "1 of your opponent's Digimon.")
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """[When Digivolving] Place up to 4 Vemmon from trash, then if 8+ Vemmon in digi-cards, delete 1 opponent Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            # Place up to 4 [Vemmon] from trash as bottom digi-cards
            placed = 0
            for c in list(player.trash_cards):
                if placed >= 4:
                    break
                if _is_vemmon_name(c):
                    player.trash_cards.remove(c)
                    perm.add_card_source_bottom(c)
                    placed += 1

            # Then, if 8+ [Vemmon] in digi-cards (excluding top card)
            vemmon_count = sum(
                1 for cs in perm.card_sources[:-1]
                if any('Vemmon' in n for n in getattr(cs, 'card_names', []))
            )
            if vemmon_count >= 8:
                # Delete 1 of opponent's Digimon
                enemy = player.enemy if player else None
                if enemy:
                    def target_filter(p):
                        return p.is_digimon
                    def on_delete(target_perm):
                        enemy.delete_permanent(target_perm)
                    game.effect_select_opponent_permanent(
                        player, on_delete, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ─── Effect 2: [All Turns] When this Digimon would leave the battle
        # area, by placing 4 [Vemmon] from this Digimon's digivolution cards at
        # the bottom of their owners' decks, prevent it from leaving play.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenRemoveField)
        effect2.set_effect_name("BT11-111 Prevent this Digimon from leaving play")
        effect2.set_effect_description(
            "[All Turns] When this Digimon would leave the battle area, by "
            "placing 4 [Vemmon] from this Digimon's digivolution cards at the "
            "bottom of their owners' decks, prevent it from leaving play.")
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be THIS Digimon leaving
            leaving_perm = context.get('permanent')
            my_perm = card.permanent_of_this_card()
            if leaving_perm is not my_perm:
                return False
            # Must have at least 4 [Vemmon] in digi-cards
            if my_perm is None:
                return False
            vemmon_in_stack = [
                cs for cs in my_perm.card_sources[:-1]
                if any('Vemmon' in n for n in getattr(cs, 'card_names', []))
            ]
            return len(vemmon_in_stack) >= 4

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Place 4 [Vemmon] from digi-cards to deck bottom to prevent leaving."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm):
                return

            # Find [Vemmon] in digi-stack (excluding top card)
            vemmon_cards = [
                cs for cs in perm.card_sources[:-1]
                if any('Vemmon' in n for n in getattr(cs, 'card_names', []))
            ]
            if len(vemmon_cards) < 4:
                return

            # Return 4 [Vemmon] to deck bottom
            for i in range(4):
                chosen = vemmon_cards[i]
                if chosen in perm.card_sources:
                    perm.card_sources.remove(chosen)
                owner = getattr(chosen, 'owner', player)
                if owner:
                    owner.library_cards.append(chosen)

            # Prevention of removal is signaled to engine by WhenRemoveField
            # with is_optional=True — engine cancels the removal when this
            # effect fires.

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # ─── Effect 3: [Start of Your Main Phase] Trash the top card of your
        # opponent's security stack.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnStartMainPhase)
        effect3.set_effect_name("BT11-111 Trash the top card of opponent's security")
        effect3.set_effect_description(
            "[Start of Your Main Phase] Trash the top card of your opponent's "
            "security stack.")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Trash opponent's top security card."""
            player = ctx.get('player')
            if not player:
                return
            enemy = player.enemy if player else None
            if enemy and enemy.security_cards:
                top_sec = enemy.security_cards[0]
                enemy.trash_security_card(top_sec)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
