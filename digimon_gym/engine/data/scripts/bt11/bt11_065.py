from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_065(CardScript):
    """BT11-065 Snatchmon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_vemmon_name(c) -> bool:
            return any('Vemmon' in n for n in getattr(c, 'card_names', []))

        def _is_fusionize_name(c) -> bool:
            return any('Fusionize' in n for n in getattr(c, 'card_names', []))

        # ─── Effect 0: [When Digivolving] You may place up to 2 [Vemmon] from
        # your trash under this Digimon as its bottom digivolution cards. Then,
        # if there are 4 or more [Vemmon] in this Digimon's digivolution cards,
        # return 1 [Fusionize] from your trash to your hand.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT11-065 Place [Vemmon] from trash to digivolution cards and return 1 [Fusionize] from trash to hand")
        effect0.set_effect_description(
            "[When Digivolving] You may place up to 2 [Vemmon] from your trash "
            "under this Digimon as its bottom digivolution cards. Then, if there "
            "are 4 or more [Vemmon] in this Digimon's digivolution cards, return "
            "1 [Fusionize] from your trash to your hand.")
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """[When Digivolving] Place up to 2 Vemmon from trash as bottom digi-cards, then check 4+ Vemmon for Fusionize recovery."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            # Place up to 2 [Vemmon] from trash as bottom digi-cards
            placed = 0
            for c in list(player.trash_cards):
                if placed >= 2:
                    break
                if _is_vemmon_name(c):
                    player.trash_cards.remove(c)
                    perm.add_card_source_bottom(c)
                    placed += 1

            # Then, if 4+ [Vemmon] in digi-cards (excluding top card)
            vemmon_count = sum(
                1 for cs in perm.card_sources[:-1]
                if any('Vemmon' in n for n in getattr(cs, 'card_names', []))
            )
            if vemmon_count >= 4:
                # Return 1 [Fusionize] from trash to hand
                for c in list(player.trash_cards):
                    if _is_fusionize_name(c):
                        player.trash_cards.remove(c)
                        player.hand_cards.append(c)
                        break

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ─── Effect 1 (Inherited): [All Turns][Once Per Turn] When [Vemmon] is
        # placed from this Digimon's digivolution cards at the bottom of its
        # owner's deck, unsuspend this Digimon, and it gains <Blocker> until the
        # end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDigivolutionCardReturnToDeckBottom)
        effect1.set_effect_name("BT11-065 Unsuspend this Digimon and it gains Blocker")
        effect1.set_effect_description(
            "[All Turns][Once Per Turn] When [Vemmon] is placed from this "
            "Digimon's digivolution cards at the bottom of its owner's deck, "
            "unsuspend this Digimon, and it gains <Blocker> until the end of "
            "your opponent's turn.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Unsuspend_BT11_065")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # The returned card must be [Vemmon]
            returned_card = context.get('returned_card')
            if returned_card is None:
                return False
            if not _is_vemmon_name(returned_card):
                return False
            # The permanent in context must be THIS Digimon
            ctx_perm = context.get('permanent')
            my_perm = card.permanent_of_this_card()
            if ctx_perm is not None and my_perm is not None and ctx_perm is not my_perm:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Unsuspend this Digimon and grant Blocker until end of opponent's turn."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            my_perm = card.permanent_of_this_card()
            if my_perm:
                my_perm.unsuspend()
                # Grant Blocker until end of opponent's turn
                from ....interfaces.modifiers import ModifierType
                game.register_modifier(
                    my_perm, ModifierType.GRANT_BLOCKER,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
