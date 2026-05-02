from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_044(CardScript):
    """BT22-044 Palmon | Lv.3 Green/Red, [Vegetation, CS]

    Card text:
      [Your Turn] [Once Per Turn] When effects place Digimon cards with the [CS]
        trait in this Digimon's digivolution cards, gain 1 memory.
      Inherited Effect [Main] [Once Per Turn] By placing this [CS] trait
        Digimon's top stacked card as its bottom digivolution card, <Draw 1>
        (Draw 1 card from your deck.)

      Alt digivolve: Lv.2 for cost 0.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Factory effect: alt_digivolve_req ───────────────────────────
        # Alternate digivolution requirement: Lv.2 for cost 0
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-044 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 2

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ── C1: [Your Turn][OPT] Gain 1 memory on CS Digimon add ────────
        # [Your Turn] [Once Per Turn] When effects place Digimon cards with the
        # [CS] trait in this Digimon's digivolution cards, gain 1 memory.
        #
        # Marked is_inherited_effect=True so the effect is scanned when Palmon
        # is a digivolution source (which is the standard lifecycle once any
        # card is placed into its stack — either via digivolve or place-as-source).
        # Mirrors the BT22-001 / BT22-004 / BT22-006 pattern for
        # OnAddDigivolutionCards body effects on low-level CS Digimon.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnAddDigivolutionCards)
        effect1.set_effect_name("BT22-044 +1 memory")
        effect1.set_effect_description("[Your Turn] [Once Per Turn] When effects place Digimon cards with the [CS] trait in this Digimon's digivolution cards, gain 1 memory.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("GainMemory_BT22_044")

        def condition1(context: Dict[str, Any]) -> bool:
            # Palmon must still be on the field (in some permanent)
            if not (card and card.permanent_of_this_card() is not None):
                return False
            # [Your Turn]
            owner = card.owner if card else None
            if not (owner and owner.is_my_turn):
                return False
            # "this Digimon's digivolution cards" — the event permanent must be
            # the permanent that actually hosts Palmon. Without this check, adds
            # to an unrelated permanent would trigger Palmon's memory gain.
            event_perm = context.get('event_permanent', context.get('permanent'))
            host_perm = card.permanent_of_this_card()
            if event_perm is None or host_perm is None or event_perm is not host_perm:
                return False
            # "Digimon cards with the [CS] trait" — the added card must be a
            # Digimon (CardKind.Digimon) with the [CS] trait.
            added_card = context.get('added_card')
            if added_card is None:
                return False
            if not getattr(added_card, 'is_digimon', False):
                return False
            traits = getattr(added_card, 'card_traits', []) or []
            if not any(t == 'CS' for t in traits):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: gain 1 memory."""
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ── C2: Inherited [Main][OPT] place top card as bottom, Draw 1 ──
        # Inherited: [Main] [Once Per Turn] By placing this [CS] trait Digimon's
        # top stacked card as its bottom digivolution card, <Draw 1> (Draw 1
        # card from your deck.)
        #
        # Cost: move the current top_card of the permanent to the bottom of its
        # digivolution stack (card_sources[-1] → insert at index 0).
        # This mirrors P-225's implementation and DCGO's
        # AddDigivolutionCardsBottom(new List<CardSource>() { topCard }) call.
        # The permanent effectively soft-de-digivolves: the card that was on top
        # becomes the new bottom source and whichever card was just beneath it
        # becomes the new top.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2._is_field_main = True
        effect2.set_effect_name("BT22-044 Place the top card of this Digimon at the bottom of digivolution cards to Draw 1")
        effect2.set_effect_description("[Main] [Once Per Turn] By placing this [CS] trait Digimon's top stacked card as its bottom digivolution card, <Draw 1> (Draw 1 card from your deck.)")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("ReturnDigivolutionCards_BT22_044")

        effect_c2 = effect2  # alias for closure

        def condition2(context: Dict[str, Any]) -> bool:
            # Palmon must be on the field (in some permanent — for inherited
            # effects this is the host perm above Palmon).
            if not (card and card.permanent_of_this_card() is not None):
                return False
            permanent = effect_c2.effect_source_permanent if hasattr(effect_c2, 'effect_source_permanent') else None
            if permanent is None:
                permanent = context.get('permanent')
            if permanent is None:
                return False
            # Must have at least 1 digivolution source beneath the top (C# uses
            # DigivolutionCards.Count >= 1, i.e., len(card_sources) >= 2).
            if len(permanent.card_sources) < 2:
                return False
            # Top card must have the [CS] trait ("this [CS] trait Digimon's
            # top stacked card" — C# checks permanent.TopCard.HasCSTraits).
            top = permanent.top_card
            if top is None:
                return False
            traits = getattr(top, 'card_traits', []) or []
            if not any(t == 'CS' for t in traits):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Cost: place the current top card at the bottom of the digi-stack.
            Action: Draw 1.
            """
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm):
                return
            if len(perm.card_sources) < 2:
                return
            # Pop the top card (card_sources[-1]) and insert at the bottom
            # (index 0). Mirrors DCGO AddDigivolutionCardsBottom semantics
            # where the Permanent.TopCard becomes the new bottom source and a
            # previously underlying source surfaces as the new top.
            top_card = perm.card_sources.pop()
            perm.card_sources.insert(0, top_card)
            # Draw 1 (only if there's a card in library)
            if len(player.library_cards) >= 1:
                player.draw_cards(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
