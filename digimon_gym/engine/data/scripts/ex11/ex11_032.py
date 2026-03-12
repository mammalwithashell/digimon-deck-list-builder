from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_032(CardScript):
    """EX11-032 GrandGalemon | Lv.5

    [Hand] [Main] If you have [Shoto Kazama], by placing 1 [Galemon] from your trash
    as any of your [Pteromon]'s bottom digivolution card, it digivolves into this card
    for a digivolution cost of 3, ignoring digivolution requirements.

    NOTE: This is a hand-activated special digivolve effect. The engine does not yet
    support "place card from trash as bottom digivolution card of an existing permanent,
    then digivolve that permanent from hand at reduced cost ignoring requirements".
    This is tracked in engine-gaps.md as a BLOCKED mechanic.
    The OnDeclaration stub is left here so the card can at least be scripted once
    the required engine API is available.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Hand][Main] Special digivolve effect — COMPLEX / PARTIALLY UNSUPPORTED
        # Requires:
        #   1. Shoto Kazama tamer on field
        #   2. Galemon in trash
        #   3. Target Pteromon on field
        #   4. Move Galemon from trash to bottom of Pteromon's digivolution stack
        #   5. Pteromon digivolves into GrandGalemon (this card) for cost 3,
        #      ignoring normal digivolution requirements
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDeclaration)
        effect0.set_effect_name(
            "EX11-032 [Hand][Main] Special digivolve via Pteromon (requires Shoto Kazama)"
        )
        effect0.set_effect_description(
            "[Hand][Main] If you have [Shoto Kazama], by placing 1 [Galemon] from your trash "
            "as any of your [Pteromon]'s bottom digivolution card, it digivolves into this card "
            "for a digivolution cost of 3, ignoring digivolution requirements."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            # Must be your turn
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must be in hand (not yet on field)
            player = card.owner if card else None
            if player is None:
                return False
            if card not in player.hand_cards:
                return False
            # Must have Shoto Kazama tamer on field
            has_shoto = any(
                p.is_tamer and p.contains_card_name('Shoto Kazama')
                for p in player.battle_area
            )
            if not has_shoto:
                return False
            # Must have a Galemon in trash
            has_galemon = any(
                any('Galemon' in n for n in getattr(c, 'card_names', []))
                for c in player.trash_cards
            )
            if not has_galemon:
                return False
            # Must have a Pteromon on field
            has_pteromon = any(
                p.is_digimon and p.contains_card_name('Pteromon')
                for p in player.battle_area
            )
            if not has_pteromon:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """
            BLOCKED: Requires engine support for placing a card from trash as the bottom
            digivolution source of an existing permanent, then triggering a special-cost
            digivolve of that permanent from hand ignoring normal requirements.

            Partial stub: select a Pteromon, then use effect_digivolve_from_hand as best
            approximation. Galemon placement from trash is not handled.
            """
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: Select a Pteromon target to digivolve
            def pteromon_filter(p):
                return p.is_digimon and p.contains_card_name('Pteromon')

            def do_special_digivolve(target_perm):
                # Step 2 (stub): Move Galemon from trash under target_perm
                # NOTE: This step is unsupported by the engine; we skip it
                galemon = next(
                    (c for c in player.trash_cards
                     if any('Galemon' in n for n in getattr(c, 'card_names', []))),
                    None
                )
                if galemon:
                    player.trash_cards.remove(galemon)
                    target_perm.add_card_source_bottom(galemon)

                # Step 3: Digivolve from hand using this card (cost 3, ignoring requirements)
                # Use effect_digivolve_from_hand as approximation
                def grand_filter(c):
                    return c is card

                game.effect_digivolve_from_hand(
                    player, target_perm, grand_filter,
                    cost_override=3, is_optional=False)

            game.effect_select_own_permanent(
                player, do_special_digivolve,
                filter_fn=pteromon_filter,
                is_optional=False,
                prompt="Select a [Pteromon] to digivolve into GrandGalemon."
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
