from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_014(CardScript):
    """EX9-014 Gabumon | Lv.3 | Reptile/DM/Ver.2
    [On Play] Reveal top 3 cards. Add 1 [DM] trait card to hand, place 1 [Ver.2]
    trait card face down as bottom digi card of a [DM] Digimon. Return rest to bottom.
    Inherited: <Jamming>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] Reveal top 3, add DM, place Ver.2
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX9-014 On Play: Reveal top 3, add DM + place Ver.2")
        effect0.set_effect_description(
            "[On Play] Reveal top 3 cards. Add 1 [DM] trait card to hand, "
            "place 1 [Ver.2] trait card as bottom digi card of a [DM] Digimon."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            revealed = []
            for _ in range(min(3, len(player.library_cards))):
                revealed.append(player.library_cards.pop(0))

            added = []
            # Add 1 [DM] trait card to hand
            for c in revealed:
                traits = getattr(c, 'card_traits', []) or []
                if any('DM' in t for t in traits):
                    added.append(c)
                    player.hand_cards.append(c)
                    break

            placed = []
            # Place 1 [Ver.2] trait card as bottom digi of a DM Digimon
            for c in revealed:
                if c not in added:
                    traits = getattr(c, 'card_traits', []) or []
                    if any('Ver.2' in t for t in traits):
                        for perm in player.battle_area:
                            if perm.is_digimon and perm.has_trait('DM'):
                                perm.add_card_source_bottom(c)
                                placed.append(c)
                                break
                        break

            # Return rest to bottom
            remaining = [c for c in revealed if c not in added and c not in placed]
            for c in remaining:
                player.library_cards.append(c)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Inherited: <Jamming>
        effect1 = ICardEffect()
        effect1.set_effect_name("EX9-014 Inherited: Jamming")
        effect1.set_effect_description("Inherited: <Jamming>")
        effect1.is_inherited_effect = True
        effect1._is_jamming = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
