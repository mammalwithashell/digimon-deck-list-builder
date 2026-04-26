from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT18_019(CardScript):
    """BT18-019 Millenniummon | Lv.7 Red/Black Composite"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] [When Digivolving] Delete 1 opponent's Digimon.
        #     Then if DNA digivolving, return 1 of each different-level Digimon
        #     card from opponent's trash to top of deck, gain 1 memory per card. ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT18-019 Delete opponent Digimon + DNA bonus")
        effect0.set_effect_description(
            "[On Play] [When Digivolving] Delete 1 of your opponent's Digimon. "
            "Then, if DNA digivolving, by returning 1 of each Digimon card with "
            "different levels from your opponent's trash to the top of the deck, "
            "gain 1 memory for each card returned."
        )
        effect0.is_on_play = True
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete 1 opponent Digimon, then DNA bonus"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Delete 1 of opponent's Digimon
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

            # DNA digivolving bonus: return different-level Digimon from
            # opponent's trash to top of their deck, gain memory per card.
            # NOTE: Detecting DNA digivolving state is not directly available
            # in the callback context. We check if this permanent has cards
            # from 2+ different base names as a heuristic for DNA.
            perm = card.permanent_of_this_card() if card else None
            is_dna = False
            if perm and len(perm.card_sources) >= 2:
                # Check if digivolution stack contains both Kimeramon and Machinedramon
                names = set()
                for cs in perm.card_sources:
                    for n in cs.card_names:
                        names.add(n.lower())
                if 'kimeramon' in names and 'machinedramon' in names:
                    is_dna = True

            if is_dna and enemy.trash_cards:
                # Return 1 card of each different level from opponent's trash
                levels_returned = set()
                cards_to_return = []
                for c in list(enemy.trash_cards):
                    if c.is_digimon and c.level is not None and c.level not in levels_returned:
                        levels_returned.add(c.level)
                        cards_to_return.append(c)
                for c in cards_to_return:
                    enemy.trash_cards.remove(c)
                    enemy.library_cards.insert(0, c)  # top of deck
                # Gain 1 memory per card returned
                if cards_to_return:
                    player.add_memory(len(cards_to_return))

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [On Deletion] Return Kimeramon + Machinedramon from
        #     trash to bottom of deck, play 1 Millenniummon from trash free ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("BT18-019 On Deletion: Recycle and play Millenniummon")
        effect1.set_effect_description(
            "[On Deletion] By returning 1 [Kimeramon] and 1 [Machinedramon] "
            "from your trash to the bottom of the deck, you may play 1 "
            "[Millenniummon] from your trash without paying the cost."
        )
        effect1.is_on_deletion = True
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner):
                return False
            player = card.owner
            has_kimeramon = any(
                c.contains_card_name('Kimeramon') for c in player.trash_cards
            )
            has_machinedramon = any(
                c.contains_card_name('Machinedramon') for c in player.trash_cards
            )
            has_millenniummon = any(
                c.contains_card_name('Millenniummon') for c in player.trash_cards
            )
            return has_kimeramon and has_machinedramon and has_millenniummon
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Return Kimeramon+Machinedramon to deck bottom, play Millenniummon free"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Return 1 Kimeramon from trash to bottom of deck
            for i, c in enumerate(player.trash_cards):
                if c.contains_card_name('Kimeramon'):
                    returned = player.trash_cards.pop(i)
                    player.library_cards.append(returned)
                    break
            # Return 1 Machinedramon from trash to bottom of deck
            for i, c in enumerate(player.trash_cards):
                if c.contains_card_name('Machinedramon'):
                    returned = player.trash_cards.pop(i)
                    player.library_cards.append(returned)
                    break
            # Play 1 Millenniummon from trash without paying cost
            def mill_filter(c):
                return c.contains_card_name('Millenniummon') and c.is_digimon
            game.effect_play_from_zone(
                player, 'trash', mill_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
