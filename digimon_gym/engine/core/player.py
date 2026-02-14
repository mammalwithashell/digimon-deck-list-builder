from __future__ import annotations
from typing import TYPE_CHECKING, List, Optional, Callable
import random
from .permanent import Permanent
from ..data.enums import EffectTiming, CardKind, AttackResolution

if TYPE_CHECKING:
    from .card_source import CardSource
    from ..data.evo_cost import DnaCost

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

        self.enemy: Optional['Player'] = None
        self.game: Optional[object] = None  # Back-reference to Game, set at start

    def _log(self, message: str):
        """Log via the game's logger if available."""
        if self.game and hasattr(self.game, 'logger'):
            self.game.logger.log(message)

    def _apply_ace_overflow(self, card_sources: List['CardSource']):
        """Check card sources for ACE cards and apply overflow memory penalty.

        ACE Overflow triggers when an ACE card moves from the field or from
        under a card to any other area (trash, hand, deck, security).
        Uses lose_memory() which correctly adjusts Game.memory based on
        which player is losing memory.
        """
        for card in card_sources:
            if card.c_entity_base and card.c_entity_base.is_ace:
                penalty = card.c_entity_base.ace_overflow_cost
                if penalty > 0:
                    self.lose_memory(penalty)
                    self._log(f"ACE Overflow: {card.card_names[0]} loses {penalty} memory")

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
            self._log(f"{self.player_name} cannot draw! Deck empty.")
            return False

        card = self.library_cards.pop(0)
        self.hand_cards.append(card)
        self._log(f"{self.player_name} drew a card. Hand size: {len(self.hand_cards)}")
        return True

    def hatch(self):
        if self.breeding_area is not None:
            self._log("Cannot hatch: Breeding area occupied.")
            return

        if not self.digitama_library_cards:
            self._log("Cannot hatch: Digitama deck empty.")
            return

        card = self.digitama_library_cards.pop(0)
        new_permanent = Permanent([card])
        self.breeding_area = new_permanent
        self._log(f"{self.player_name} hatched {card.card_names[0]}.")

    def move_from_breeding(self):
        if self.breeding_area is None:
            self._log("Cannot move: Breeding area empty.")
            return

        # Rule: Must be Level 3 or higher to move?
        # Actually, Level 2 (Digitama) cannot move. Level 3 (Rookie) can.
        if (self.breeding_area.level or 0) < 3:
            self._log("Cannot move: Digimon level too low (must be Level 3+).")
            return

        perm = self.breeding_area
        self.breeding_area = None
        # Entering battle area from breeding — track game ref (not sickness; digimon was hatched earlier)
        if self.game:
            perm._owner_game = self.game
            # Moving from breeding doesn't cause summoning sickness per rules
            # (the Digimon was hatched on a prior turn)
            if perm.turn_played < 0:
                perm.turn_played = -1  # keep -1 = no sickness
        self.battle_area.append(perm)
        self._log(f"{self.player_name} moved {perm.top_card.card_names[0]} from Breeding to Battle Area.")

    def play_card(self, card_source: 'CardSource'):
        if card_source in self.hand_cards:
            self.hand_cards.remove(card_source)
            new_permanent = Permanent([card_source])
            # Track summoning sickness
            if self.game:
                new_permanent.turn_played = self.game.turn_count
                new_permanent._owner_game = self.game
            self.battle_area.append(new_permanent)
            self._log(f"{self.player_name} played {card_source.card_names[0]}.")

    def unsuspend_all(self, skip_reboot: bool = False):
        """Unsuspend all permanents. If skip_reboot=True, skip those with <Reboot>
        (they unsuspend during the opponent's unsuspend phase instead)."""
        for perm in self.battle_area:
            if skip_reboot and perm.has_keyword('_is_reboot'):
                continue  # <Reboot> permanents unsuspend on opponent's turn
            perm.unsuspend()
        if self.breeding_area:
            self.breeding_area.is_suspended = False
        self._log(f"{self.player_name} unsuspended permanents.")

    def unsuspend_reboot_only(self):
        """Unsuspend only permanents with <Reboot>. Called during opponent's unsuspend phase."""
        for perm in self.battle_area:
            if perm.has_keyword('_is_reboot') and perm.is_suspended:
                perm.unsuspend()
                self._log(f"{perm.top_card.card_names[0] if perm.top_card else 'Unknown'} unsuspends (Reboot).")

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
                    self._log(f"Effect {effect.effect_name} reduced cost by {effect.cost_reduction}")

        final_cost = max(0, base_cost - reduction)

        # 3. Pay Cost
        # Memory can go negative.
        # self.memory -= final_cost # Game handles memory
        self._log(f"{self.player_name} pays {final_cost} memory to digivolve.")

        # 4. Stack Card
        if card_source in self.hand_cards:
            self.hand_cards.remove(card_source)
        permanent.add_card_source(card_source)
        self._log(f"Digivolved into {card_source.card_names[0]}.")

        return final_cost

    def dna_digivolve(self, top_perm: 'Permanent', bottom_perm: 'Permanent',
                      card_source: 'CardSource', dna_cost: 'DnaCost') -> int:
        """Execute DNA Digivolution: combine two permanents under a new card.

        Stacking order (bottom to top):
          bottom_perm's sources → top_perm's sources → card_source

        Rules applied:
        - Both permanents are removed from battle area
        - New permanent is placed unsuspended (can attack immediately)
        - Digivolution bonus: draw 1 card
        - The result is treated as a new Digimon

        Args:
            top_perm: The permanent matching requirement1 (sources go on top).
            bottom_perm: The permanent matching requirement2 (sources go on bottom).
            card_source: The DNA digivolve card from hand.
            dna_cost: The DnaCost entry being used.

        Returns:
            The memory cost paid.
        """
        base_cost = dna_cost.memory_cost

        # Remove card from hand
        if card_source in self.hand_cards:
            self.hand_cards.remove(card_source)

        # Build combined source stack: bottom sources, then top sources, then DNA card
        combined_sources = list(bottom_perm.card_sources) + list(top_perm.card_sources)
        combined_sources.append(card_source)

        # Remove both permanents from battle area
        if top_perm in self.battle_area:
            self.battle_area.remove(top_perm)
        if bottom_perm in self.battle_area:
            self.battle_area.remove(bottom_perm)

        # Create new permanent with combined stack (unsuspended)
        new_perm = Permanent(combined_sources)
        new_perm.is_suspended = False
        # DNA digivolve creates a new permanent; no summoning sickness per rules
        if self.game:
            new_perm._owner_game = self.game
            new_perm.turn_played = -1  # DNA digivolve doesn't have summoning sickness
        self.battle_area.append(new_perm)

        self._log(f"{self.player_name} DNA Digivolved into {card_source.card_names[0]}.")

        # Digivolution bonus: draw 1
        self.draw()

        return base_cost

    def delete_permanent(self, permanent: 'Permanent', is_battle: bool = False):
        """Delete a permanent from the battle area.

        Checks deletion prevention keywords in order:
        - <Armor Purge>: trash top digivolution card to survive
        - <Evade>: suspend self (if unsuspended) to survive
        - <Barrier>: trash top security card (if in battle) to survive

        Args:
            permanent: The permanent to delete.
            is_battle: True if deletion is from battle resolution (needed for Barrier).
        """
        if permanent not in self.battle_area:
            return

        perm_name = permanent.top_card.card_names[0] if permanent.top_card else "Unknown"

        # <Armor Purge>: trash top digivolution card to prevent deletion
        if permanent.has_keyword('_is_armor_purge') and len(permanent.card_sources) > 1:
            trashed = permanent.trash_digivolution_cards(1, from_top=True)
            if trashed:
                self.trash_cards.extend(trashed)
                self._log(f"{perm_name} activates Armor Purge! Trashed top card to survive.")
                return

        # <Evade>: suspend self to prevent deletion (must be unsuspended)
        if permanent.has_keyword('_is_evade') and not permanent.is_suspended:
            permanent.suspend()
            self._log(f"{perm_name} activates Evade! Suspended to survive.")
            return

        # <Barrier>: trash top security card to prevent deletion (battle only)
        if is_battle and permanent.has_keyword('_is_barrier') and len(self.security_cards) > 0:
            trashed_sec = self.security_cards.pop()
            self.trash_cards.append(trashed_sec)
            self._log(f"{perm_name} activates Barrier! Trashed top security to survive.")
            return

        # No prevention — actually delete
        self.battle_area.remove(permanent)
        self._apply_ace_overflow(permanent.card_sources)
        self.trash_cards.extend(permanent.card_sources)
        self._log(f"{self.player_name}'s permanent {perm_name} deleted.")

        # Execute On Deletion effects
        if self.game and hasattr(self.game, 'execute_deletion_effects'):
            self.game.execute_deletion_effects(permanent, self)

    def security_attack(self, attacker: 'Permanent') -> AttackResolution:
        self._log(f"{self.player_name} receives Security Attack from {attacker.top_card.card_names[0] if attacker.top_card else 'Unknown'}!")

        if len(self.security_cards) == 0:
            self._log("Direct Attack! No Security cards left.")
            return AttackResolution.GameEnd

        # Reveal top card
        security_card = self.security_cards.pop(0)
        self._log(f"Security Check: Revealed {security_card.card_names[0]}")

        # Execute Security Effects
        security_effects = security_card.effect_list(EffectTiming.SecuritySkill)
        for effect in security_effects:
            if effect.is_security_effect:
                self._log(f"Activating Security Effect: {effect.effect_name}")
                if effect.on_process_callback:
                    effect.on_process_callback()

        result = AttackResolution.Survivor

        # Battle
        if security_card.is_digimon:
            a_dp = attacker.dp or 0
            s_dp = security_card.base_dp or 0
            self._log(f"Battle: Attacker DP {a_dp} vs Security DP {s_dp}")
            if a_dp < s_dp or a_dp == s_dp:
                # <Jamming>: Digimon can't be deleted in battles against Security Digimon
                if attacker.has_keyword('_is_jamming'):
                    self._log(f"Attacker {attacker.top_card.card_names[0]} survives (Jamming).")
                else:
                    self._log(f"Attacker {attacker.top_card.card_names[0]} is deleted by Security Digimon!")
                    result = AttackResolution.AttackerDeleted
            else:
                self._log(f"Attacker {attacker.top_card.card_names[0]} survives.")

        # Trash the security card
        self.trash_cards.append(security_card)
        return result

    # ─── Effect Action Methods ───────────────────────────────────────

    def draw_cards(self, count: int) -> List['CardSource']:
        """Draw N cards. Returns the cards drawn.
        If deck runs out mid-draw, signals deck-out loss via game."""
        drawn = []
        for _ in range(count):
            if not self.library_cards:
                # Deck-out: player loses when they must draw but can't
                if self.game and hasattr(self.game, 'declare_winner'):
                    opponent = self.game.player2 if self is self.game.player1 else self.game.player1
                    self.game.declare_winner(opponent)
                break
            card = self.library_cards.pop(0)
            self.hand_cards.append(card)
            drawn.append(card)
        return drawn

    def add_memory(self, amount: int):
        """Gain memory for this player (adjusts Game.memory)."""
        if self.game:
            if self is self.game.turn_player:
                self.game.memory += amount
            else:
                self.game.memory -= amount

    def lose_memory(self, amount: int):
        """Lose memory for this player."""
        self.add_memory(-amount)

    def trash_from_hand(self, cards: List['CardSource']):
        """Move specific cards from hand to trash."""
        for card in cards:
            if card in self.hand_cards:
                self.hand_cards.remove(card)
                self.trash_cards.append(card)

    def bounce_permanent_to_hand(self, permanent: 'Permanent'):
        """Return a permanent from battle area to its owner's hand (top card only, rest to trash)."""
        owner = self._find_permanent_owner(permanent)
        if owner and permanent in owner.battle_area:
            owner.battle_area.remove(permanent)
            if permanent.top_card:
                owner.hand_cards.append(permanent.top_card)
            # Digivolution cards under top go to trash — trigger ACE overflow
            under_cards = permanent.card_sources[:-1]
            self._apply_ace_overflow(under_cards)
            for card in under_cards:
                owner.trash_cards.append(card)

    def recovery(self, count: int):
        """Add cards from the top of library to security stack."""
        for _ in range(count):
            if self.library_cards:
                card = self.library_cards.pop(0)
                self.security_cards.append(card)

    def mill(self, count: int) -> List['CardSource']:
        """Trash cards from the top of library."""
        milled = []
        for _ in range(count):
            if self.library_cards:
                card = self.library_cards.pop(0)
                self.trash_cards.append(card)
                milled.append(card)
        return milled

    def add_to_security_from_hand(self, card: 'CardSource', to_top: bool = True):
        """Move a card from hand to security stack."""
        if card in self.hand_cards:
            self.hand_cards.remove(card)
            if to_top:
                self.security_cards.append(card)
            else:
                self.security_cards.insert(0, card)

    def trash_security_card(self, card: 'CardSource'):
        """Remove a specific card from the security stack and trash it."""
        if card in self.security_cards:
            self.security_cards.remove(card)
            self.trash_cards.append(card)

    def remove_from_security(self, card: 'CardSource') -> Optional['CardSource']:
        """Remove a card from the security stack (without trashing). Returns the card."""
        if card in self.security_cards:
            self.security_cards.remove(card)
            return card
        return None

    def reveal_top_cards(self, count: int) -> List['CardSource']:
        """Reveal top N cards of library (peek without removing)."""
        return self.library_cards[:count]

    def play_card_from_source(self, card: 'CardSource', pay_cost: bool = True):
        """Play a card (from hand, trash, or reveal) onto the battle area."""
        if card in self.hand_cards:
            self.hand_cards.remove(card)
        elif card in self.trash_cards:
            self.trash_cards.remove(card)
        # Don't remove from library — caller handles reveal placement
        new_permanent = Permanent([card])
        # Track summoning sickness
        if self.game:
            new_permanent.turn_played = self.game.turn_count
            new_permanent._owner_game = self.game
        self.battle_area.append(new_permanent)
        return new_permanent

    def get_battle_area_digimons(self) -> List['Permanent']:
        """Get all Digimon permanents in battle area."""
        return [p for p in self.battle_area if p.is_digimon]

    def _find_permanent_owner(self, permanent: 'Permanent') -> Optional['Player']:
        """Find which player owns a permanent."""
        if permanent in self.battle_area:
            return self
        if self.enemy and permanent in self.enemy.battle_area:
            return self.enemy
        return None
