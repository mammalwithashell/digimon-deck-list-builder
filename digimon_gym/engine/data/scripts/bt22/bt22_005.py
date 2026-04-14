from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_005(CardScript):
    """BT22-005 Tsumemon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] [Once Per Turn] When any of your Digimon with the [Unidentified] or [CS] trait are played, <Draw 1> (Draw 1 card from your deck.)
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT22-005 Draw 1")
        effect0.set_effect_description("[Your Turn] [Once Per Turn] When any of your Digimon with the [Unidentified] or [CS] trait are played, <Draw 1> (Draw 1 card from your deck.)")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("BT22_005_Draw1")
        # NOTE: Do NOT set is_on_play = True. This is an observer effect that
        # triggers when ANY matching Digimon is played, not just the one this
        # card is under. The engine's OnEnterFieldAnyone timing without is_on_play
        # fires for ALL permanents on field, and the condition filters.

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Must be on a Digimon on the battle area (inherited effect requirement)
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Card text: "When any of your Digimon with the [Unidentified] or [CS] trait are played"
            played_perm = context.get('played_permanent')
            if not played_perm:
                return False
            # Must be a Digimon (not Tamer/Option)
            if not played_perm.is_digimon:
                return False
            # Must be owned by the same player
            owner = card.owner if card else None
            event_player = context.get('event_player')
            if event_player and owner and event_player is not owner:
                return False
            # Check traits on the played Digimon's top card
            top = played_perm.top_card
            if not top:
                return False
            traits = getattr(top, 'card_traits', []) or []
            if not any('Unidentified' in t or t == 'CS' for t in traits):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
