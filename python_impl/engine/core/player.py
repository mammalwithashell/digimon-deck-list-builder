from __future__ import annotations
from typing import TYPE_CHECKING, List, Optional
import random
from .permanent import Permanent
from ..data.enums import EffectTiming

if TYPE_CHECKING:
    from .card_source import CardSource

class Player:
    def __init__(self):
        self.player_id: int = 0
        self.player_name: str = ""
        self.is_my_turn: bool = False
        self.memory: int = 0
        self.hand_cards: List['CardSource'] = []
        self.library_cards: List['CardSource'] = []
        self.security_cards: List['CardSource'] = []
        self.trash_cards: List['CardSource'] = []
        self.digitama_library_cards: List['CardSource'] = []

        self.battle_area: List['Permanent'] = []
        self.breeding_area: Optional['Permanent'] = None

    @property
    def is_lose(self) -> bool:
        # Simple check: empty deck is not instant loss, loss happens when drawing from empty.
        # But we can flag it.
        return False

    def setup_game(self):
        # Shuffle decks
        random.shuffle(self.library_cards)
        random.shuffle(self.digitama_library_cards)

        # Set Security (top 5 cards of library)
        for _ in range(5):
            if self.library_cards:
                self.security_cards.append(self.library_cards.pop(0))

        # Draw initial hand (5 cards)
        for _ in range(5):
            self.draw()

    def draw(self):
        if not self.library_cards:
            print(f"{self.player_name} cannot draw! Deck empty.")
            return

        card = self.library_cards.pop(0)
        self.hand_cards.append(card)
        print(f"{self.player_name} drew a card. Hand size: {len(self.hand_cards)}")

    def hatch(self):
        if self.breeding_area is not None:
            print("Cannot hatch: Breeding area occupied.")
            return

        if not self.digitama_library_cards:
            print("Cannot hatch: Digitama deck empty.")
            return

        card = self.digitama_library_cards.pop(0)
        new_permanent = Permanent([card])
        self.breeding_area = new_permanent
        print(f"{self.player_name} hatched {card.card_names[0]}.")

    def play_card(self, card_source: 'CardSource'):
        if card_source in self.hand_cards:
            self.hand_cards.remove(card_source)
            new_permanent = Permanent([card_source])
            self.battle_area.append(new_permanent)
            print(f"{self.player_name} played {card_source.card_names[0]}.")

    def unsuspend_all(self):
        for perm in self.battle_area:
            perm.is_suspended = False
        if self.breeding_area:
            self.breeding_area.is_suspended = False
        print(f"{self.player_name} unsuspended all permanents.")

    def digivolve(self, permanent: 'Permanent', card_source: 'CardSource'):
        # 1. Determine Base Cost
        # Assuming evo_costs is a list of EvoCost objects in CEntity_Base.
        # We need to find the matching cost for the permanent's color/level.
        # For simplicity, taking the first valid cost or default.
        base_cost = 0
        if card_source.c_entity_base and card_source.c_entity_base.evo_costs and len(card_source.c_entity_base.evo_costs) > 0:
             # Basic logic: match color. For PoC, take first.
             # EvoCost uses 'memory_cost'
             base_cost = card_source.c_entity_base.evo_costs[0].memory_cost

        # 2. Trigger WhenWouldDigivolve
        # Collect effects from Permanent (inherited and top)
        # Using the same get_active_effects logic or similar, but filtered by timing.

        reduction = 0

        # Check effects on the permanent (Sources + Top)
        all_effects = []

        # Sources
        for source in permanent.digivolution_cards:
             effects = source.effect_list(EffectTiming.WhenWouldDigivolve)
             for effect in effects:
                 # Check if inherited (if source is not top) or normal (if source is top)
                 is_top = (source == permanent.top_card)
                 if (is_top and not effect.is_inherited_effect) or (not is_top and effect.is_inherited_effect):
                     all_effects.append(effect)

        context = {
            "player": self,
            "permanent": permanent,
            "card_source": card_source
        }

        for effect in all_effects:
            if effect.can_use_condition and effect.can_use_condition(context):
                if effect.cost_reduction > 0:
                    reduction += effect.cost_reduction
                    print(f"Effect {effect.effect_name} reduced cost by {effect.cost_reduction}")

        final_cost = max(0, base_cost - reduction)

        # 3. Pay Cost
        if self.memory < final_cost:
             # Actually, memory can go negative (passing turn).
             pass

        self.memory -= final_cost
        print(f"{self.player_name} pays {final_cost} memory to digivolve.")

        # 4. Stack Card
        if card_source in self.hand_cards:
            self.hand_cards.remove(card_source)
        permanent.add_card_source(card_source)
        print(f"Digivolved into {card_source.card_names[0]}.")
