from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST19_05(CardScript):
    """ST19-05 PawnChessmon | Lv.3 (Black/Yellow, Puppet)

    <Blocker>
    [On Deletion] By trashing 1 card with the [Puppet] trait in your hand,
    <Draw 2>.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: <Blocker> ---
        effect0 = ICardEffect()
        effect0.set_effect_name("ST19-05 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [On Deletion] Trash 1 Puppet from hand -> Draw 2 ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("ST19-05 On Deletion: Trash Puppet, Draw 2")
        effect1.set_effect_description(
            "[On Deletion] By trashing 1 card with the [Puppet] trait in your "
            "hand, <Draw 2>."
        )
        effect1.is_optional = True
        effect1.is_on_deletion = True

        def condition1(context: Dict[str, Any]) -> bool:
            # On deletion check — card no longer needs to be on field
            owner = card.owner if card else None
            if not owner:
                return False
            # Must have a Puppet card in hand
            return any(
                any('Puppet' in t for t in (getattr(c, 'card_traits', []) or []))
                for c in owner.hand_cards
            )

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
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

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
