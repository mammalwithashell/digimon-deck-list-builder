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

    def _fire_timing(self, timing: 'EffectTiming', context: dict = None):
        """Fire an effect timing via the game if available."""
        if self.game and hasattr(self.game, 'execute_effects'):
            self.game.execute_effects(timing, context)

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
        """Legacy setup helper: shuffle, set security, draw opening hand.

        New game flow uses explicit mulligan-aware steps in Game.start_game().
        This method is retained for compatibility in tests/utilities.
        """
        self.shuffle_for_game_start()
        self.setup_security_stack(5)
        self.draw_opening_hand(5)

    def shuffle_for_game_start(self):
        random.shuffle(self.library_cards)
        random.shuffle(self.digitama_library_cards)

    def draw_opening_hand(self, count: int = 5):
        for _ in range(count):
            self.draw()

    def setup_security_stack(self, count: int = 5):
        for _ in range(count):
            if self.library_cards:
                self.security_cards.append(self.library_cards.pop(0))

    def draw(self) -> bool:
        if not self.library_cards:
            self._log(f"{self.player_name} cannot draw! Deck empty.")
            return False

        card = self.library_cards.pop(0)
        self.hand_cards.append(card)
        self._log(f"{self.player_name} drew a card. Hand size: {len(self.hand_cards)}")
        self._fire_timing(EffectTiming.OnAddHand, {"added_card": card, "player": self})
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
        # OnMove timing is fired by game.py after verifying the move succeeded

    def play_card(self, card_source: 'CardSource') -> Optional['Permanent']:
        if card_source in self.hand_cards:
            self.hand_cards.remove(card_source)
            new_permanent = Permanent([card_source])
            # Track summoning sickness
            if self.game:
                new_permanent.turn_played = self.game.turn_count
                new_permanent._owner_game = self.game
            self.battle_area.append(new_permanent)
            self._log(f"{self.player_name} played {card_source.card_names[0]}.")
            return new_permanent
        return None

    def unsuspend_all(self, skip_reboot: bool = False):
        """Unsuspend all permanents. If skip_reboot=True, skip those with <Reboot>
        (they unsuspend during the opponent's unsuspend phase instead).
        Permanents with <cannot unsuspend> are skipped."""
        for perm in self.battle_area:
            if skip_reboot and perm.has_keyword('_is_reboot'):
                continue  # <Reboot> permanents unsuspend on opponent's turn
            if perm.has_keyword('_is_cannot_unsuspend'):
                continue  # Restriction: cannot unsuspend
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
        # 1. Determine Base Cost (cheapest matching route)
        from ..validation.digivolve_validator import get_alt_digi_cost, _check_alt_digivolve
        candidates = []
        # Standard evo costs: find matching ones for this base permanent
        if card_source.c_entity_base and card_source.c_entity_base.evo_costs:
            base_colors = set(permanent.top_card.card_colors) if permanent.top_card else set()
            base_level = permanent.level
            for evo_cost in card_source.c_entity_base.evo_costs:
                if evo_cost.level == base_level and evo_cost.card_color in base_colors:
                    candidates.append(evo_cost.memory_cost)
        # Alt evo cost
        if _check_alt_digivolve(card_source, permanent):
            candidates.append(get_alt_digi_cost(card_source, permanent))
        base_cost = min(candidates) if candidates else 0

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

    def delete_permanent(self, permanent: 'Permanent', is_battle: bool = False,
                         is_opponent_effect: bool = False) -> bool:
        """Delete a permanent from the battle area.

        Checks deletion prevention keywords in order:
        - <Progress>: immune to opponent effects while attacking
        - <Decoy>: substitute another Digimon to prevent deletion (opponent effect only)
        - <Scapegoat>: redirect own deletion to another friendly Digimon
        - <Material Save>: save digi source cards under a Tamer before deletion
        - <Armor Purge>: trash top digivolution card to survive
        - <Evade>: suspend self (if unsuspended) to survive
        - <Barrier>: trash top security card (if in battle) to survive
        Post-deletion:
        - <Fragment>: return to bottom of deck instead of trash
        - <Fortitude>: replay from trash if had digi cards (mandatory)
        - <Save>: place under a Tamer (optional)

        Args:
            permanent: The permanent to delete.
            is_battle: True if deletion is from battle resolution (needed for Barrier).
            is_opponent_effect: True if deletion is caused by an opponent's effect.

        Returns:
            True if the permanent was actually deleted, False if not found or
            deletion was prevented by a keyword.
        """
        if permanent not in self.battle_area:
            return False

        perm_name = permanent.top_card.card_names[0] if permanent.top_card else "Unknown"

        # Fire pre-deletion timing (allows effects to react before prevention checks)
        self._fire_timing(EffectTiming.WhenPermanentWouldBeDeleted,
                          {"permanent": permanent, "player": self, "is_battle": is_battle})

        # Check modifier registry for destruction prevention
        if self.game and hasattr(self.game, 'modifiers'):
            if not self.game.modifiers.can_be_destroyed(permanent, is_battle=is_battle):
                self._log(f"{perm_name} is protected by a continuous effect!")
                return False

        # <Progress>: immune to opponent effects while attacking
        # This blocks opponent effects even during battle (e.g. Retaliation)
        if is_opponent_effect and permanent.is_immune_to_opponent_effects:
            self._log(f"{perm_name} is protected by Progress while attacking!")
            return False

        # <Scapegoat>: redirect own deletion to another friendly Digimon
        # (DCGO: triggers when the permanent would be removed from field, except by own effects)
        if permanent.has_keyword('_is_scapegoat'):
            scapegoat_targets = [
                p for p in self.battle_area
                if p is not permanent and p.is_digimon
            ]
            if scapegoat_targets and self.game:
                target = scapegoat_targets[0]
                target_name = target.top_card.card_names[0] if target.top_card else "Unknown"
                self._log(f"[Scapegoat] {perm_name} redirects deletion to {target_name}!")
                self.battle_area.remove(target)
                if self.game and hasattr(self.game, 'cleanup_modifiers_for_permanent'):
                    self.game.cleanup_modifiers_for_permanent(target)
                self._apply_ace_overflow(target.card_sources)
                self.trash_cards.extend(target.card_sources)
                if self.game and hasattr(self.game, 'execute_deletion_effects'):
                    self.game.execute_deletion_effects(target, self)
                return False

        # <Decoy>: another Digimon with Decoy can be deleted instead (opponent effect only)
        if is_opponent_effect and not is_battle:
            decoy_candidates = [
                p for p in self.battle_area
                if p is not permanent and p.has_keyword('_is_decoy') and p.is_digimon
            ]
            if decoy_candidates and self.game:
                # For RL simplicity, auto-activate the first Decoy candidate
                # (a full implementation would park for selection, but Decoy is rare
                # and the transpiler doesn't extract color, so we use the first match)
                decoy = decoy_candidates[0]
                decoy_name = decoy.top_card.card_names[0] if decoy.top_card else "Unknown"
                self._log(f"[Decoy] {decoy_name} sacrificed to save {perm_name}!")
                # Delete the Decoy instead (not opponent effect, to avoid recursion)
                self.battle_area.remove(decoy)
                self._apply_ace_overflow(decoy.card_sources)
                self.trash_cards.extend(decoy.card_sources)
                if self.game and hasattr(self.game, 'execute_deletion_effects'):
                    self.game.execute_deletion_effects(decoy, self)
                return False

        # <Material Save>: save digi source cards under a Tamer before deletion
        if permanent.has_keyword('_is_material_save') and len(permanent.card_sources) > 1:
            tamers = [p for p in self.battle_area if p.is_tamer and p is not permanent]
            if tamers:
                # Save 1 card from digi sources to the first available Tamer
                saved_card = permanent.card_sources[0]  # Bottom of digi stack
                permanent.card_sources.remove(saved_card)
                tamers[0].add_card_source_bottom(saved_card)
                tamer_name = tamers[0].top_card.card_names[0] if tamers[0].top_card else "Unknown"
                saved_name = saved_card.card_names[0] if saved_card.card_names else "Unknown"
                self._log(f"[Material Save] {saved_name} placed under {tamer_name}")

        # <Armor Purge>: trash top digivolution card to prevent deletion
        if permanent.has_keyword('_is_armor_purge') and len(permanent.card_sources) > 1:
            trashed = permanent.trash_digivolution_cards(1, from_top=True)
            if trashed:
                self.trash_cards.extend(trashed)
                self._log(f"{perm_name} activates Armor Purge! Trashed top card to survive.")
                return False

        # <Evade>: suspend self to prevent deletion (must be unsuspended)
        if permanent.has_keyword('_is_evade') and not permanent.is_suspended:
            permanent.suspend()
            self._log(f"{perm_name} activates Evade! Suspended to survive.")
            return False

        # <Barrier>: trash top security card to prevent deletion (battle only)
        if is_battle and permanent.has_keyword('_is_barrier') and len(self.security_cards) > 0:
            trashed_sec = self.security_cards.pop(0)  # Remove from top (index 0)
            self.trash_cards.append(trashed_sec)
            self._log(f"{perm_name} activates Barrier! Trashed top security to survive.")
            return False

        # No prevention — actually delete
        # Capture state before deletion for Fragment/Fortitude/Save checks
        had_digi_cards = len(permanent.card_sources) > 1
        has_fragment = permanent.has_keyword('_is_fragment')
        has_fortitude = permanent.has_keyword('_is_fortitude')
        has_save = permanent.has_keyword('_is_save')
        top_card = permanent.top_card

        self.battle_area.remove(permanent)
        # Clean up continuous modifiers sourced from this permanent
        if self.game and hasattr(self.game, 'cleanup_modifiers_for_permanent'):
            self.game.cleanup_modifiers_for_permanent(permanent)

        if permanent.is_token:
            # Tokens cease to exist — don't go to trash
            self._log(f"Token {perm_name} removed from the game.")
        else:
            self._apply_ace_overflow(permanent.card_sources)
            # <Fragment>: return top card to bottom of deck instead of trash
            if has_fragment and top_card:
                self.library_cards.append(top_card)
                under_cards = [c for c in permanent.card_sources if c is not top_card]
                self.trash_cards.extend(under_cards)
                self._log(f"[Fragment] {perm_name} returned to bottom of deck!")
            else:
                self.trash_cards.extend(permanent.card_sources)
            self._log(f"{self.player_name}'s permanent {perm_name} deleted.")

        self._fire_timing(EffectTiming.WhenRemoveField, {"permanent": permanent, "player": self})
        self._fire_timing(EffectTiming.OnRemovedField, {"permanent": permanent, "player": self})

        # Execute On Deletion effects (fires for tokens too)
        if self.game and hasattr(self.game, 'execute_deletion_effects'):
            self.game.execute_deletion_effects(permanent, self)

        if not permanent.is_token:
            # <Fortitude>: mandatory — replay from trash if had digi cards
            if has_fortitude and had_digi_cards and top_card and top_card in self.trash_cards:
                self.trash_cards.remove(top_card)
                new_perm = self.play_card_from_source(top_card, pay_cost=False)
                self._log(f"[Fortitude] {perm_name} replays from trash!")
                if self.game and hasattr(self.game, 'execute_effects'):
                    self.game.execute_effects(EffectTiming.OnEnterFieldAnyone, {"played_card": top_card})

            # <Save>: optional — place under a Tamer
            elif has_save and top_card and top_card in self.trash_cards:
                tamers = [p for p in self.battle_area if p.is_tamer]
                if tamers:
                    # Auto-select first Tamer for RL simplicity
                    tamer = tamers[0]
                    self.trash_cards.remove(top_card)
                    tamer.add_card_source_bottom(top_card)
                    tamer_name = tamer.top_card.card_names[0] if tamer.top_card else "Unknown"
                    self._log(f"[Save] {perm_name} placed under {tamer_name}")

        return True

    def security_attack(self, attacker: 'Permanent') -> AttackResolution:
        """Process a security check against this player.

        DCGO reference: ISecurityCheck class + AttackProcess security region.
        - Reveals top security card as 'security_digimon' if it's a Digimon
        - Activates SecuritySkill effects on the revealed card
        - If security card is a Digimon, battles against attacker
        - Jamming prevents attacker deletion vs security Digimon
        - DontBattleSecurityDigimon modifier skips the battle entirely
        """
        self._log(f"{self.player_name} receives Security Attack from {attacker.top_card.card_names[0] if attacker.top_card else 'Unknown'}!")

        if len(self.security_cards) == 0:
            self._log("Direct Attack! No Security cards left.")
            return AttackResolution.GameEnd

        # Reveal top card
        security_card = self.security_cards.pop(0)
        self._log(f"Security Check: Revealed {security_card.card_names[0]}")

        # Track Security Digimon concept (DCGO: AttackProcess.SecurityDigimon)
        security_digimon = security_card if security_card.is_digimon else None

        # Execute Security Effects
        # <Progress>: attacker is immune to opponent's effects (including security) while attacking
        if attacker.is_immune_to_opponent_effects:
            self._log(f"Security effects blocked — attacker has Progress!")
        else:
            security_effects = security_card.effect_list(EffectTiming.SecuritySkill)
            for effect in security_effects:
                if effect.is_security_effect:
                    self._log(f"Activating Security Effect: {effect.effect_name}")
                    if effect.on_process_callback:
                        context = {
                            "game": self.game,
                            "player": self,
                            "permanent": None,  # Security card isn't on a permanent
                            "card": security_card,
                            "security_digimon": security_digimon,
                            "attacker": attacker,
                            "turn_player": self.game.turn_player if self.game else None,
                            "opponent_player": self.game.opponent_player if self.game else None,
                        }
                        effect.on_process_callback(context)

        result = AttackResolution.Survivor

        # Battle against Security Digimon
        if security_digimon is not None:
            # DCGO: DontBattleSecurityDigimon modifier skips battle entirely
            dont_battle = False
            if self.game and hasattr(self.game, 'modifiers'):
                from ..interfaces.modifiers import ModifierType
                dont_battle = self.game.modifiers.has_modifier(
                    attacker, ModifierType.DONT_BATTLE_SECURITY_DIGIMON)

            if dont_battle:
                self._log(f"Attacker skips battle with Security Digimon (effect)")
            else:
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
        self._fire_timing(EffectTiming.OnLoseSecurity, {"lost_card": security_card, "player": self})
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
        if drawn:
            self._fire_timing(EffectTiming.OnAddHand, {"added_cards": drawn, "player": self})
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
        trashed = []
        for card in cards:
            if card in self.hand_cards:
                self.hand_cards.remove(card)
                self.trash_cards.append(card)
                trashed.append(card)
        if trashed:
            self._fire_timing(EffectTiming.OnDiscardHand, {"discarded_cards": trashed, "player": self})

    def bounce_permanent_to_hand(self, permanent: 'Permanent'):
        """Return a permanent from battle area to its owner's hand (top card only, rest to trash)."""
        # Restriction: cannot return to hand
        if permanent.has_keyword('_is_cannot_return_to_hand'):
            perm_name = permanent.top_card.card_names[0] if permanent.top_card and permanent.top_card.card_names else 'Permanent'
            self._log(f"{perm_name} cannot be returned to hand!")
            return
        owner = self._find_permanent_owner(permanent)
        if owner and permanent in owner.battle_area:
            owner.battle_area.remove(permanent)
            if self.game and hasattr(self.game, 'cleanup_modifiers_for_permanent'):
                self.game.cleanup_modifiers_for_permanent(permanent)
            if permanent.is_token:
                # Tokens cease to exist when bounced
                self._log(f"Token bounced — removed from the game.")
            else:
                if permanent.top_card:
                    owner.hand_cards.append(permanent.top_card)
                # Digivolution cards under top go to trash — trigger ACE overflow
                under_cards = permanent.card_sources[:-1]
                self._apply_ace_overflow(under_cards)
                for card in under_cards:
                    owner.trash_cards.append(card)
            self._fire_timing(EffectTiming.WhenReturntoHandAnyone, {"permanent": permanent, "player": owner})
            self._fire_timing(EffectTiming.OnPermamemtReturnedToHand, {"permanent": permanent, "player": owner})

    def recovery(self, count: int):
        """Add cards from the top of library to security stack."""
        added = []
        for _ in range(count):
            if self.library_cards:
                card = self.library_cards.pop(0)
                self.security_cards.append(card)
                added.append(card)
        if added:
            self._fire_timing(EffectTiming.OnAddSecurity, {"added_cards": added, "player": self})

    def mill(self, count: int) -> List['CardSource']:
        """Trash cards from the top of library."""
        milled = []
        for _ in range(count):
            if self.library_cards:
                card = self.library_cards.pop(0)
                self.trash_cards.append(card)
                milled.append(card)
        if milled:
            self._fire_timing(EffectTiming.OnDiscardLibrary, {"milled_cards": milled, "player": self})
        return milled

    def add_to_security_from_hand(self, card: 'CardSource', to_top: bool = True):
        """Move a card from hand to security stack."""
        if card in self.hand_cards:
            self.hand_cards.remove(card)
            if to_top:
                self.security_cards.append(card)
            else:
                self.security_cards.insert(0, card)
            self._fire_timing(EffectTiming.OnAddSecurity, {"added_cards": [card], "player": self})

    def trash_security_card(self, card: 'CardSource'):
        """Remove a specific card from the security stack and trash it."""
        if card in self.security_cards:
            self.security_cards.remove(card)
            self.trash_cards.append(card)
            self._fire_timing(EffectTiming.OnDiscardSecurity, {"discarded_card": card, "player": self})
            self._fire_timing(EffectTiming.OnLoseSecurity, {"lost_card": card, "player": self})

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

    def put_permanent_to_security(self, permanent: 'Permanent'):
        """Move a permanent from the battle area to the security stack.
        The top card goes to security; digivolution cards under it go to trash."""
        if permanent not in self.battle_area:
            return
        self.battle_area.remove(permanent)
        if permanent.is_token:
            # Tokens cease to exist when moved to security
            self._log(f"Token moved to security — removed from the game.")
            return
        if permanent.top_card:
            self.security_cards.append(permanent.top_card)
            self._log(f"{permanent.top_card.card_names[0]} placed into security.")
        # Digivolution cards under top go to trash
        under_cards = permanent.card_sources[:-1]
        self._apply_ace_overflow(under_cards)
        self.trash_cards.extend(under_cards)

    def return_permanent_to_deck_bottom(self, permanent: 'Permanent'):
        """Return a permanent to the bottom of its owner's deck.
        Top card goes to deck bottom; digivolution cards under go to trash."""
        # Restriction: cannot return to deck
        if permanent.has_keyword('_is_cannot_return_to_deck'):
            perm_name = permanent.top_card.card_names[0] if permanent.top_card and permanent.top_card.card_names else 'Permanent'
            self._log(f"{perm_name} cannot be returned to deck!")
            return
        owner = self._find_permanent_owner(permanent)
        if owner and permanent in owner.battle_area:
            owner.battle_area.remove(permanent)
            if self.game and hasattr(self.game, 'cleanup_modifiers_for_permanent'):
                self.game.cleanup_modifiers_for_permanent(permanent)
            if permanent.is_token:
                # Tokens cease to exist when returned to deck
                self._log(f"Token returned to deck — removed from the game.")
            else:
                if permanent.top_card:
                    owner.library_cards.append(permanent.top_card)
                under_cards = permanent.card_sources[:-1]
                self._apply_ace_overflow(under_cards)
                owner.trash_cards.extend(under_cards)
            self._fire_timing(EffectTiming.WhenReturntoLibraryAnyone, {"permanent": permanent, "player": owner})

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
