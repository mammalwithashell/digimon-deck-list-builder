from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import CardColor, EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST12_12(CardScript):
    """ST12-12 Sistermon Blanc (Awakened) | Lv.3

    [On Play] By trashing 1 card in your hand, <Draw 2>.
        (Draw 2 cards from your deck.)
    [All Turns] While you have a Digimon in play with [Huckmon] in its name
        or [Royal Knight] trait, this Digimon gains <Decoy (Red/Black)>.
        (When your other Red or Black Digimon would be deleted by an
        opponent's effect, you may delete this Digimon to prevent 1 of
        those Digimon's deletion.)
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] By trashing 1 card in your hand, <Draw 2> ---
        # "By trashing" = cost; entire effect is optional.
        # Must have at least 1 card in hand to pay the cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("ST12-12 Trash 1 card from hand to Draw 2")
        effect0.set_effect_description(
            "[On Play] By trashing 1 card in your hand, <Draw 2>. "
            "(Draw 2 cards from your deck.)"
        )
        effect0.is_on_play = True
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must have at least 1 card in hand to pay trash cost
            player = card.owner if card else None
            if not player or len(player.hand_cards) < 1:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Trash 1 card from hand as cost, then Draw 2."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def hand_filter(c):
                return True

            def on_trashed(selected):
                # Use the player API to properly fire OnDiscardHand events
                player.trash_from_hand([selected])
                # Cost paid -- now draw 2
                player.draw_cards(2)

            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [All Turns] Conditional Decoy (Red/Black) ---
        # Gains <Decoy (Red/Black)> while you have a Digimon with
        # [Huckmon] in name or [Royal Knight] trait in play.
        # _decoy_colors restricts which Digimon colors this Decoy can protect.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.NoTiming)
        effect1.set_effect_name("ST12-12 Decoy (Red/Black)")
        effect1.set_effect_description(
            "<Decoy (Red/Black)> (When your other Red or Black Digimon "
            "would be deleted by an opponent's effect, you may delete "
            "this Digimon to prevent 1 of those Digimon's deletion.)"
        )
        effect1._is_decoy = True
        effect1._decoy_colors = [CardColor.Red, CardColor.Black]
        effect1.is_declarative = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Check if any allied Digimon has [Huckmon] in name
            # or [Royal Knight] trait
            for p in player.battle_area:
                if not p.is_digimon:
                    continue
                if p.contains_card_name('Huckmon'):
                    return True
                if p.has_trait('Royal Knight'):
                    return True
            return False

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
