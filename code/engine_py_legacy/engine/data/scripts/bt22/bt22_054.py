from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_054(CardScript):
    """BT22-054 Hagurumon | Lv.3

    Digivolve: Lv.2 w/[CS] trait for cost 0.
    [Your Turn] [Once Per Turn] When effects place Digimon cards with the [CS]
        trait in this Digimon's digivolution cards, 1 of your opponent's Digimon
        gets -3000 DP for the turn.
    Inherited [Main] [Once Per Turn] By placing this [CS] trait Digimon's top
        stacked card as its bottom digivolution card, <Draw 1>.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Alt-digivolve: Lv.2 w/[CS] trait for cost 0 ──────────────────
        # C# reference: targetPermanent.TopCard.IsLevel2 && HasCSTraits
        # NO color restriction (Black is the card's own color, not a requirement).
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-054 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution: Lv.2 w/[CS] trait for cost 0")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 2
        effect0._alt_digi_trait = "CS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ── C1: [Your Turn][OPT] OnAddDigivolutionCards CS Digimon
        #         → 1 opponent Digimon gets -3000 DP for the turn. ──
        # MUST be is_inherited_effect=True so the effect remains reachable
        # once Hagurumon drops out of top_card after stacking.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnAddDigivolutionCards)
        effect1.set_effect_name("BT22-054 Give 1 digimon -3k DP")
        effect1.set_effect_description(
            "[Your Turn] [Once Per Turn] When effects place Digimon cards with "
            "the [CS] trait in this Digimon's digivolution cards, 1 of your "
            "opponent's Digimon gets -3000 DP for the turn."
        )
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BT22_054_OnAddDigivolutionCards")

        def condition1(context: Dict[str, Any]) -> bool:
            # Field presence
            if card is None or card.permanent_of_this_card() is None:
                return False
            # [Your Turn]
            owner = card.owner
            if owner is None or not owner.is_my_turn:
                return False
            # Scope: only when the event fired on THIS Digimon's permanent
            event_perm = context.get('event_permanent') or context.get('permanent')
            if event_perm is None or event_perm is not card.permanent_of_this_card():
                return False
            # Filter: added card must be a Digimon with the [CS] trait
            added = context.get('added_card')
            if added is None:
                return False
            if not getattr(added, 'is_digimon', False):
                return False
            traits = getattr(added, 'card_traits', []) or []
            if not any('CS' == t for t in traits):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Select 1 opponent Digimon, apply -3000 DP until end of turn."""
            game = ctx.get('game')
            if game is None or card is None:
                return
            owner = card.owner
            if owner is None:
                return

            def on_select(target_perm):
                # change_dp uses _dp_modifiers cleared at end of turn.
                # Attached directly to the target permanent, so no target-equality
                # guard is needed.
                target_perm.change_dp(-3000)

            game.effect_select_opponent_permanent(
                owner, on_select,
                filter_fn=lambda p: p.is_digimon and p.dp is not None,
                is_optional=False,
                prompt="Select 1 of your opponent's Digimon to give -3000 DP.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ── C2: Inherited [Main][OPT] By placing this [CS] trait Digimon's
        #       top stacked card as its bottom digivolution card, <Draw 1>. ──
        # "Top stacked card" = card_sources[-1] (the CURRENT TOP card). The
        # top card (the CS Digimon itself) is moved to the bottom, effectively
        # soft de-digivolving. Whatever was under it becomes the new top.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2._is_field_main = True
        effect2.is_inherited_effect = True
        effect2.set_effect_name(
            "BT22-054 Place the top card of this Digimon at the bottom of "
            "digivolution cards to Draw 1"
        )
        effect2.set_effect_description(
            "[Main] [Once Per Turn] By placing this [CS] trait Digimon's top "
            "stacked card as its bottom digivolution card, <Draw 1> (Draw 1 "
            "card from your deck)."
        )
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("ReturnDigivolutionCards_BT22_054")

        def condition2(context: Dict[str, Any]) -> bool:
            # Field presence
            if card is None or card.permanent_of_this_card() is None:
                return False
            # [Main] gate — owner's turn
            owner = card.owner
            if owner is None or not owner.is_my_turn:
                return False
            # For field_main effects, effect.effect_source_permanent is NOT set.
            # Use context['permanent'] — the scanned permanent hosting the effect.
            perm = context.get('permanent')
            if perm is None:
                return False
            # Must have at least 2 card sources (top + at least 1 underneath)
            if len(perm.card_sources) < 2:
                return False
            # Top card must be a CS Digimon
            top = perm.top_card
            if top is None or not getattr(top, 'is_digimon', False):
                return False
            traits = getattr(top, 'card_traits', []) or []
            if not any('CS' == t for t in traits):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Cost: move current top stacked card to bottom of stack.
            Action: draw 1 card."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player is None or perm is None:
                return
            # Pop current top (card_sources[-1]) and insert at index 0.
            # Whatever was under the top becomes the new top.
            if len(perm.card_sources) >= 2:
                top = perm.card_sources.pop()
                perm.card_sources.insert(0, top)
            # <Draw 1>
            player.draw_cards(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
