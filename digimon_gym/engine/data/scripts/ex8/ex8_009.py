from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_009(CardScript):
    """EX8-009 Guilmon (X Antibody) | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX8-009 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Guilmon] for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Guilmon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Guilmon') or permanent.contains_card_name('Gigimon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 3 cards of your deck. Add 1 card with [Growlmon]/[Gallantmon] in its name and 1 [X Antibody] among them to the hand. Return the rest to the bottom of the deck.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX8-009 Reveal the top 3 cards of deck")
        effect1.set_effect_description("[On Play] Reveal the top 3 cards of your deck. Add 1 card with [Growlmon]/[Gallantmon] in its name and 1 [X Antibody] among them to the hand. Return the rest to the bottom of the deck.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def _reveal_and_add(ctx: Dict[str, Any]):
            """Reveal top 3, add 1 Growlmon/Gallantmon + 1 X Antibody to hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Reveal top 3
            revealed = []
            for _ in range(min(3, len(player.library_cards))):
                revealed.append(player.library_cards.pop(0))

            added = []
            # Add 1 card with Growlmon or Gallantmon in name
            for c in revealed:
                if c.contains_card_name('Growlmon') or c.contains_card_name('Gallantmon'):
                    added.append(c)
                    player.hand_cards.append(c)
                    break

            # Add 1 X Antibody trait card
            for c in revealed:
                if c not in added:
                    traits = getattr(c, 'card_traits', []) or []
                    if any('X Antibody' in t for t in traits):
                        added.append(c)
                        player.hand_cards.append(c)
                        break

            # Return rest to bottom
            remaining = [c for c in revealed if c not in added]
            for c in remaining:
                player.library_cards.append(c)

        effect1.set_on_process_callback(_reveal_and_add)
        effects.append(effect1)

        # [When Digivolving] Same reveal effect
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX8-009 When Digivolving: Reveal top 3")
        effect2.set_effect_description("[When Digivolving] Reveal top 3, add Growlmon/Gallantmon + X Antibody.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_reveal_and_add)
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [Your Turn][Once Per Turn] When any of your opponent's Digimon is deleted, gain 1 memory.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDestroyedAnyone)
        effect3.set_effect_name("EX8-009 Memory +1")
        effect3.set_effect_description("[Your Turn][Once Per Turn] When any of your opponent's Digimon is deleted, gain 1 memory.")
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Memory+1_EX8_009")
        effect3.is_on_deletion = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
