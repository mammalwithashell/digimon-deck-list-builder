from __future__ import annotations
from typing import TYPE_CHECKING, List, Optional
import random
from .permanent import Permanent
from ..data.enums import EffectTiming, CardKind, AttackResolution

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
        # Loss condition: Deck out is checked at Draw.
        # But if we want a state check:
        # Note: Rules say you lose if you CANNOT draw.
        # Rules also say you lose if you have 0 security and take a hit.
        return False

    def check_loss(self) -> bool:
        if len(self.library_cards) == 0:
             # This is actually only triggered if required to draw.
             # But for simplicity, let's say if deck is empty at start of turn or something.
             # Actually, standard rule: Lose when deck becomes 0? No, when you must draw and cannot.
             pass
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

    def draw(self) -> bool:
        if not self.library_cards:
            print(f"{self.player_name} cannot draw! Deck empty.")
            return False

        card = self.library_cards.pop(0)
        self.hand_cards.append(card)
        print(f"{self.player_name} drew a card. Hand size: {len(self.hand_cards)}")
        return True

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

    def move_from_breeding(self):
        if self.breeding_area is None:
            print("Cannot move: Breeding area empty.")
            return

        # Rule: Must be Level 3 or higher to move?
        # Actually, Level 2 (Digitama) cannot move. Level 3 (Rookie) can.
        if self.breeding_area.level < 3:
            print("Cannot move: Digimon level too low (must be Level 3+).")
            return

        perm = self.breeding_area
        self.breeding_area = None
        self.battle_area.append(perm)
        print(f"{self.player_name} moved {perm.top_card.card_names[0]} from Breeding to Battle Area.")

    def play_card(self, card_source: 'CardSource'):
        if card_source in self.hand_cards:
            self.hand_cards.remove(card_source)
            new_permanent = Permanent([card_source])
            self.battle_area.append(new_permanent)
            print(f"{self.player_name} played {card_source.card_names[0]}.")

    def unsuspend_all(self):
        for perm in self.battle_area:
            perm.unsuspend()
        if self.breeding_area:
            perm = self.breeding_area
            # Breeding area cards don't really suspend/unsuspend in standard play logic usually?
            # They stay active. But sure.
            perm.is_suspended = False
        print(f"{self.player_name} unsuspended all permanents.")

    def digivolve(self, permanent: 'Permanent', card_source: 'CardSource'):
        # 1. Determine Base Cost
        base_cost = 0
        if card_source.c_entity_base and card_source.c_entity_base.evo_costs and len(card_source.c_entity_base.evo_costs) > 0:
             base_cost = card_source.c_entity_base.evo_costs[0].memory_cost

        # 2. Trigger WhenWouldDigivolve
        reduction = 0
        all_effects = []

        # Sources
        for source in permanent.digivolution_cards:
             effects = source.effect_list(EffectTiming.WhenWouldDigivolve)
             for effect in effects:
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
        # Memory can go negative.
        # self.memory -= final_cost # Game handles memory
        print(f"{self.player_name} pays {final_cost} memory to digivolve.")

        # 4. Stack Card
        if card_source in self.hand_cards:
            self.hand_cards.remove(card_source)
        permanent.add_card_source(card_source)
        print(f"Digivolved into {card_source.card_names[0]}.")

        return final_cost

    def delete_permanent(self, permanent: 'Permanent'):
        if permanent in self.battle_area:
            self.battle_area.remove(permanent)
            self.trash_cards.extend(permanent.card_sources)
            print(f"{self.player_name}'s permanent {permanent.top_card.card_names[0]} deleted.")

    def security_attack(self, attacker: 'Permanent') -> AttackResolution:
        print(f"{self.player_name} receives Security Attack from {attacker.top_card.card_names[0] if attacker.top_card else 'Unknown'}!")

        if len(self.security_cards) == 0:
            print("Direct Attack! No Security cards left.")
            return AttackResolution.GameEnd

        # Reveal top card
        security_card = self.security_cards.pop(0)
        print(f"Security Check: Revealed {security_card.card_names[0]}")

        # Trigger Attacker Effects (OnSecurityCheck)
        # Context includes the revealed card
        context = {
            "player": self, # The player owning the security stack (Defending player)
            "attacker": attacker,
            "security_card": security_card
        }
        # Note: Attacker is owned by the opponent.
        # We need to execute attacker's effects.
        attacker_effects = attacker.effect_list(EffectTiming.OnSecurityCheck)
        for effect in attacker_effects:
             # Context for the effect usually expects "permanent" to be the source.
             # And "player" to be the effect owner.
             # We should probably construct context properly for the effect.
             # But here we are in Player (Defender).
             # We can just run the callback if condition passes.
             effect_owner = attacker.top_card.owner if attacker.top_card else None
             eff_context = {
                 "player": effect_owner,
                 "permanent": attacker,
                 "target_card": security_card
             }
             if effect.can_use_condition and effect.can_use_condition(eff_context):
                 print(f"Activating Attacker Effect: {effect.effect_name}")
                 if effect.on_process_callback:
                     effect.on_process_callback()

        # Execute Security Effects
        security_effects = security_card.effect_list(EffectTiming.SecuritySkill)
        for effect in security_effects:
            if effect.is_security_effect:
                print(f"Activating Security Effect: {effect.effect_name}")
                if effect.on_process_callback:
                    effect.on_process_callback()

        result = AttackResolution.Survivor

        # Battle
        if security_card.is_digimon:
            print(f"Battle: Attacker DP {attacker.dp} vs Security DP {security_card.base_dp}")
            if attacker.dp < security_card.base_dp:
                print(f"Attacker {attacker.top_card.card_names[0]} is deleted by Security Digimon!")
                result = AttackResolution.AttackerDeleted
            elif attacker.dp == security_card.base_dp:
                print(f"Attacker {attacker.top_card.card_names[0]} is deleted by Security Digimon (Equal DP).")
                result = AttackResolution.AttackerDeleted
            else:
                print(f"Attacker {attacker.top_card.card_names[0]} survives.")

        # Trash the security card
        self.trash_cards.append(security_card)
        return result
