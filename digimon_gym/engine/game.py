from __future__ import annotations
from typing import TYPE_CHECKING, Optional, Union, List, Dict, Any, Callable
from dataclasses import dataclass, field
import random

# Support both import prefixes
try:
    from python_impl.engine.data.enums import GamePhase, EffectTiming, AttackResolution, PendingAction
    from python_impl.engine.core.player import Player
    from python_impl.engine.core.permanent import Permanent
    from python_impl.engine.data.card_registry import CardRegistry
    from python_impl.engine.loggers import IGameLogger, SilentLogger
    from python_impl.engine.validation.digivolve_validator import (
        can_digivolve, has_valid_dna_targets, get_valid_dna_first_targets,
        get_valid_dna_second_targets, get_dna_stacking_order,
    )
except ImportError:
    from digimon_gym.engine.data.enums import GamePhase, EffectTiming, AttackResolution, PendingAction
    from digimon_gym.engine.core.player import Player
    from digimon_gym.engine.core.permanent import Permanent
    from digimon_gym.engine.data.card_registry import CardRegistry
    from digimon_gym.engine.loggers import IGameLogger, SilentLogger
    from digimon_gym.engine.validation.digivolve_validator import (
        can_digivolve, has_valid_dna_targets, get_valid_dna_first_targets,
        get_valid_dna_second_targets, get_dna_stacking_order,
    )

if TYPE_CHECKING:
    from .core.card_source import CardSource

# ─── Tensor / Action Space Constants (match C# Digimon.Core) ────────
TENSOR_SIZE = 695
ACTION_SPACE_SIZE = 2120
FIELD_SLOTS = 12
SLOT_SIZE = 20
MAX_HAND = 20
MAX_TRASH = 45
MAX_SECURITY = 10
MAX_SOURCES = 14
MAX_REVEALED = 10

# ─── Selection Action Conventions ───────────────────────────────────
# When in SelectTarget/SelectMaterial/SelectHand/SelectReveal/SelectSecurity,
# valid_indices use these ranges so the RL agent can distinguish what it's selecting:
SEL_HAND_START = 0         # 0-29:     select hand card by index
SEL_HAND_END = 29
SEL_REVEALED_START = 30    # 30-39:    select from revealed cards
SEL_REVEALED_END = 39
SEL_MY_SECURITY_START = 40 # 40-49:    select from own security stack
SEL_MY_SECURITY_END = 49
SEL_OPP_SECURITY_START = 50 # 50-59:   select from opponent's security stack
SEL_OPP_SECURITY_END = 59
SEL_MY_BREEDING = 99       # 99:       select own breeding area permanent
SEL_MY_FIELD_START = 100   # 100-111:  select own battle_area permanent
SEL_MY_FIELD_END = 111
SEL_OPP_FIELD_START = 112  # 112-123:  select opponent's battle_area permanent
SEL_OPP_FIELD_END = 123
SEL_EFFECT_CHOICE_START = 1000  # 1000-1009: choose between effect branches
SEL_EFFECT_CHOICE_END = 1009


@dataclass
class PendingAttack:
    """Context for an attack in progress (paused for block/counter decisions)."""
    attacker: Permanent
    original_target: Union[Permanent, Player]
    effective_target: Union[Permanent, Player]  # changes if blocked
    is_blocked: bool = False
    blocker: Optional[Permanent] = None


@dataclass
class PendingSelection:
    """Context for an effect waiting for player selection."""
    callback: Callable[[int], None]  # receives the selected index
    selecting_player: Player
    previous_phase: GamePhase
    valid_indices: List[int] = field(default_factory=list)
    is_optional: bool = False  # if True, player can decline with action 62


class Game:
    def __init__(self, logger: Optional[IGameLogger] = None):
        self.logger: IGameLogger = logger if logger is not None else SilentLogger()

        self.player1: Player = Player()
        self.player2: Player = Player()
        self.player1.player_name = "Player 1"
        self.player2.player_name = "Player 2"
        self.player1.player_id = 1
        self.player2.player_id = 2

        # Wire up cross-references
        self.player1.enemy = self.player2
        self.player2.enemy = self.player1
        self.player1.game = self
        self.player2.game = self

        self.turn_player: Player = self.player1
        self.opponent_player: Player = self.player2

        self.memory: int = 0
        self.turn_count: int = 0
        self.current_phase: GamePhase = GamePhase.Start
        self.pending_action: PendingAction = PendingAction.NO_ACTION
        self.game_over: bool = False
        self.winner: Optional[Player] = None

        # Interrupt phase state
        self.pending_attack: Optional[PendingAttack] = None
        self.pending_selection: Optional[PendingSelection] = None
        self.active_player: Optional[Player] = None  # None = turn_player

        # Revealed cards zone (for reveal-and-select effects)
        self.revealed_cards: List['CardSource'] = []

    @property
    def current_player_id(self) -> int:
        """Return the player_id of the active player.

        During normal phases, this is the turn player.
        During BlockTiming/CounterTiming, this is the opponent (defender).
        """
        if self.active_player is not None:
            return self.active_player.player_id
        return self.turn_player.player_id

    def start_game(self):
        if random.choice([True, False]):
            self.turn_player = self.player1
            self.opponent_player = self.player2
        else:
            self.turn_player = self.player2
            self.opponent_player = self.player1

        self.player1.setup_game()
        self.player2.setup_game()

        self.turn_count = 1
        self.current_phase = GamePhase.Start
        self.memory = 0
        self.turn_player.is_my_turn = True
        self.opponent_player.is_my_turn = False

        self.phase_start()

    def next_phase(self):
        if self.game_over:
            return

        if self.current_phase == GamePhase.Start:
            self.current_phase = GamePhase.Draw
            self.phase_draw()
        elif self.current_phase == GamePhase.Draw:
            self.current_phase = GamePhase.Breeding
            self.phase_breeding()
        elif self.current_phase == GamePhase.Breeding:
            self.current_phase = GamePhase.Main
            self.phase_main()
        elif self.current_phase == GamePhase.Main:
            self.current_phase = GamePhase.End
            self.phase_end()
        elif self.current_phase == GamePhase.End:
            self.switch_turn()
            self.phase_start()

    def phase_start(self):
        self.current_phase = GamePhase.Start
        self.logger.log(f"=== Turn {self.turn_count} — {self.turn_player.player_name} ===")
        self.turn_player.unsuspend_all()
        self._reset_effect_turn_counts()
        self._clear_temp_dp()
        self.execute_effects(EffectTiming.OnStartTurn)

        if self.memory <= 0:
            self.memory = 3

        self.next_phase()

    def phase_draw(self):
        if self.turn_count == 1:
            pass  # First turn: no draw
        else:
            if not self.turn_player.draw():
                self.declare_winner(self.opponent_player)
                return
            self.execute_effects(EffectTiming.OnDraw)

        self.next_phase()

    def phase_breeding(self):
        self.logger.log_verbose("Phase: Breeding")
        pass  # Waiting for agent action

    def phase_main(self):
        self.logger.log_verbose("Phase: Main")
        self.execute_effects(EffectTiming.OnStartMainPhase)
        pass  # Waiting for agent actions

    def phase_end(self):
        self.execute_effects(EffectTiming.OnEndTurn)
        self.next_phase()

    def switch_turn(self):
        self.turn_player, self.opponent_player = self.opponent_player, self.turn_player
        self.turn_count += 1
        self.memory = -self.memory
        self.turn_player.is_my_turn = True
        self.opponent_player.is_my_turn = False

    def pass_turn(self):
        if self.memory >= 0:
            self.memory = -3
        self.current_phase = GamePhase.End
        self.next_phase()

    def check_turn_end(self):
        if self.memory < 0:
            self.current_phase = GamePhase.End
            self.next_phase()

    # ─── Effect Execution ────────────────────────────────────────────

    def execute_effects(self, timing: EffectTiming, extra_context: Optional[dict] = None):
        """Execute all effects matching the given timing across all permanents."""
        all_perms: List[Permanent] = list(self.turn_player.battle_area) + list(self.opponent_player.battle_area)

        for perm in all_perms:
            effects = perm.effect_list(timing)
            for effect in effects:
                if not effect.can_activate_this_turn():
                    continue

                owner = self._find_owner(perm)
                context = {
                    "game": self,
                    "player": owner,
                    "permanent": perm,
                    "card": effect.effect_source_card,
                    "turn_player": self.turn_player,
                    "opponent_player": self.opponent_player,
                }
                if extra_context:
                    context.update(extra_context)

                if effect.can_use_condition is None or effect.can_use_condition(context):
                    effect.record_activation()
                    if effect.on_process_callback:
                        effect.on_process_callback(context)

    def execute_deletion_effects(self, deleted_permanent: Permanent, owner: Player):
        """Execute OnDeletion effects for a permanent that was just deleted."""
        # Effects from the deleted permanent's card stack
        for source in deleted_permanent.card_sources:
            effects = source.effect_list(EffectTiming.OnDestroyedAnyone)
            for effect in effects:
                if not effect.can_activate_this_turn():
                    continue
                context = {
                    "game": self,
                    "player": owner,
                    "permanent": deleted_permanent,
                    "card": effect.effect_source_card,
                    "turn_player": self.turn_player,
                    "opponent_player": self.opponent_player,
                    "deleted_permanent": deleted_permanent,
                }
                if effect.can_use_condition is None or effect.can_use_condition(context):
                    effect.record_activation()
                    if effect.on_process_callback:
                        effect.on_process_callback(context)

    def _find_owner(self, perm: Permanent) -> Player:
        """Determine which player owns a permanent."""
        if perm.top_card and perm.top_card.owner:
            return perm.top_card.owner
        if perm in self.turn_player.battle_area:
            return self.turn_player
        if perm in self.opponent_player.battle_area:
            return self.opponent_player
        return self.turn_player

    def _reset_effect_turn_counts(self):
        """Reset once-per-turn counters for all effects at start of turn."""
        for player in [self.player1, self.player2]:
            for perm in player.battle_area:
                for source in perm.card_sources:
                    for effect in source.effect_list(EffectTiming.NoTiming):
                        effect.reset_turn_count()
            if player.breeding_area:
                for source in player.breeding_area.card_sources:
                    for effect in source.effect_list(EffectTiming.NoTiming):
                        effect.reset_turn_count()

    def _clear_temp_dp(self):
        """Clear temporary DP modifiers at start of turn."""
        for player in [self.player1, self.player2]:
            for perm in player.battle_area:
                perm.clear_temp_dp()

    # ─── Game Actions ────────────────────────────────────────────────

    def declare_winner(self, winner: Player):
        self.game_over = True
        self.winner = winner
        self.logger.log(f"Game Over! Winner: {winner.player_name}")

    def to_json(self) -> Dict[str, Any]:
        """Serialize game state to a dictionary (mirrors C# Game.ToJson)."""
        def player_data(p: Player) -> Dict[str, Any]:
            return {
                "Id": p.player_id,
                "Memory": self._get_memory_for(p),
                "HandCount": len(p.hand_cards),
                "HandIds": [c.card_id for c in p.hand_cards],
                "SecurityCount": len(p.security_cards),
                "DeckCount": len(p.library_cards),
                "BattleAreaCount": len(p.battle_area),
                "BattleArea": [
                    {
                        "TopCardId": perm.top_card.card_id if perm.top_card else None,
                        "TopCardName": perm.top_card.card_names[0] if perm.top_card and perm.top_card.card_names else None,
                        "DP": perm.dp,
                        "Level": perm.level,
                        "IsSuspended": perm.is_suspended,
                        "SourceCount": len(perm.card_sources),
                    }
                    for perm in p.battle_area
                ],
                "BreedingArea": {
                    "TopCardId": p.breeding_area.top_card.card_id if p.breeding_area and p.breeding_area.top_card else None,
                    "Level": p.breeding_area.level if p.breeding_area else None,
                } if p.breeding_area else None,
            }

        return {
            "TurnCount": self.turn_count,
            "CurrentPhase": self.current_phase.name,
            "CurrentPlayer": self.turn_player.player_id,
            "MemoryGauge": self.memory,
            "IsGameOver": self.game_over,
            "Winner": self.winner.player_id if self.winner else None,
            "Player1": player_data(self.player1),
            "Player2": player_data(self.player2),
        }

    def resolve_attack(self, attacker: Permanent, target: Union[Permanent, Player]):
        """Begin an attack sequence. May pause for BlockTiming/CounterTiming.

        When blockers or counter options exist, the game transitions to
        interrupt phases and returns control to the game loop. The decoders
        for those phases resume the attack flow.

        When no interrupts are available, the entire attack resolves
        synchronously in a single call (backward compatible).
        """
        if not attacker.can_attack():
            return

        target_name = target.player_name if isinstance(target, Player) else (target.top_card.card_names[0] if target.top_card else "Unknown")
        attacker_name = attacker.top_card.card_names[0] if attacker.top_card else "Unknown"
        self.logger.log(f"[Attack] {attacker_name} attacks {target_name}")
        attacker.suspend()

        # Trigger When Attacking (OnAllyAttack for the attacker's effects)
        self.execute_effects(EffectTiming.OnAllyAttack, {"attacker": attacker})

        # Store pending attack context
        self.pending_attack = PendingAttack(
            attacker=attacker,
            original_target=target,
            effective_target=target,
        )

        # Check if opponent has any potential blockers
        has_blockers = any(
            perm.can_block(attacker) for perm in self.opponent_player.battle_area
        )

        if has_blockers:
            # Transition to BlockTiming — opponent decides
            self.current_phase = GamePhase.BlockTiming
            self.active_player = self.opponent_player
            return  # Park here; _decode_block() will resume

        # No blockers — check for counter timing
        self._enter_counter_timing()

    def _enter_counter_timing(self):
        """Check for counter opportunities and enter CounterTiming if any exist."""
        has_counter = self._opponent_has_counter_options()

        if has_counter:
            self.current_phase = GamePhase.CounterTiming
            self.active_player = self.opponent_player
            return  # Park here; _decode_counter() will resume

        # No counter options — resolve battle immediately
        self._resolve_battle()

    def _opponent_has_counter_options(self) -> bool:
        """Check if the defending player has any valid blast digivolve options."""
        defender = self.opponent_player
        for card in defender.hand_cards:
            if not card.is_digimon:
                continue
            # Check if this card has blast digivolve
            effects = card.effect_list(EffectTiming.NoTiming)
            has_blast = any(getattr(e, '_is_blast_digivolve', False) for e in effects)
            if not has_blast:
                continue
            # Check if there's a valid target on field using proper evo_costs validation
            for perm in defender.battle_area:
                if can_digivolve(card, perm):
                    return True
        return False

    def _resolve_battle(self):
        """Execute the actual battle resolution after block/counter decisions."""
        pa = self.pending_attack
        if pa is None:
            return

        attacker = pa.attacker
        target = pa.effective_target

        # Clear interrupt state — back to turn_player control
        self.active_player = None
        self.pending_attack = None
        self.current_phase = GamePhase.Main

        if isinstance(target, Player):
            result = target.security_attack(attacker)
            if result == AttackResolution.AttackerDeleted:
                self.turn_player.delete_permanent(attacker)
            elif result == AttackResolution.GameEnd:
                self.declare_winner(self.turn_player)
        elif isinstance(target, Permanent):
            if attacker.dp > target.dp:
                self.opponent_player.delete_permanent(target)
            elif attacker.dp < target.dp:
                self.turn_player.delete_permanent(attacker)
            else:
                self.opponent_player.delete_permanent(target)
                self.turn_player.delete_permanent(attacker)

        self.execute_effects(EffectTiming.OnEndAttack)
        self.check_turn_end()

    def action_play_card(self, card_index: int):
        if self.current_phase != GamePhase.Main:
            return
        if card_index < 0 or card_index >= len(self.turn_player.hand_cards):
            return

        card = self.turn_player.hand_cards[card_index]
        cost = card.get_cost_itself

        self.logger.log(f"[Play] {self.turn_player.player_name} plays {card.card_names[0]} (cost: {cost})")
        self.turn_player.play_card(card)
        self.memory -= cost

        self.execute_effects(EffectTiming.OnEnterFieldAnyone, {"played_card": card})
        self.check_turn_end()

    def action_digivolve(self, permanent_index: int, card_index: int):
        if self.current_phase != GamePhase.Main:
            return
        if permanent_index >= len(self.turn_player.battle_area):
            return
        if card_index >= len(self.turn_player.hand_cards):
            return

        perm = self.turn_player.battle_area[permanent_index]
        card = self.turn_player.hand_cards[card_index]

        cost = self.turn_player.digivolve(perm, card)
        self.memory -= cost

        self.execute_effects(EffectTiming.WhenDigivolving, {"digivolved_permanent": perm})
        self.execute_effects(EffectTiming.OnEnterFieldAnyone, {"played_card": card})
        self.check_turn_end()

    def action_attack_player(self, attacker_index: int):
        if self.current_phase != GamePhase.Main:
            return
        if attacker_index < 0 or attacker_index >= len(self.turn_player.battle_area):
            return
        attacker = self.turn_player.battle_area[attacker_index]
        self.resolve_attack(attacker, self.opponent_player)

    def action_attack_digimon(self, attacker_index: int, target_index: int):
        """Attack an opponent's digimon (by field index)."""
        if self.current_phase != GamePhase.Main:
            return
        if attacker_index < 0 or attacker_index >= len(self.turn_player.battle_area):
            return
        if target_index < 0 or target_index >= len(self.opponent_player.battle_area):
            return
        attacker = self.turn_player.battle_area[attacker_index]
        target = self.opponent_player.battle_area[target_index]
        self.resolve_attack(attacker, target)

    def action_hatch(self):
        """Hatch from digitama deck into breeding area."""
        if self.current_phase != GamePhase.Breeding:
            return
        self.logger.log(f"[Hatch] {self.turn_player.player_name} hatches from egg deck")
        self.turn_player.hatch()

    def action_move_from_breeding(self):
        """Move breeding area digimon to battle area."""
        if self.current_phase != GamePhase.Breeding:
            return
        self.logger.log(f"[Move] {self.turn_player.player_name} moves from breeding to battle area")
        self.turn_player.move_from_breeding()

    def action_breeding_pass(self):
        """Skip breeding phase and advance to main."""
        if self.current_phase != GamePhase.Breeding:
            return
        self.logger.log_verbose(f"{self.turn_player.player_name} passes breeding phase")
        self.current_phase = GamePhase.Main
        self.phase_main()

    def action_pass_turn(self):
        self.logger.log(f"[Pass] {self.turn_player.player_name} passes turn (memory: {self.memory})")
        self.pass_turn()

    # ─── Board State Tensor ──────────────────────────────────────────

    def get_board_state_tensor(self, player_id: int) -> List[float]:
        """Build a 695-float tensor representing the board from player's perspective.

        Layout matches C# Digimon.Core.Game.GetBoardStateTensor exactly:
          [0-9]     Global data
          [10-249]  My battle area  (12 slots × 20)
          [250-489] Opp battle area (12 slots × 20)
          [490-509] My hand  (20)
          [510-529] Opp hand (20)
          [530-574] My trash (45)
          [575-619] Opp trash (45)
          [620-629] My security (10)
          [630-639] Opp security (10)
          [640-659] My breeding (1 slot × 20)
          [660-679] Opp breeding (1 slot × 20)
        """
        me = self.player1 if player_id == 1 else self.player2
        opp = self.player2 if player_id == 1 else self.player1

        t: List[float] = []

        # --- Global [0-9] ---
        t.append(float(self.turn_count))                            # 0
        t.append(float(self.current_phase.value))                   # 1
        t.append(float(self._get_memory_for(me)))                   # 2
        t.extend([0.0] * 7)                                        # 3-9 reserved

        # --- My field [10-249] ---
        self._append_field(t, me.battle_area, FIELD_SLOTS)

        # --- Opp field [250-489] ---
        self._append_field(t, opp.battle_area, FIELD_SLOTS)

        # --- My hand [490-509] ---
        self._append_card_ids(t, me.hand_cards, MAX_HAND)

        # --- Opp hand [510-529] ---
        self._append_card_ids(t, opp.hand_cards, MAX_HAND)

        # --- My trash [530-574] ---
        self._append_card_ids(t, me.trash_cards, MAX_TRASH)

        # --- Opp trash [575-619] ---
        self._append_card_ids(t, opp.trash_cards, MAX_TRASH)

        # --- My security [620-629] ---
        self._append_card_ids(t, me.security_cards, MAX_SECURITY)

        # --- Opp security [630-639] ---
        self._append_card_ids(t, opp.security_cards, MAX_SECURITY)

        # --- My breeding [640-659] ---
        breeding_list = [me.breeding_area] if me.breeding_area else []
        self._append_field(t, breeding_list, 1)

        # --- Opp breeding [660-679] ---
        opp_breeding_list = [opp.breeding_area] if opp.breeding_area else []
        self._append_field(t, opp_breeding_list, 1)

        # --- Revealed cards [680-689] ---
        self._append_card_ids(t, self.revealed_cards, MAX_REVEALED)

        # --- Selection context [690-694] ---
        # 690: selection phase type (0=none, 5=SelectTarget, 6=SelectMaterial, etc.)
        # 691: number of valid selections
        # 692: selecting player (0=none, 1=player1, 2=player2)
        # 693-694: reserved
        ps = self.pending_selection
        t.append(float(self.current_phase.value) if self.current_phase in (
            GamePhase.SelectTarget, GamePhase.SelectMaterial,
            GamePhase.SelectTrash, GamePhase.SelectSource,
            GamePhase.SelectHand, GamePhase.SelectReveal,
            GamePhase.SelectEffectChoice, GamePhase.SelectSecurity,
        ) else 0.0)
        t.append(float(len(ps.valid_indices)) if ps else 0.0)
        t.append(float(ps.selecting_player.player_id) if ps else 0.0)
        t.extend([0.0, 0.0])  # reserved

        return t

    def _get_memory_for(self, player: Player) -> int:
        """Memory relative to player (positive = their favour)."""
        if player is self.player1:
            return self.memory
        return -self.memory

    @staticmethod
    def _append_field(tensor: List[float], permanents: List[Permanent], slots: int):
        """Append field slot data: per slot 20 floats."""
        for i in range(slots):
            if i < len(permanents):
                perm = permanents[i]
                top = perm.top_card
                # +0: top card internal ID
                tensor.append(float(CardRegistry.get_id(top.card_id) if top else 0))
                # +1: current DP
                tensor.append(float(perm.dp))
                # +2: suspended
                tensor.append(1.0 if perm.is_suspended else 0.0)
                # +3: OPT exhausted (1.0 if all once-per-turn effects used)
                tensor.append(1.0 if perm.opt_exhausted else 0.0)
                # +4: linked card count (option cards attached sideways, e.g. [TS])
                tensor.append(float(len(perm.linked_cards)))
                # +5: source count
                tensor.append(float(len(perm.card_sources)))
                # +6-19: source card IDs (bottom to top, max 14)
                for j in range(MAX_SOURCES):
                    if j < len(perm.card_sources):
                        src = perm.card_sources[j]
                        tensor.append(float(CardRegistry.get_id(src.card_id)))
                    else:
                        tensor.append(0.0)
            else:
                tensor.extend([0.0] * SLOT_SIZE)

    @staticmethod
    def _append_card_ids(tensor: List[float], cards: list, limit: int):
        """Append card ID list padded to limit."""
        for i in range(limit):
            if i < len(cards):
                tensor.append(float(CardRegistry.get_id(cards[i].card_id)))
            else:
                tensor.append(0.0)

    # ─── Action Mask ─────────────────────────────────────────────────

    def get_action_mask(self, player_id: int) -> List[float]:
        """Build a 2120-float mask (1.0 = valid, 0.0 = invalid).

        Ranges match C# Digimon.Core.ActionDecoder:
          0-29:      Play card from hand
          30-59:     Trash card from hand (effect-driven)
          60:        Hatch
          61:        Move from breeding
          62:        Pass / end turn
          100-399:   Attack (100 + attacker*15 + target, target 12 = security)
          400-999:   Digivolve (400 + hand*15 + field)
          1000-1999: Activate effect (1000 + source*10 + effectIdx)
          2000-2119: Source selection (2000 + field*10 + sourceIdx)
        """
        mask = [0.0] * ACTION_SPACE_SIZE
        me = self.player1 if player_id == 1 else self.player2
        opp = self.player2 if player_id == 1 else self.player1

        phase = self.current_phase

        if phase == GamePhase.Main:
            # Play cards (0-29)
            for i in range(min(len(me.hand_cards), 30)):
                card = me.hand_cards[i]
                if card.get_cost_itself <= self.memory:
                    mask[i] = 1.0

            # Attack (100-399): 100 + attacker*15 + target
            for i in range(min(len(me.battle_area), FIELD_SLOTS)):
                attacker = me.battle_area[i]
                if attacker.is_suspended:
                    continue
                if not attacker.is_digimon:
                    continue
                # Security attack (target index 12)
                mask[100 + i * 15 + 12] = 1.0
                # Digimon attacks (targets 0-11, suspended only per rules)
                for j in range(min(len(opp.battle_area), FIELD_SLOTS)):
                    target = opp.battle_area[j]
                    if target.is_suspended:
                        mask[100 + i * 15 + j] = 1.0

            # Digivolve (400-999): 400 + hand*15 + field
            for h in range(min(len(me.hand_cards), 30)):
                card = me.hand_cards[h]
                if not card.is_digimon:
                    continue
                for f in range(min(len(me.battle_area), FIELD_SLOTS)):
                    base_perm = me.battle_area[f]
                    if can_digivolve(card, base_perm):
                        mask[400 + h * 15 + f] = 1.0

            # DNA Digivolve (63-92): 63 + hand_idx
            for h in range(min(len(me.hand_cards), 30)):
                card = me.hand_cards[h]
                if not card.is_digimon:
                    continue
                if has_valid_dna_targets(card, me.battle_area):
                    mask[63 + h] = 1.0

            # Pass (62) - always valid in main
            mask[62] = 1.0

        elif phase == GamePhase.Breeding:
            # Hatch (60)
            if me.breeding_area is None and me.digitama_library_cards:
                mask[60] = 1.0
            # Move (61)
            if me.breeding_area is not None and me.breeding_area.level >= 3:
                mask[61] = 1.0
            # Pass (62)
            mask[62] = 1.0

        elif phase == GamePhase.BlockTiming:
            mask[62] = 1.0  # always can decline
            attacker = self.pending_attack.attacker if self.pending_attack else Permanent([])
            for i in range(min(len(me.battle_area), FIELD_SLOTS)):
                perm = me.battle_area[i]
                if perm.can_block(attacker):
                    mask[100 + i] = 1.0

        elif phase == GamePhase.CounterTiming:
            mask[62] = 1.0  # always can decline
            # Blast digivolve options (400-999): 400 + hand*15 + field
            for h in range(min(len(me.hand_cards), 30)):
                card = me.hand_cards[h]
                if not card.is_digimon:
                    continue
                # Check for blast digivolve effect
                effects = card.effect_list(EffectTiming.NoTiming)
                has_blast = any(getattr(e, '_is_blast_digivolve', False) for e in effects)
                if not has_blast:
                    continue
                # Check valid field targets using proper evo_costs validation
                for f in range(min(len(me.battle_area), FIELD_SLOTS)):
                    base_perm = me.battle_area[f]
                    if can_digivolve(card, base_perm):
                        mask[400 + h * 15 + f] = 1.0

        elif phase == GamePhase.SelectTrash:
            for i in range(min(len(me.trash_cards), 60)):
                mask[i] = 1.0

        elif phase == GamePhase.SelectSource:
            # Source selection (2000-2119): 2000 + field*10 + sourceIdx
            for f in range(min(len(me.battle_area), FIELD_SLOTS)):
                perm = me.battle_area[f]
                for s in range(min(len(perm.card_sources), 10)):
                    mask[2000 + f * 10 + s] = 1.0

        elif phase in (GamePhase.SelectTarget, GamePhase.SelectMaterial,
                       GamePhase.SelectHand, GamePhase.SelectReveal,
                       GamePhase.SelectEffectChoice, GamePhase.SelectSecurity):
            # Generic selection: use valid_indices from pending selection
            ps = self.pending_selection
            if ps and ps.valid_indices:
                for idx in ps.valid_indices:
                    if 0 <= idx < ACTION_SPACE_SIZE:
                        mask[idx] = 1.0
            # Optional selections allow declining (pass)
            if ps and getattr(ps, 'is_optional', False):
                mask[62] = 1.0

        return mask

    # ─── Action Decoder ──────────────────────────────────────────────

    def decode_action(self, action_id: int, player_id: int):
        """Decode an integer action and execute the corresponding game action.

        Mirrors C# Digimon.Core.ActionDecoder.DecodeAndExecute.
        """
        phase = self.current_phase

        if phase == GamePhase.Main:
            self._decode_main(action_id)
        elif phase == GamePhase.Breeding:
            self._decode_breeding(action_id)
        elif phase in (GamePhase.SelectTarget, GamePhase.SelectMaterial,
                       GamePhase.SelectHand, GamePhase.SelectReveal,
                       GamePhase.SelectEffectChoice, GamePhase.SelectSecurity):
            self._decode_selection(action_id)
        elif phase == GamePhase.BlockTiming:
            self._decode_block(action_id)
        elif phase == GamePhase.CounterTiming:
            self._decode_counter(action_id)
        elif phase == GamePhase.SelectTrash:
            self._decode_trash_selection(action_id)
        elif phase == GamePhase.SelectSource:
            self._decode_source_selection(action_id)

    def _decode_main(self, action_id: int):
        if 0 <= action_id <= 29:
            self.action_play_card(action_id)
        elif action_id == 62:
            self.action_pass_turn()
        elif 100 <= action_id <= 399:
            normalized = action_id - 100
            attacker_idx = normalized // 15
            target_idx = normalized % 15
            if target_idx == 12:
                self.action_attack_player(attacker_idx)
            else:
                self.action_attack_digimon(attacker_idx, target_idx)
        elif 63 <= action_id <= 92:
            hand_idx = action_id - 63
            self._initiate_dna_digivolve(hand_idx)
        elif 400 <= action_id <= 999:
            normalized = action_id - 400
            hand_idx = normalized // 15
            field_idx = normalized % 15
            self.action_digivolve(field_idx, hand_idx)

    def _decode_breeding(self, action_id: int):
        if action_id == 60:
            self.action_hatch()
        elif action_id == 61:
            self.action_move_from_breeding()
        elif action_id == 62:
            self.action_breeding_pass()

    def _decode_selection(self, action_id: int):
        """Handle target or material selection from an effect callback."""
        ps = self.pending_selection
        if ps is None:
            return

        # Optional selection: action 62 = decline/pass
        if action_id == 62 and getattr(ps, 'is_optional', False):
            prev_phase = ps.previous_phase
            self.pending_selection = None
            self.revealed_cards = []  # clear any revealed cards
            self.current_phase = prev_phase
            self.active_player = None
            return

        if ps.valid_indices and action_id not in ps.valid_indices:
            return  # invalid selection, ignore

        callback = ps.callback
        prev_phase = ps.previous_phase
        self.pending_selection = None
        self.current_phase = prev_phase
        self.active_player = None
        callback(action_id)

    def _decode_block(self, action_id: int):
        """Handle the defender's blocking decision during an attack."""
        pa = self.pending_attack
        if pa is None:
            return

        if action_id == 62:
            # Decline to block — proceed to counter timing
            self.logger.log("[Block] Declined to block")
            self._enter_counter_timing()

        elif 100 <= action_id <= 111:
            blocker_idx = action_id - 100
            defender = self.opponent_player

            if blocker_idx >= len(defender.battle_area):
                return

            blocker = defender.battle_area[blocker_idx]
            if not blocker.can_block(pa.attacker):
                return

            # Suspend the blocker and redirect the attack
            blocker.suspend()
            pa.is_blocked = True
            pa.blocker = blocker
            pa.effective_target = blocker

            blocker_name = blocker.top_card.card_names[0] if blocker.top_card else "Unknown"
            self.logger.log(f"[Block] {blocker_name} blocks the attack")

            # Fire block-related effects
            self.execute_effects(EffectTiming.OnBlockAnyone, {"blocker": blocker})
            self.execute_effects(EffectTiming.OnEndBlockDesignation, {"blocker": blocker})

            # Proceed to counter timing
            self._enter_counter_timing()

    def _decode_counter(self, action_id: int):
        """Handle the defender's counter/blast digivolve decision."""
        pa = self.pending_attack
        if pa is None:
            return

        if action_id == 62:
            # Decline counter — resolve battle
            self.logger.log("[Counter] Declined counter")
            self._resolve_battle()

        elif 400 <= action_id <= 999:
            normalized = action_id - 400
            hand_idx = normalized // 15
            field_idx = normalized % 15

            defender = self.opponent_player

            if hand_idx >= len(defender.hand_cards):
                self._resolve_battle()
                return
            if field_idx >= len(defender.battle_area):
                self._resolve_battle()
                return

            card = defender.hand_cards[hand_idx]
            perm = defender.battle_area[field_idx]

            # Validate blast digivolve effect exists on card
            effects = card.effect_list(EffectTiming.NoTiming)
            has_blast = any(getattr(e, '_is_blast_digivolve', False) for e in effects)
            if not has_blast:
                self._resolve_battle()
                return

            # Execute blast digivolve (free cost — no memory change)
            card_name = card.card_names[0] if card.card_names else "Unknown"
            target_name = perm.top_card.card_names[0] if perm.top_card else "Unknown"
            self.logger.log(f"[Counter] Blast Digivolve: {card_name} onto {target_name}")

            defender.hand_cards.remove(card)
            perm.add_card_source(card)

            # Fire digivolution-related effects
            self.execute_effects(EffectTiming.OnCounterTiming, {"counter_card": card, "counter_permanent": perm})
            self.execute_effects(EffectTiming.WhenDigivolving, {"digivolved_permanent": perm})
            self.execute_effects(EffectTiming.OnEnterFieldAnyone, {"played_card": card})

            # Resolve battle (DP has changed due to digivolution)
            self._resolve_battle()

    def _decode_trash_selection(self, action_id: int):
        """Handle trash card selection from an effect callback."""
        ps = self.pending_selection
        if ps is None:
            return

        if 0 <= action_id <= 59:
            selecting = ps.selecting_player
            if action_id < len(selecting.trash_cards):
                callback = ps.callback
                prev_phase = ps.previous_phase
                self.pending_selection = None
                self.current_phase = prev_phase
                self.active_player = None
                callback(action_id)

    def _decode_source_selection(self, action_id: int):
        """Handle digivolution source selection from an effect callback."""
        ps = self.pending_selection
        if ps is None:
            return

        if 2000 <= action_id <= 2119:
            normalized = action_id - 2000
            field_idx = normalized // 10
            source_idx = normalized % 10

            selecting = ps.selecting_player
            if field_idx < len(selecting.battle_area):
                perm = selecting.battle_area[field_idx]
                if source_idx < len(perm.card_sources):
                    callback = ps.callback
                    prev_phase = ps.previous_phase
                    self.pending_selection = None
                    self.current_phase = prev_phase
                    self.active_player = None
                    callback(action_id)

    # ─── Selection Request Helper ─────────────────────────────────

    def request_selection(self, phase: GamePhase, player: Player,
                          callback: Callable[[int], None],
                          valid_indices: Optional[List[int]] = None,
                          is_optional: bool = False):
        """Pause the game to request a selection from the given player.

        Used by effect callbacks that need player input (target, trash, source).
        The game transitions to the specified phase and parks until the
        player's agent provides a selection via decode_action().

        Args:
            phase: The GamePhase to transition to during selection.
            player: The player who must make the selection.
            callback: Called with the selected action_id when chosen.
            valid_indices: List of valid action IDs the player can choose from.
            is_optional: If True, the player can decline (action 62 = pass).
        """
        self.pending_selection = PendingSelection(
            callback=callback,
            selecting_player=player,
            previous_phase=self.current_phase,
            valid_indices=valid_indices or [],
            is_optional=is_optional,
        )
        self.current_phase = phase
        self.active_player = player

    # ─── Effect Helper Methods ─────────────────────────────────────────
    # These helpers implement common effect patterns that card scripts use.
    # They handle selection conventions, tensor integration, and callbacks.

    def effect_select_opponent_permanent(
        self, player: Player, callback: Callable[['Permanent'], None],
        filter_fn: Optional[Callable[['Permanent'], bool]] = None,
        is_optional: bool = False,
    ):
        """Request selection of an opponent's permanent (for delete, suspend, bounce, etc.).

        Covers: delete_opponent (34 cards), suspend_target (19), return_bounce (15),
        de_digivolve (13) = 81 cards total.

        Args:
            player: The player whose effect is triggering.
            callback: Called with the selected Permanent.
            filter_fn: Optional filter (e.g. lambda p: p.dp <= 5000).
            is_optional: If True, player can decline.
        """
        opponent = self.player2 if player is self.player1 else self.player1
        valid = []
        for i, perm in enumerate(opponent.battle_area):
            if filter_fn is None or filter_fn(perm):
                valid.append(SEL_OPP_FIELD_START + i)
        if not valid:
            return

        def on_select(action_id: int):
            idx = action_id - SEL_OPP_FIELD_START
            opp = self.player2 if player is self.player1 else self.player1
            if 0 <= idx < len(opp.battle_area):
                callback(opp.battle_area[idx])

        self.request_selection(
            GamePhase.SelectTarget, player, on_select, valid, is_optional)

    def effect_select_own_permanent(
        self, player: Player, callback: Callable[['Permanent'], None],
        filter_fn: Optional[Callable[['Permanent'], bool]] = None,
        is_optional: bool = False,
    ):
        """Request selection of one of the player's own permanents.

        Covers: mind_link (3 cards), save (2), sacrifice_cost (12) = 17 cards.

        Args:
            player: The player making the selection.
            callback: Called with the selected Permanent.
            filter_fn: Optional filter (e.g. lambda p: 'SoC' in p.traits).
            is_optional: If True, player can decline.
        """
        valid = []
        for i, perm in enumerate(player.battle_area):
            if filter_fn is None or filter_fn(perm):
                valid.append(SEL_MY_FIELD_START + i)
        if not valid:
            return

        def on_select(action_id: int):
            idx = action_id - SEL_MY_FIELD_START
            if 0 <= idx < len(player.battle_area):
                callback(player.battle_area[idx])

        self.request_selection(
            GamePhase.SelectTarget, player, on_select, valid, is_optional)

    def effect_reveal_and_select(
        self, player: Player, count: int,
        filter_fn: Callable[['CardSource'], bool],
        on_selected: Callable[['CardSource', List['CardSource']], None],
        is_optional: bool = False,
    ):
        """Reveal top N cards, let agent pick one matching filter, return rest to bottom.

        Covers: reveal_top (28 cards).

        Args:
            player: The player revealing cards.
            count: Number of cards to reveal.
            filter_fn: Which revealed cards are valid picks.
            on_selected: Called with (selected_card, remaining_cards).
            is_optional: If True, player can decline (all go to bottom).
        """
        revealed = player.library_cards[:count]
        if not revealed:
            return

        # Store in game state so tensor can see them
        self.revealed_cards = list(revealed)

        valid = []
        for i, card in enumerate(revealed):
            if filter_fn(card):
                valid.append(SEL_REVEALED_START + i)
        if not valid:
            # No valid picks — return all to bottom
            for card in revealed:
                player.library_cards.remove(card)
                player.library_cards.append(card)
            self.revealed_cards = []
            return

        def on_select(action_id: int):
            idx = action_id - SEL_REVEALED_START
            if 0 <= idx < len(revealed):
                selected = revealed[idx]
                remaining = [c for c in revealed if c is not selected]
                # Remove all revealed from library top
                for c in revealed:
                    if c in player.library_cards:
                        player.library_cards.remove(c)
                on_selected(selected, remaining)
            self.revealed_cards = []

        self.request_selection(
            GamePhase.SelectReveal, player, on_select, valid, is_optional)

    def effect_play_from_zone(
        self, player: Player,
        zone: str,
        filter_fn: Callable[['CardSource'], bool],
        free: bool = True,
        is_optional: bool = True,
    ):
        """Let agent pick a card from a zone to play onto the field.

        Covers: play (85 cards).

        Args:
            player: The player whose effect triggers.
            zone: 'hand', 'trash', or 'revealed' (which zone to pick from).
            filter_fn: Which cards in the zone are valid.
            free: If True, play without paying cost.
            is_optional: If True, player can decline.
        """
        if zone == 'hand':
            source_list = player.hand_cards
            offset = SEL_HAND_START
        elif zone == 'trash':
            source_list = player.trash_cards
            offset = SEL_HAND_START  # reuse 0-29 for trash in SelectTarget
        elif zone == 'revealed':
            source_list = list(self.revealed_cards)
            offset = SEL_REVEALED_START
        else:
            return

        valid = []
        for i, card in enumerate(source_list):
            if filter_fn(card) and (offset + i) < ACTION_SPACE_SIZE:
                valid.append(offset + i)
        if not valid:
            return

        def on_select(action_id: int):
            idx = action_id - offset
            if 0 <= idx < len(source_list):
                card = source_list[idx]
                player.play_card_from_source(card, pay_cost=not free)
                self.logger.log(f"[Effect] {player.player_name} played "
                                f"{card.card_names[0]} from {zone}")
                self.execute_effects(EffectTiming.OnEnterFieldAnyone,
                                     {"played_card": card})

        phase = GamePhase.SelectReveal if zone == 'revealed' else GamePhase.SelectTarget
        self.request_selection(phase, player, on_select, valid, is_optional)

    def effect_digivolve_from_hand(
        self, player: Player, permanent: 'Permanent',
        filter_fn: Callable[['CardSource'], bool],
        cost_override: Optional[int] = None,
        cost_reduction: int = 0,
        ignore_requirements: bool = False,
        is_optional: bool = True,
    ):
        """Let agent pick a hand card to digivolve a permanent into via effect.

        Covers: digivolve_into (52 cards).

        Args:
            player: The card owner.
            permanent: The permanent to digivolve.
            filter_fn: Which hand cards are valid digivolution targets.
            cost_override: Fixed cost (None = use card's evo cost).
            cost_reduction: Reduce normal evo cost by this amount.
            ignore_requirements: If True, skip level/color requirements.
            is_optional: If True, player can decline.
        """
        valid = []
        for i, card in enumerate(player.hand_cards):
            if filter_fn(card):
                valid.append(SEL_HAND_START + i)
        if not valid:
            return

        def on_select(action_id: int):
            idx = action_id - SEL_HAND_START
            if 0 <= idx < len(player.hand_cards):
                card = player.hand_cards[idx]
                if cost_override is not None:
                    cost = cost_override
                else:
                    base = card.get_cost_itself
                    cost = max(0, base - cost_reduction)
                # Stack card onto permanent
                player.hand_cards.remove(card)
                permanent.add_card_source(card)
                self.memory -= cost
                self.logger.log(
                    f"[Effect Digivolve] {card.card_names[0]} onto "
                    f"{permanent.top_card.card_names[0] if permanent.top_card else 'Unknown'} "
                    f"(cost: {cost})")
                # Draw 1 (digivolution bonus)
                player.draw()
                self.execute_effects(EffectTiming.WhenDigivolving,
                                     {"digivolved_permanent": permanent})
                self.execute_effects(EffectTiming.OnEnterFieldAnyone,
                                     {"played_card": card})

        self.request_selection(
            GamePhase.SelectTarget, player, on_select, valid, is_optional)

    def effect_select_hand_card(
        self, player: Player,
        filter_fn: Callable[['CardSource'], bool],
        callback: Callable[['CardSource'], None],
        is_optional: bool = False,
    ):
        """Let agent pick a card from hand (for trash-as-cost, discard, etc.).

        Covers: trash_selection (19 cards), general hand picks.

        Args:
            player: The player selecting.
            filter_fn: Which hand cards are valid.
            callback: Called with the selected CardSource.
            is_optional: If True, player can decline.
        """
        valid = []
        for i, card in enumerate(player.hand_cards):
            if filter_fn(card):
                valid.append(SEL_HAND_START + i)
        if not valid:
            return

        def on_select(action_id: int):
            idx = action_id - SEL_HAND_START
            if 0 <= idx < len(player.hand_cards):
                callback(player.hand_cards[idx])

        self.request_selection(
            GamePhase.SelectHand, player, on_select, valid, is_optional)

    def effect_choose_branch(
        self, player: Player, num_choices: int,
        callback: Callable[[int], None],
    ):
        """Let agent choose between N effect branches ("activate 1 of the effects below").

        Covers: multi_choice (2 cards).

        Args:
            player: The player choosing.
            num_choices: Number of branches (typically 2-3).
            callback: Called with the branch index (0-based).
        """
        valid = [SEL_EFFECT_CHOICE_START + i for i in range(num_choices)]

        def on_select(action_id: int):
            branch = action_id - SEL_EFFECT_CHOICE_START
            if 0 <= branch < num_choices:
                callback(branch)

        self.request_selection(
            GamePhase.SelectEffectChoice, player, on_select, valid)

    def effect_select_own_security(
        self, player: Player,
        filter_fn: Callable[['CardSource'], bool],
        callback: Callable[['CardSource'], None],
        is_optional: bool = True,
    ):
        """Let agent select a card from their own security stack.

        Covers: search_security (BT14-033 Patamon, BT14-093 Emissary of Hope),
        barrier cost, and effects that interact with own security.

        Args:
            player: The player whose security stack is searched.
            filter_fn: Which security cards are valid selections.
            callback: Called with the selected CardSource.
            is_optional: If True, player can decline.
        """
        valid = []
        for i, card in enumerate(player.security_cards):
            if filter_fn(card):
                valid.append(SEL_MY_SECURITY_START + i)
        if not valid:
            return

        def on_select(action_id: int):
            idx = action_id - SEL_MY_SECURITY_START
            if 0 <= idx < len(player.security_cards):
                callback(player.security_cards[idx])

        self.request_selection(
            GamePhase.SelectSecurity, player, on_select, valid, is_optional)

    def effect_select_opponent_security(
        self, player: Player,
        filter_fn: Optional[Callable[['CardSource'], bool]],
        callback: Callable[['CardSource'], None],
        is_optional: bool = True,
    ):
        """Let agent select a card from the opponent's security stack.

        Covers: BT24-018 Styracomon (trash opponent security), and effects
        that interact with opponent security cards.

        Args:
            player: The player whose effect is triggering.
            filter_fn: Optional filter on opponent's security cards.
            callback: Called with the selected CardSource.
            is_optional: If True, player can decline.
        """
        opponent = self.player2 if player is self.player1 else self.player1
        valid = []
        for i, card in enumerate(opponent.security_cards):
            if filter_fn is None or filter_fn(card):
                valid.append(SEL_OPP_SECURITY_START + i)
        if not valid:
            return

        def on_select(action_id: int):
            idx = action_id - SEL_OPP_SECURITY_START
            opp = self.player2 if player is self.player1 else self.player1
            if 0 <= idx < len(opp.security_cards):
                callback(opp.security_cards[idx])

        self.request_selection(
            GamePhase.SelectSecurity, player, on_select, valid, is_optional)

    def effect_link_to_permanent(
        self, player: Player, card_to_link: 'CardSource',
        filter_fn: Optional[Callable[['Permanent'], bool]] = None,
        is_optional: bool = True,
    ):
        """Let agent choose a Digimon to link an option card to (sideways attach).

        Covers: [TS] option cards (BT24-091, 092, 095, 097) that link after
        their security/main effect resolves. Includes battle area and breeding
        area targets.

        Restrictions: Cannot link to tokens or eggs (level <= 2 in breeding).

        Args:
            player: The player whose effect is triggering.
            card_to_link: The option card to link.
            filter_fn: Optional additional filter on target permanents.
            is_optional: If True, player can decline (default True).
        """
        valid = []

        # Battle area targets (100-111): exclude tokens
        for i, perm in enumerate(player.battle_area):
            if perm.is_token:
                continue
            if not perm.is_digimon:
                continue
            if filter_fn is not None and not filter_fn(perm):
                continue
            valid.append(SEL_MY_FIELD_START + i)

        # Breeding area target (99): must be a Digimon, not an egg (level > 2)
        ba = player.breeding_area
        if ba is not None and ba.is_digimon and ba.level > 2:
            if filter_fn is None or filter_fn(ba):
                valid.append(SEL_MY_BREEDING)

        if not valid:
            return

        def on_select(action_id: int):
            if action_id == SEL_MY_BREEDING:
                target = player.breeding_area
            else:
                idx = action_id - SEL_MY_FIELD_START
                if 0 <= idx < len(player.battle_area):
                    target = player.battle_area[idx]
                else:
                    return
            if target is None:
                return
            target.link_card(card_to_link)
            self.logger.log(
                f"[Link] {card_to_link.card_names[0]} linked to "
                f"{target.top_card.card_names[0] if target.top_card else 'Unknown'}")

        self.request_selection(
            GamePhase.SelectTarget, player, on_select, valid, is_optional)

    # ─── DNA Digivolve ────────────────────────────────────────────────

    def _initiate_dna_digivolve(self, hand_idx: int):
        """Start DNA digivolve: enter SelectMaterial to pick first field target."""
        if self.current_phase != GamePhase.Main:
            return
        if hand_idx >= len(self.turn_player.hand_cards):
            return

        card = self.turn_player.hand_cards[hand_idx]
        if not card.is_digimon or not card.c_entity_base or not card.c_entity_base.dna_costs:
            return

        valid_first = get_valid_dna_first_targets(card, self.turn_player.battle_area)
        if not valid_first:
            return

        self.request_selection(
            GamePhase.SelectMaterial,
            self.turn_player,
            lambda first_idx: self._dna_select_second(hand_idx, first_idx),
            valid_indices=valid_first,
        )

    def _dna_select_second(self, hand_idx: int, first_field_idx: int):
        """DNA digivolve step 2: select second field target."""
        if hand_idx >= len(self.turn_player.hand_cards):
            return
        if first_field_idx >= len(self.turn_player.battle_area):
            return

        card = self.turn_player.hand_cards[hand_idx]
        valid_second = get_valid_dna_second_targets(
            card, first_field_idx, self.turn_player.battle_area,
        )
        if not valid_second:
            return

        self.request_selection(
            GamePhase.SelectMaterial,
            self.turn_player,
            lambda second_idx: self._execute_dna_digivolve(
                hand_idx, first_field_idx, second_idx,
            ),
            valid_indices=valid_second,
        )

    def _execute_dna_digivolve(self, hand_idx: int, first_field_idx: int,
                                second_field_idx: int):
        """Execute the actual DNA digivolve after both targets are selected."""
        player = self.turn_player
        if hand_idx >= len(player.hand_cards):
            return
        if first_field_idx >= len(player.battle_area):
            return
        if second_field_idx >= len(player.battle_area):
            return

        card = player.hand_cards[hand_idx]
        perm1 = player.battle_area[first_field_idx]
        perm2 = player.battle_area[second_field_idx]

        stacking = get_dna_stacking_order(card, perm1, perm2)
        if stacking is None:
            return

        top_perm, bottom_perm, dna_cost = stacking

        top_name = top_perm.top_card.card_names[0] if top_perm.top_card else "Unknown"
        bottom_name = bottom_perm.top_card.card_names[0] if bottom_perm.top_card else "Unknown"
        card_name = card.card_names[0] if card.card_names else "Unknown"
        self.logger.log(
            f"[DNA Digivolve] {card_name} from "
            f"{top_name} + {bottom_name} (cost: {dna_cost.memory_cost})"
        )

        cost = player.dna_digivolve(top_perm, bottom_perm, card, dna_cost)
        self.memory -= cost

        # Find the new permanent (last in battle area after dna_digivolve)
        new_perm = player.battle_area[-1] if player.battle_area else None

        self.execute_effects(EffectTiming.WhenDigivolving, {"digivolved_permanent": new_perm})
        self.execute_effects(EffectTiming.OnEnterFieldAnyone, {"played_card": card})
        self.check_turn_end()
