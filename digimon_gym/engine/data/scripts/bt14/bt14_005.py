from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_005(CardScript):
    """BT14-005 Missimon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] By returning 3 cards with the [D-Brigade] or [DigiPolice] trait from your trash to the top of the deck, this Digimon gets +2000 DP for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-005 Return cards from trash to gain DP +2000")
        effect0.set_effect_description("[When Attacking][Once Per Turn] By returning 3 cards with the [D-Brigade] or [DigiPolice] trait from your trash to the top of the deck, this Digimon gets +2000 DP for the turn.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("ReturnCards_BT14_005")
        effect0.is_on_attack = True
        effect0.dp_modifier = 2000

        def _extract_traits(c: Any) -> List[str]:
            traits: List[str] = []

            if hasattr(c, 'get_type'):
                try:
                    t = c.get_type()
                    if isinstance(t, list):
                        traits.extend([x for x in t if isinstance(x, str)])
                    elif isinstance(t, str):
                        traits.append(t)
                except Exception:
                    pass

            if hasattr(c, 'card_data') and isinstance(getattr(c, 'card_data'), dict):
                cd = c.card_data
                raw = cd.get('type_eng', [])
                if isinstance(raw, list):
                    traits.extend([x for x in raw if isinstance(x, str)])
                elif isinstance(raw, str):
                    traits.append(raw)

            return traits

        def _is_valid_trait_card(c: Any) -> bool:
            traits = _extract_traits(c)
            return ('D-Brigade' in traits) or ('DigiPolice' in traits)

        def _get_trash_cards(player: Any) -> List[Any]:
            if player is None:
                return []
            for attr in ('trash', 'trash_cards', 'graveyard'):
                zone = getattr(player, attr, None)
                if isinstance(zone, list):
                    return zone
            return []

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = context.get('player')
            trash_cards = _get_trash_cards(player)
            valid_count = sum(1 for c in trash_cards if _is_valid_trait_card(c))
            return valid_count >= 3

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Cost: return 3 matching cards from trash to top of deck. Effect: DP +2000 for turn."""
            player = ctx.get('player')
            perm = ctx.get('permanent')

            trash_cards = _get_trash_cards(player)
            selected = []
            for c in list(trash_cards):
                if _is_valid_trait_card(c):
                    selected.append(c)
                    if len(selected) == 3:
                        break

            if len(selected) < 3:
                return

            # Return selected cards from trash to top of deck (deterministic order).
            for c in selected:
                if c in trash_cards:
                    trash_cards.remove(c)

            deck = getattr(player, 'deck', None) if player is not None else None
            if isinstance(deck, list):
                for c in reversed(selected):
                    deck.insert(0, c)
            elif player is not None and hasattr(player, 'move_card'):
                for c in selected:
                    try:
                        player.move_card(c, 'trash', 'deck_top')
                    except Exception:
                        pass

            if perm:
                perm.change_dp(2000)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
