from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any, Optional
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import CardKind, CardColor, Rarity, EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource
    from ....core.permanent import Permanent
    from ....core.player import Player

class BT20_034(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Fortitude>
        fortitude = ICardEffect()
        fortitude.set_effect_name("Fortitude")
        fortitude.set_effect_description("<Fortitude> (When this Digimon with digivolution cards is deleted, play this card without paying the cost.)")
        fortitude.is_keyword_effect = True
        fortitude.keyword = "Fortitude"
        effects.append(fortitude)


        # [All Turns] When Tamer cards are placed in this Digimon's digivolution cards,
        # 1 of your opponent's Digimon can't activate [When Digivolving] effects until the end of their turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("Block When Digivolving Effects")
        effect1.set_effect_description("[All Turns] When Tamer cards are placed in this Digimon's digivolution cards, 1 of your opponent's Digimon can't activate [When Digivolving] effects until the end of their turn.")
        effect1.set_timing(EffectTiming.OnAddDigivolutionCards)

        def condition1(context: Dict[str, Any]) -> bool:
            permanent: Optional[Permanent] = effect1.effect_source_permanent
            target_permanent: Optional[Permanent] = context.get("permanent")

            if permanent != target_permanent:
                return False

            added_cards: List[CardSource] = context.get("added_cards", [])
            for c in added_cards:
                if c.card_kind == CardKind.Tamer:
                    return True
            return False

        def activate1(context: Dict[str, Any]):
            # Select 1 opponent digimon
            # Since this is headless, we might need a selection manager or simple heuristic (e.g. highest DP)
            # For now, simplistic approach: pick first opponent digimon.
            permanent: Optional[Permanent] = effect1.effect_source_permanent
            player: Optional[Player] = permanent.player
            opponent = context.get("opponent") # Assumed context has opponent or derived
            # If no opponent in context, we can't target.
            # In a real game loop, we'd have access to Game state.
            # Here we act as if we can access the opponent via the permanent's reference if linked properly.
            # But Permanent.player is just a reference.
            # Let's assume we can get opponent via simple logic if 2 players.

            # Placeholder: print effect activation
            print("BT20-034 Effect Activated: Preventing opponent When Digivolving effects.")
            # In a real implementation, we would attach a temporary effect/condition to the target permanent.

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(activate1) # Use callback for activation
        effects.append(effect1)

        # Inherited Effect
        # [All Turns] [Once Per Turn] When this Digimon deletes your opponent's Digimon in battle, trash their top security card.
        inherited = ICardEffect()
        inherited.set_effect_name("Trash Security on Deletion")
        inherited.set_effect_description("[All Turns] [Once Per Turn] When this Digimon deletes your opponent's Digimon in battle, trash their top security card.")
        inherited.is_inherited_effect = True
        inherited.set_timing(EffectTiming.OnDestroyedAnyone)
        inherited.set_limit_once_per_turn()

        def inherited_condition(context: Dict[str, Any]) -> bool:
            winner: Optional[Permanent] = context.get("winner")
            loser: Optional[Permanent] = context.get("loser")

            if winner == inherited.effect_source_permanent and loser and loser.player != winner.player:
                return True
            return False

        def inherited_activate(context: Dict[str, Any]):
            permanent: Optional[Permanent] = inherited.effect_source_permanent
            # Assuming we can find opponent player
            # For now, just print
            print("BT20-034 Inherited Effect Activated: Trash opponent security.")
            # If we had access to opponent player object:
            # opponent.trash_security(1)

        inherited.set_can_use_condition(inherited_condition)
        inherited.set_on_process_callback(inherited_activate)
        effects.append(inherited)

        return effects
