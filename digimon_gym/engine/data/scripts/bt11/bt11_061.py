from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_061(CardScript):
    """BT11-061 Vemmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_vemmon_name(c) -> bool:
            return any('Vemmon' in n for n in getattr(c, 'card_names', []))

        def _is_hand_target(c) -> bool:
            """Snatchmon, Destromon, Galacticmon, or Fusionize."""
            names = getattr(c, 'card_names', [])
            for n in names:
                if ('Snatchmon' in n or 'Destromon' in n
                        or 'Galacticmon' in n or 'Fusionize' in n):
                    return True
            return False

        # ─── Effect 0: [Main] By suspending this Digimon, reveal the top 3 cards
        # of your deck. Add 1 [Snatchmon], [Destromon], [Galacticmon], or
        # [Fusionize] among them to your hand, and place 1 [Vemmon] among them
        # under this Digimon as its bottom digivolution card. Place the rest at
        # the bottom of your deck in any order.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDeclaration)
        effect0.set_effect_name("BT11-061 Reveal the top 3 cards of deck")
        effect0.set_effect_description(
            "[Main] By suspending this Digimon, reveal the top 3 cards of your "
            "deck. Add 1 [Snatchmon], [Destromon], [Galacticmon], or [Fusionize] "
            "among them to your hand, and place 1 [Vemmon] among them under this "
            "Digimon as its bottom digivolution card. Place the rest at the bottom "
            "of your deck in any order.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must not already be suspended
            perm = card.permanent_of_this_card()
            if perm and perm.is_suspended:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """[Main] Suspend self, reveal top 3, add 1 target to hand, place 1 Vemmon as bottom digi-card, rest to deck bottom."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            # Cost: suspend this Digimon
            perm.suspend()

            # Reveal top 3 cards
            revealed = player.library_cards[:3]
            if not revealed:
                return
            for c in revealed:
                player.library_cards.remove(c)

            pool = list(revealed)

            # Pass 1: Add 1 Snatchmon/Destromon/Galacticmon/Fusionize to hand
            hand_candidates = [c for c in pool if _is_hand_target(c)]
            if hand_candidates:
                chosen = hand_candidates[0]
                pool.remove(chosen)
                player.hand_cards.append(chosen)

            # Pass 2: Place 1 [Vemmon] as bottom digi-card of this Digimon
            vemmon_candidates = [c for c in pool if _is_vemmon_name(c)]
            if vemmon_candidates:
                chosen = vemmon_candidates[0]
                pool.remove(chosen)
                perm.add_card_source_bottom(chosen)

            # Remaining: place at bottom of deck
            for c in pool:
                player.library_cards.append(c)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ─── Effect 1 (Inherited): [Your Turn] When this Digimon would
        # digivolve into [Destromon] or [Galacticmon], reduce the digivolution
        # cost by 1. [Once Per Turn]
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT11-061 Inherited cost -1 for Destromon/Galacticmon")
        effect1.set_effect_description(
            "[Your Turn] When this Digimon would digivolve into [Destromon] or "
            "[Galacticmon], reduce the digivolution cost by 1.")
        effect1.is_inherited_effect = True
        effect1.cost_reduction = 1
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("InheritedCostReduction_BT11_061")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Leak guard: only apply when THIS card's permanent is being digivolved
            if context.get('card_source') is not card:
                return False
            # Check that the digivolve target is Destromon or Galacticmon
            digi_target = context.get('digivolve_card')
            if digi_target is None:
                return True  # timing may not provide target; allow engine to handle
            target_names = getattr(digi_target, 'card_names', [])
            return any('Destromon' in n or 'Galacticmon' in n for n in target_names)

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Cost reduction handled via cost_reduction property."""
            pass

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
