from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST19_04(CardScript):
    """ST19-04 PawnChessmon | Lv.3 (Yellow/Black, Puppet)

    [On Play] By trashing 1 card with the [Puppet] trait in your hand,
    <Draw 2>.

    Inherited Effect:
    <Reboot> (Unsuspend this Digimon during your opponent's unsuspend phase.)
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] Trash 1 Puppet from hand -> Draw 2 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("ST19-04 Trash Puppet, Draw 2")
        effect0.set_effect_description(
            "[On Play] By trashing 1 card with the [Puppet] trait in your "
            "hand, <Draw 2>."
        )
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            # Must have a Puppet card in hand
            return any(
                any('Puppet' in t for t in (getattr(c, 'card_traits', []) or []))
                for c in owner.hand_cards
            )

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Select and trash 1 Puppet from hand, then draw 2."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def puppet_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return any('Puppet' in t for t in traits)

            def on_trash(trashed_card):
                player.hand_cards.remove(trashed_card)
                player.trash_cards.append(trashed_card)
                player.draw_cards(2)

            game.effect_select_hand_card(
                player, puppet_filter, on_trash,
                is_optional=False,
                prompt="Select 1 card with [Puppet] trait to trash.",
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1 (Inherited): <Reboot> ---
        effect1 = ICardEffect()
        effect1.set_effect_name("ST19-04 Reboot")
        effect1.set_effect_description("Reboot")
        effect1.is_inherited_effect = True
        effect1._is_reboot = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
