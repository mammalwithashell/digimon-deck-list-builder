from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_043(CardScript):
    """BT22-043 Terriermon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-043 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.2 for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 2

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAddDigivolutionCards
        # [Your Turn] [Once Per Turn] When effects place Digimon cards with the [CS] trait in this Digimon's digivolution cards, if you have 1 or fewer Tamers, you may play 1 Tamer card with the [CS] trait from your hand without paying the cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnAddDigivolutionCards)
        effect1.set_effect_name("BT22-043 Play 1 [CS] Tamer")
        effect1.set_effect_description("[Your Turn] [Once Per Turn] When effects place Digimon cards with the [CS] trait in this Digimon's digivolution cards, if you have 1 or fewer Tamers, you may play 1 Tamer card with the [CS] trait from your hand without paying the cost.")
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("PlayTamer_BT22_043")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play 1 Tamer card with [CS] trait from hand without paying cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Check 1 or fewer Tamers in play
            tamer_count = sum(1 for p in player.battle_area if p.is_tamer)
            if tamer_count > 1:
                return
            def play_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('CS' in t for t in traits)
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] [Once Per Turn] By placing this [CS] trait Digimon's top stacked card as its bottom digivolution card, <Draw 1> (Draw 1 card from your deck).
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2._is_field_main = True
        effect2.set_effect_name("BT22-043 Place the top card of this Digimon at the bottom of digivolution cards to Draw 1")
        effect2.set_effect_description("[Main] [Once Per Turn] By placing this [CS] trait Digimon's top stacked card as its bottom digivolution card, <Draw 1> (Draw 1 card from your deck).")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("ReturnDigivolutionCards_BT22_043")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not permanent:
                return False
            # Must have at least 2 card_sources (top card + at least 1 stacked card)
            if len(permanent.card_sources) < 2:
                return False
            # Must be a [CS] trait Digimon
            if permanent.top_card:
                traits = getattr(permanent.top_card, 'card_traits', []) or []
                if not any('CS' in t for t in traits):
                    return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Cost: place top stacked card to bottom. Action: Draw 1."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm):
                return
            # Cost: place the top stacked card (card_sources[-2], just under top)
            # as the bottom digivolution card
            if len(perm.card_sources) >= 2:
                # The "top stacked card" is card_sources[-2] (just under the top card)
                top_stacked = perm.card_sources[-2]
                perm.card_sources.remove(top_stacked)
                perm.card_sources.insert(0, top_stacked)
            # Draw 1
            player.draw_cards(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
