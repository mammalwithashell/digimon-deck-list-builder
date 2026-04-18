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
        # Alternate digivolution requirement: Lv.2 w/[CS] trait for cost 0
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-043 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 2
        effect0._alt_digi_trait = "CS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # C1: [Your Turn][Once Per Turn] Non-inherited top-text effect.
        # When effects place [CS] trait Digimon cards in this Digimon's
        # digivolution cards, if you have 1 or fewer Tamers, you may play
        # 1 Tamer card with [CS] trait from your hand without paying the cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnAddDigivolutionCards)
        effect1.set_effect_name("BT22-043 Play 1 [CS] Tamer")
        effect1.set_effect_description(
            "[Your Turn] [Once Per Turn] When effects place Digimon cards with "
            "the [CS] trait in this Digimon's digivolution cards, if you have "
            "1 or fewer Tamers, you may play 1 Tamer card with the [CS] trait "
            "from your hand without paying the cost."
        )
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("PlayTamer_BT22_043")

        def _cs_tamer_filter(c) -> bool:
            if not getattr(c, 'is_tamer', False):
                return False
            traits = getattr(c, 'card_traits', []) or []
            return 'CS' in traits

        def condition1(context: Dict[str, Any]) -> bool:
            # Non-inherited: only active when Terriermon is the top of a
            # permanent on the field. Since perm.effect_list only collects
            # non-inherited effects from the top card, reaching this condition
            # already implies Terriermon IS the top of its host permanent.
            host = card.permanent_of_this_card() if card else None
            if host is None:
                return False
            owner = card.owner if card else None
            if owner is None or not owner.is_my_turn:
                return False
            # The added_card must be a Digimon with [CS] trait.
            added_card = context.get('added_card')
            if added_card is None:
                return False
            if not getattr(added_card, 'is_digimon', False):
                return False
            added_traits = getattr(added_card, 'card_traits', []) or []
            if 'CS' not in added_traits:
                return False
            # The permanent being added to must be Terriermon's own permanent.
            event_perm = context.get('event_permanent')
            if event_perm is None or event_perm is not host:
                return False
            # 1 or fewer Tamers in play
            tamer_count = sum(1 for p in owner.battle_area if p.is_tamer)
            if tamer_count > 1:
                return False
            # Must have at least one [CS] Tamer card in hand
            if not any(_cs_tamer_filter(c) for c in owner.hand_cards):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: play 1 [CS] Tamer card from hand without paying cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Re-check Tamer count at resolution (card text gate)
            tamer_count = sum(1 for p in player.battle_area if p.is_tamer)
            if tamer_count > 1:
                return
            game.effect_play_from_zone(
                player, 'hand', _cs_tamer_filter,
                free=True, is_optional=True,
                prompt="Select 1 [CS] Tamer card to play without paying the cost.",
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # C2: Inherited [Main][Once Per Turn] effect.
        # By placing this [CS] trait Digimon's top stacked card as its bottom
        # digivolution card, <Draw 1>.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2._is_field_main = True
        effect2.set_effect_name(
            "BT22-043 Place top stacked card on bottom, Draw 1"
        )
        effect2.set_effect_description(
            "[Main] [Once Per Turn] By placing this [CS] trait Digimon's top "
            "stacked card as its bottom digivolution card, <Draw 1> (Draw 1 "
            "card from your deck.)"
        )
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("ReturnDigivolutionCards_BT22_043")

        def condition2(context: Dict[str, Any]) -> bool:
            # Must be a [CS] trait Digimon on the field with at least 1
            # digivolution card (so the top can be placed under).
            permanent = context.get('permanent')
            if permanent is None:
                return False
            top = permanent.top_card
            if top is None or not getattr(top, 'is_digimon', False):
                return False
            traits = getattr(top, 'card_traits', []) or []
            if 'CS' not in traits:
                return False
            # Need at least 2 card_sources so there's something to re-stack.
            # (DCGO: DigivolutionCards.Count >= 1; DigivolutionCards excludes
            # the TopCard in C#, so >= 1 under-card == >= 2 sources in Python.)
            if len(permanent.card_sources) < 2:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Cost: move the current top stacked card to the bottom of the
            digivolution stack. Effect: <Draw 1>."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm):
                return
            # Re-validate: at least 2 sources and top is CS Digimon
            if len(perm.card_sources) < 2:
                return
            top = perm.top_card
            if top is None or not getattr(top, 'is_digimon', False):
                return
            traits = getattr(top, 'card_traits', []) or []
            if 'CS' not in traits:
                return
            # Pay the cost: move card_sources[-1] (the current top) to index 0
            # (bottom of digivolution stack). The previous under-card becomes
            # the new top card of the permanent.
            top_card = perm.card_sources.pop()  # removes and returns last
            perm.card_sources.insert(0, top_card)
            # <Draw 1>
            player.draw_cards(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
