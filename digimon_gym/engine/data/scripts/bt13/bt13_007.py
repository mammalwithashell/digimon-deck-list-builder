from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_007(CardScript):
    """BT13-007 King Drasil_7D6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect: Play Cost reduction
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-007 Play Cost -")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Implement cost reduction logic here if needed (e.g. based on Royal Knights)
            # For now, just setting property as placeholder
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            pass

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnStartMainPhase
        # [Breeding][Start of Your Main Phase] Reveal the top card of your Digi-Egg deck, then place that card and all of your [Royal Knight] trait Digimon as this Digimon as its bottom digivolution cards.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-007 Reveal the top card of your Digi - Egg deck and place your Digimons under this Digimon")
        effect1.set_effect_description("[Breeding][Start of Your Main Phase] Reveal the top card of your Digi-Egg deck, then place that card and all of your [Royal Knight] trait Digimon as this Digimon as its bottom digivolution cards.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must be in Breeding Area
            perm = card.permanent_of_this_card()
            if not perm:
                return False
            # Check if perm is breeding area
            if card.owner.breeding_area is not perm:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent') # King Drasil
            game = ctx.get('game')
            if not (player and perm and game):
                return

            # Reveal top egg
            if player.digitama_library_cards:
                egg = player.digitama_library_cards.pop(0)
                game.logger.log(f"[BT13-007] Revealed egg: {egg.card_names[0]}")
                perm.add_card_source_bottom(egg)

            # Find all Royal Knight Digimon in Battle Area
            royal_knights = []
            for p in list(player.battle_area): # Copy list to iterate safely
                if p.is_digimon and p.has_trait("Royal Knight"):
                    royal_knights.append(p)

            for p in royal_knights:
                game.logger.log(f"[BT13-007] Absorbing {p.top_card.card_names[0]}")
                if p in player.battle_area:
                    player.battle_area.remove(p)

                # Move sources to King Drasil
                # Order: usually top card first if stack preserved, or whole stack?
                # "place ... Digimon ... as its bottom digivolution cards"
                # We'll just append them.
                for source in p.card_sources:
                    perm.add_card_source_bottom(source)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Breeding][Your Turn][Once Per Turn] When an Option card with the [Royal Knight] trait is placed in the battle area, gain 1 memory.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-007 Memory +1")
        effect2.set_effect_description("[Breeding][Your Turn][Once Per Turn] When an Option card with the [Royal Knight] trait is placed in the battle area, gain 1 memory.")
        # Inherited: False (it's a Breeding effect of the main card)
        # But transpiler marked it as inherited? No, "Breeding" effects are usually on the card itself but active in breeding.
        effect2.is_inherited_effect = False
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Memory+1_BT13_007")
        # Trigger: OnEnterFieldAnyone (Option placed)
        # Transpiler set is_on_play=True, which is wrong for "When an Option... is placed".
        # It should check the played card.

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            perm = card.permanent_of_this_card()
            if not perm or card.owner.breeding_area is not perm:
                return False

            # Check if played card is Option with Royal Knight trait
            played_card = context.get('played_card')
            if not played_card:
                return False
            if not played_card.is_option:
                return False
            # Trait check (fuzzy)
            if "Royal Knight" not in played_card.card_traits:
                return False

            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
