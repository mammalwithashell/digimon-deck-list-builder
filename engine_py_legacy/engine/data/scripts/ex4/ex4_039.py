from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX4_039(CardScript):
    """EX4-039 Gabumon | Lv.3 Purple Digimon

    [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with
        [Garurumon] in its name and 1 Digimon card with [Agumon], [Greymon],
        or [Omnimon] in its name among them to your hand. Place the remaining
        cards at the bottom of your deck in any order.

    --- Inherited ---
    [Your Turn][Once Per Turn] When one of your other Digimon digivolves,
        gain 1 memory.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_garurumon_digimon(c) -> bool:
            if not getattr(c, 'is_digimon', False):
                return False
            return c.contains_card_name('Garurumon')

        def _is_agumon_greymon_omnimon_digimon(c) -> bool:
            if not getattr(c, 'is_digimon', False):
                return False
            if c.contains_card_name('Omnimon'):
                return True
            if c.contains_card_name('Agumon'):
                return True
            if c.contains_card_name('Greymon'):
                return True
            return False

        # --- Effect 0: [On Play] Reveal top 3, add Garurumon + Agumon/Greymon/Omnimon ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX4-039 Reveal top 3, add Garurumon + Agumon/Greymon/Omnimon")
        effect0.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card "
            "with [Garurumon] in its name and 1 Digimon card with [Agumon], "
            "[Greymon], or [Omnimon] in its name among them to your hand. "
            "Place the remaining cards at the bottom of your deck in any order."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player or len(player.library_cards) < 1:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Reveal top 3
            revealed = []
            for _ in range(min(3, len(player.library_cards))):
                revealed.append(player.library_cards.pop(0))

            added = []

            # Pass 1: Add 1 Digimon with [Garurumon] in its name
            for c in revealed:
                if c not in added and _is_garurumon_digimon(c):
                    added.append(c)
                    player.hand_cards.append(c)
                    break

            # Pass 2: Add 1 Digimon with [Agumon], [Greymon], or [Omnimon]
            for c in revealed:
                if c not in added and _is_agumon_greymon_omnimon_digimon(c):
                    added.append(c)
                    player.hand_cards.append(c)
                    break

            # Place the rest at the bottom of deck
            remaining = [c for c in revealed if c not in added]
            for c in remaining:
                player.library_cards.append(c)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1 (Inherited): [Your Turn][Once Per Turn] When one of your
        #     other Digimon digivolves, gain 1 memory. ---
        # Uses _is_digivolve_observer pattern: engine fires this via
        # _fire_digivolve_observers, which already skips the digivolving
        # permanent itself (only fires on OTHER permanents).
        effect1 = ICardEffect()
        effect1.set_effect_name("EX4-039 Inherited: Memory +1 on other Digimon digivolve")
        effect1.set_effect_description(
            "[Your Turn][Once Per Turn] When one of your other Digimon digivolves, "
            "gain 1 memory."
        )
        effect1.is_inherited_effect = True
        effect1._is_digivolve_observer = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Memory+1_EX4_039")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            if not player.is_my_turn:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
