from __future__ import annotations
from typing import TYPE_CHECKING, Optional, Union, List, Dict, Any, Callable
from dataclasses import dataclass, field
import random

import numpy as np

from digimon_gym.engine.data.enums import GamePhase, EffectTiming, AttackResolution, PendingAction
from digimon_gym.engine.core.player import Player
from digimon_gym.engine.core.permanent import Permanent
from digimon_gym.engine.data.card_registry import CardRegistry
from digimon_gym.engine.loggers import IGameLogger, SilentLogger
from digimon_gym.engine.events import GameEvent
from digimon_gym.engine.interfaces.modifiers import ModifierRegistry, ModifierType, ModifierEntry
from digimon_gym.engine.validation.digivolve_validator import (
    can_digivolve, has_valid_dna_targets, get_valid_dna_first_targets,
    get_valid_dna_second_targets, get_dna_stacking_order,
)

if TYPE_CHECKING:
    from .core.card_source import CardSource

# ─── Tensor / Action Space Constants (match C# Digimon.Core) ────────
FIELD_SLOTS = 14
MAX_HAND = 20
MAX_TRASH = 45
MAX_SECURITY = 10
MAX_SOURCES = 11
MAX_REVEALED = 10

# Named slot indices for action formulas (breeding/security share index = FIELD_SLOTS)
BREEDING_SLOT = FIELD_SLOTS       # 14 — virtual field index for breeding area
SECURITY_TARGET = FIELD_SLOTS     # 14 — attack target index for security attack

# Per-slot layout: 1 (card_id) + 6 scalars + MAX_SOURCES * 3 (card_id + opt_state + dp_contribution)
SOURCE_ENTRY_SIZE = 3             # card_id + opt_state + dp_contribution
SLOT_SIZE = 1 + 6 + MAX_SOURCES * SOURCE_ENTRY_SIZE  # 1 + 6 + 11*3 = 40

# Action space strides
TARGETS_PER_ATTACKER = 15         # 0..FIELD_SLOTS-1 = opp field, FIELD_SLOTS = security
FIELDS_PER_HAND = 15              # 0..FIELD_SLOTS-1 = battle area, FIELD_SLOTS = breeding
EFFECTS_PER_PERM = 10             # effect sub-indices per permanent
SOURCES_PER_FIELD = 12            # source sub-indices per field slot (accommodates MAX_SOURCES=11)
DP_NORM = 30000.0                 # DP normalization factor (covers rare ~30k buffed archetypes)

# Action space size: max source action = 2000 + (FIELD_SLOTS-1)*SOURCES_PER_FIELD + (SOURCES_PER_FIELD-1)
ACTION_SPACE_SIZE = 2000 + FIELD_SLOTS * SOURCES_PER_FIELD  # 2168

# Tensor layout offsets (compact: card IDs are single integer indices, not embeddings)
_GLOBAL = 10
_MY_BATTLE = FIELD_SLOTS * SLOT_SIZE      # 560
_OPP_BATTLE = FIELD_SLOTS * SLOT_SIZE     # 560
_MY_HAND = MAX_HAND                       # 20
_OPP_HAND = MAX_HAND                      # 20
_MY_TRASH = MAX_TRASH                     # 45
_OPP_TRASH = MAX_TRASH                    # 45
_MY_SECURITY = MAX_SECURITY               # 10
_OPP_SECURITY = MAX_SECURITY              # 10
_MY_BREEDING = 1 * SLOT_SIZE              # 40
_OPP_BREEDING = 1 * SLOT_SIZE             # 40
_REVEALED = MAX_REVEALED                  # 10
_SELECTION = 5

TENSOR_SIZE = (_GLOBAL + _MY_BATTLE + _OPP_BATTLE + _MY_HAND + _OPP_HAND +
               _MY_TRASH + _OPP_TRASH + _MY_SECURITY + _OPP_SECURITY +
               _MY_BREEDING + _OPP_BREEDING + _REVEALED + _SELECTION)  # 1375

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
SEL_MY_FIELD_START = 100   # 100-113:  select own battle_area permanent
SEL_MY_FIELD_END = 100 + FIELD_SLOTS - 1   # 113
SEL_OPP_FIELD_START = 100 + FIELD_SLOTS     # 114:  select opponent's battle_area permanent
SEL_OPP_FIELD_END = 100 + 2 * FIELD_SLOTS - 1  # 127
SEL_TRASH_START = 130      # 130-179:  select trash card by index (up to 50)
SEL_TRASH_END = 179
SEL_EFFECT_CHOICE_START = 1000  # 1000-1009: choose between effect branches
SEL_EFFECT_CHOICE_END = 1009


@dataclass
class TriggeredEffect:
    """An effect that has been collected for stack-based resolution.

    Mirrors DCGO's SkillInfo — captures the effect, its owning permanent,
    and the context at the moment it triggered so resolution can proceed
    even if the board state has since changed.
    """
    effect: Any  # ICardEffect
    permanent: Permanent  # permanent that owns this effect
    owner: Any  # Player who owns the permanent
    context: Dict[str, Any]  # full context dict for on_process_callback
    is_turn_player: bool = False  # whether owner is the current turn player


@dataclass
class PendingAttack:
    """Context for an attack in progress (paused for block/counter decisions)."""
    attacker: Permanent
    original_target: Union[Permanent, Player]
    effective_target: Union[Permanent, Player]  # changes if blocked
    is_blocked: bool = False
    blocker: Optional[Permanent] = None
    without_suspend: bool = False  # Overclock: attack without suspending
    is_vortex: bool = False  # Vortex: end-of-turn Digimon-only attack
    return_phase: Optional[GamePhase] = None  # Phase to return to after resolution (e.g. EndOfTurnAction)
    is_end_attack: bool = False  # DCGO: effects can force-end the attack early


@dataclass
class PendingSelection:
    """Context for an effect waiting for player selection."""
    callback: Callable[[int], None]  # receives the selected index
    selecting_player: Player
    previous_phase: GamePhase
    valid_indices: List[int] = field(default_factory=list)
    is_optional: bool = False  # if True, player can decline with action 62
    prompt: str = ""  # human-readable prompt for the UI
    effect_choices: Optional[List[dict]] = None  # for SelectEffectChoice: [{index, cardId, cardName, label}]
    keyword_prompt: Optional[dict] = None  # for keyword triggers: {keyword, cardId, cardName}
    on_decline: Optional[Callable[[], None]] = None  # called when player declines optional selection


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

        # Continuous effect modifier registry (mirrors DCGO's CardEffectInterfaces)
        self.modifiers: ModifierRegistry = ModifierRegistry()

        # Deferred turn-end: set when memory crosses 0 during a pending selection.
        # The turn ends after the selection resolves, not immediately.
        self._turn_end_deferred: bool = False

        # Opening mulligan state (set during start_game()).
        self._mulligan_order: List[Player] = []
        self._mulligan_index: int = 0
        self._mulligan_used: Dict[int, bool] = {}

        # Structured event sequence counter
        self._event_seq: int = 0

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

        # Opening setup: shuffle and draw opening hand for both players.
        # Security is set after mulligan declarations.
        for player in (self.player1, self.player2):
            player.shuffle_for_game_start()
            player.draw_opening_hand(5)

        self.turn_count = 1
        self.memory = 0
        self.turn_player.is_my_turn = True
        self.opponent_player.is_my_turn = False
        self.pending_attack = None
        self.pending_selection = None
        self.revealed_cards = []

        # Mulligan order starts with the first player.
        self._mulligan_order = [self.turn_player, self.opponent_player]
        self._mulligan_index = 0
        self._mulligan_used = {
            self.player1.player_id: False,
            self.player2.player_id: False,
        }
        self.current_phase = GamePhase.Mulligan
        self.active_player = self._mulligan_order[0]

        self.logger.log("[Setup] Opening hands drawn. Mulligan phase begins.")

    def _advance_mulligan(self):
        """Advance mulligan priority to the next player or finalize setup."""
        self._mulligan_index += 1
        if self._mulligan_index >= len(self._mulligan_order):
            self._finalize_opening_setup()
            return

        self.current_phase = GamePhase.Mulligan
        self.active_player = self._mulligan_order[self._mulligan_index]

    def _finalize_opening_setup(self):
        """Set security stacks and begin the first turn after mulligans."""
        for player in (self.player1, self.player2):
            player.setup_security_stack(5)

        self.active_player = None
        self.current_phase = GamePhase.Start
        self.logger.log("[Setup] Security stacks set. Starting turn 1.")
        self.phase_start()

    def action_keep_opening_hand(self):
        """Mulligan decision: keep current opening hand."""
        if self.current_phase != GamePhase.Mulligan:
            self.logger.log(f"[Rejected] keep hand: not in Mulligan phase (phase={self.current_phase})")
            return
        if self.active_player is None:
            return

        player = self.active_player
        self.logger.log(f"[Mulligan] {player.player_name} keeps opening hand")
        self._advance_mulligan()

    def action_mulligan_opening_hand(self):
        """Mulligan decision: return hand to deck, shuffle, draw 5 new cards."""
        if self.current_phase != GamePhase.Mulligan:
            self.logger.log(f"[Rejected] mulligan: not in Mulligan phase (phase={self.current_phase})")
            return
        if self.active_player is None:
            return

        player = self.active_player
        pid = player.player_id
        if self._mulligan_used.get(pid, False):
            self.logger.log(f"[Mulligan] {player.player_name} already used mulligan; keeping hand")
            self._advance_mulligan()
            return

        hand_size = len(player.hand_cards)
        player.library_cards.extend(player.hand_cards)
        player.hand_cards.clear()
        random.shuffle(player.library_cards)
        player.draw_opening_hand(5)
        self._mulligan_used[pid] = True
        self.logger.log(f"[Mulligan] {player.player_name} redraws opening hand ({hand_size} -> {len(player.hand_cards)})")
        self._advance_mulligan()

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
        elif self.current_phase == GamePhase.EndOfTurnAction:
            # After end-of-turn keywords resolve, proceed to switch turn
            self.current_phase = GamePhase.End
            self.switch_turn()
            self.phase_start()

    def phase_start(self):
        self.current_phase = GamePhase.Start
        self.logger.log(f"=== Turn {self.turn_count} — {self.turn_player.player_name} ===")
        self._emit('phase_change', phase='Start', turn=self.turn_count)
        # Unsuspend turn player's permanents (skip those with <Reboot> — they unsuspend on opponent's turn)
        self.turn_player.unsuspend_all(skip_reboot=True)
        # <Reboot>: unsuspend opponent's Reboot permanents (they unsuspend during opponent's unsuspend phase)
        self.opponent_player.unsuspend_reboot_only()
        self._reset_effect_turn_counts()
        self._clear_temp_dp()
        self.modifiers.clear_expiry('end_of_turn')
        # "Until opponent's turn ends" modifiers expire at the start of the
        # granting player's next turn (i.e., after the opponent's turn ended).
        self.modifiers.clear_opponent_turn_expiry(self.turn_player)
        self.execute_effects(EffectTiming.OnStartTurn)

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
        me = self.turn_player

        can_hatch = me.breeding_area is None and bool(me.digitama_library_cards)
        can_move = me.breeding_area is not None and (me.breeding_area.level or 0) >= 3
        can_training = (
            me.breeding_area is not None
            and me.breeding_area.is_digimon
            and not me.breeding_area.is_suspended
            and me.breeding_area.has_keyword('_is_training')
            and bool(me.library_cards)
        )

        # If the only legal action is pass, auto-advance to Main.
        if not (can_hatch or can_move or can_training):
            self.logger.log_verbose("No actionable breeding options; auto-skipping to Main.")
            self.action_breeding_pass()
            return

        pass  # Waiting for agent action

    def phase_main(self):
        self.logger.log_verbose("Phase: Main")
        self.execute_effects(EffectTiming.OnStartMainPhase)
        pass  # Waiting for agent actions

    def phase_end(self):
        self.execute_effects(EffectTiming.OnEndTurn)
        # Check for end-of-turn keyword actions (Vortex, Overclock)
        if self._has_end_of_turn_keywords():
            self.current_phase = GamePhase.EndOfTurnAction
            return  # Park for agent decision
        self.next_phase()

    def _has_end_of_turn_keywords(self) -> bool:
        """Check if the turn player has any Digimon with Vortex or Overclock."""
        for perm in self.turn_player.battle_area:
            if not perm.is_digimon:
                continue
            if perm.has_keyword('_is_vortex') and perm.can_attack(is_vortex=True):
                return True
            if perm.has_keyword('_is_overclock'):
                # Overclock needs a sacrifice target (Token or other Digimon)
                has_sacrifice = any(
                    p is not perm and (p.is_token or p.is_digimon)
                    for p in self.turn_player.battle_area
                )
                if has_sacrifice:
                    return True
        return False

    def switch_turn(self):
        self.turn_player, self.opponent_player = self.opponent_player, self.turn_player
        self.turn_count += 1
        self.memory = -self.memory
        self.turn_player.is_my_turn = True
        self.opponent_player.is_my_turn = False
        self._turn_end_deferred = False

    def pass_turn(self):
        if self.memory >= 0:
            self.memory = -3
        self.execute_effects(EffectTiming.OnEndMainPhase)
        self.current_phase = GamePhase.End
        self.next_phase()

    def check_turn_end(self):
        if self.memory < 0:
            if self.pending_selection is not None:
                # An effect triggered a selection — defer the turn end until
                # the selection resolves so the effect finishes executing.
                self._turn_end_deferred = True
                return
            self.current_phase = GamePhase.End
            self.next_phase()

    def _check_deferred_turn_end(self):
        """End the turn if it was deferred while waiting for a selection."""
        if self._turn_end_deferred and self.pending_selection is None:
            self._turn_end_deferred = False
            self.check_turn_end()

    @staticmethod
    def _card_ref(card) -> str:
        """Format a card reference as [CARD_ID:Name] for frontend parsing."""
        if card is None:
            return "Unknown"
        card_id = getattr(card, "card_id", None)
        names = getattr(card, "card_names", None)
        name = names[0] if names else "Unknown"
        if card_id:
            return f"[{card_id}:{name}]"
        return name

    @staticmethod
    def _perm_ref(perm) -> str:
        """Format a permanent's top card as [CARD_ID:Name]."""
        if perm is None:
            return "Unknown"
        top = getattr(perm, "top_card", None)
        return Game._card_ref(top)

    @staticmethod
    def _effect_source_name(effect) -> str:
        src = getattr(effect, "effect_source_card", None)
        if src is None:
            return "Unknown"
        return Game._card_ref(src)

    @staticmethod
    def _effect_text_for_log(effect) -> str:
        text = (effect.effect_description or "").strip()
        if not text:
            text = (effect.effect_name or "").strip()
        if not text:
            text = "<no effect text>"
        return " ".join(text.replace("\r", "\n").split())

    def _emit(self, event_type: str, player: int = 0, **kwargs) -> None:
        """Emit a structured GameEvent if the logger supports it."""
        if not hasattr(self.logger, 'emit'):
            return
        event = GameEvent(
            type=event_type,
            seq=self._event_seq,
            player=player or self.turn_player.player_id,
            source_card_id=kwargs.pop('source_card_id', None),
            source_slot=kwargs.pop('source_slot', None),
            target_card_id=kwargs.pop('target_card_id', None),
            target_slot=kwargs.pop('target_slot', None),
            meta=kwargs,
        )
        self._event_seq += 1
        self.logger.emit(event)

    @staticmethod
    def _card_id(card) -> Optional[str]:
        """Extract card_id string from a CardSource, or None."""
        return getattr(card, 'card_id', None) if card else None

    @staticmethod
    def _card_name(card) -> str:
        """Extract first card name from a CardSource."""
        if card is None:
            return "Unknown"
        names = getattr(card, 'card_names', None)
        return names[0] if names else "Unknown"

    def _perm_slot(self, perm: Permanent) -> Optional[int]:
        """Return the battle area slot index for a permanent, or None."""
        for player in (self.player1, self.player2):
            try:
                return player.battle_area.index(perm)
            except ValueError:
                continue
        return None

    def _log_effect_activation(self, effect, timing: EffectTiming) -> None:
        source_name = self._effect_source_name(effect)
        effect_text = self._effect_text_for_log(effect)
        self.logger.log(f"[Effect] {timing.name} | {source_name}: {effect_text}")
        src_card = getattr(effect, 'effect_source_card', None)
        src_perm = getattr(effect, 'effect_source_permanent', None)
        is_inherited = getattr(effect, 'is_inherited_effect', False)
        effect_desc = (getattr(effect, 'effect_description', None) or "").strip()
        self._emit(
            'effect_activate',
            source_card_id=self._card_id(src_card),
            source_slot=self._perm_slot(src_perm) if src_perm else None,
            effect_name=effect.effect_name or "",
            effect_text=effect_desc,
            timing=timing.name,
            card_name=self._card_name(src_card),
            is_inherited=is_inherited,
        )

    # ─── Effect Execution ────────────────────────────────────────────

    def _option_stays_on_field(self, card: 'CardSource') -> bool:
        if not card.is_option:
            return False
        for effect in card.effect_list(EffectTiming.NoTiming):
            if getattr(effect, '_is_delay', False) or getattr(effect, '_is_training', False):
                return True
        return False

    def _trash_option_after_resolution(self, owner: Player, perm: Optional[Permanent]) -> None:
        if perm is None or perm not in owner.battle_area:
            return
        owner.battle_area.remove(perm)
        self.cleanup_modifiers_for_permanent(perm)
        owner.trash_cards.extend(perm.card_sources)
        self.logger.log(f"[Option] {self._perm_ref(perm)} trashed after resolving")

    def calculate_play_cost(
        self,
        player: Player,
        card: 'CardSource',
        *,
        source_zone: str = "hand",
        free: bool = False,
        manual_reduction: int = 0,
    ) -> int:
        if free:
            return 0

        base_cost = card.get_cost_itself
        reduction = max(0, manual_reduction)

        def apply_effect(effect, context: Dict[str, Any]) -> None:
            nonlocal reduction
            if getattr(effect, 'timing', None) != EffectTiming.BeforePayCost:
                return
            if not effect.can_activate_this_turn():
                return
            if effect.can_use_condition is not None and not effect.can_use_condition(context):
                return
            dynamic_reduction = getattr(effect, '_cost_reduction_value_fn', None)
            if callable(dynamic_reduction):
                try:
                    reduction += max(0, int(dynamic_reduction(context)))
                except Exception:
                    return
                return
            effect_reduction = getattr(effect, 'cost_reduction', 0)
            if effect_reduction:
                reduction += max(0, int(effect_reduction))

        for perm in list(player.battle_area):
            for effect in perm.effect_list(EffectTiming.NoTiming):
                apply_effect(
                    effect,
                    {
                        "game": self,
                        "player": player,
                        "permanent": perm,
                        "card_source": card,
                        "played_card": card,
                        "source_zone": source_zone,
                        "free": free,
                    },
                )

        if player.breeding_area is not None:
            breeding_perm = player.breeding_area
            for effect in breeding_perm.effect_list(EffectTiming.NoTiming):
                if not getattr(effect, '_allow_breeding_source', False):
                    continue
                apply_effect(
                    effect,
                    {
                        "game": self,
                        "player": player,
                        "permanent": breeding_perm,
                        "card_source": card,
                        "played_card": card,
                        "source_zone": source_zone,
                        "free": free,
                    },
                )

        for effect in card.effect_list(EffectTiming.NoTiming):
            apply_effect(
                effect,
                {
                    "game": self,
                    "player": player,
                    "permanent": card.permanent_of_this_card(),
                    "card_source": card,
                    "played_card": card,
                    "source_zone": source_zone,
                    "free": free,
                },
            )

        reduction += max(0, int(getattr(player, "_temp_play_cost_reduction", 0)))
        return max(0, base_cost - reduction)

    def execute_effects(self, timing: EffectTiming, extra_context: Optional[dict] = None):
        """Execute all effects matching the given timing with stack-based resolution.

        Mirrors DCGO's AutoProcessing:
        1. Collect all triggered effects into a stack.
        2. Sort: turn player first, then opponent; mandatory before optional
           within each group.
        3. Resolve one at a time, validating each can still fire.
        4. After each resolution, check for newly-triggered effects (chain).
        """
        stack = self._collect_triggered_effects(timing, extra_context)
        if not stack:
            return

        self._resolve_effect_stack(stack, timing, extra_context)

    def _collect_triggered_effects(
        self, timing: EffectTiming, extra_context: Optional[dict] = None
    ) -> List[TriggeredEffect]:
        """Collect all activatable effects for a timing into a sorted stack.

        Resolution order (matching DCGO MultipleSkills):
        1. Turn player mandatory effects
        2. Turn player optional effects
        3. Non-turn player mandatory effects
        4. Non-turn player optional effects
        """
        all_perms: List[Permanent] = list(self.turn_player.battle_area) + list(self.opponent_player.battle_area)
        if self.turn_player.breeding_area is not None:
            all_perms.append(self.turn_player.breeding_area)
        if self.opponent_player.breeding_area is not None:
            all_perms.append(self.opponent_player.breeding_area)

        played_card = extra_context.get('played_card') if extra_context else None
        digivolved_perm = extra_context.get('digivolved_permanent') if extra_context else None

        stack: List[TriggeredEffect] = []

        for perm in all_perms:
            effects = perm.effect_list(timing)
            for effect in effects:
                if not effect.on_process_callback:
                    continue
                if not effect.can_activate_this_turn():
                    continue
                if not self._effect_matches_timing(effect, timing, perm, played_card, digivolved_perm):
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
                    if "player" in extra_context:
                        context["event_player"] = extra_context["player"]
                    if "permanent" in extra_context:
                        context["event_permanent"] = extra_context["permanent"]
                        if "played_permanent" not in extra_context and "played_card" in extra_context:
                            context["played_permanent"] = extra_context["permanent"]
                    for key, value in extra_context.items():
                        if key in {"player", "permanent"}:
                            continue
                        context[key] = value

                is_tp = (owner is self.turn_player)

                stack.append(TriggeredEffect(
                    effect=effect,
                    permanent=perm,
                    owner=owner,
                    context=context,
                    is_turn_player=is_tp,
                ))

        # Sort: turn player first, mandatory before optional within each group
        def sort_key(te: TriggeredEffect):
            player_order = 0 if te.is_turn_player else 1
            optional_order = 1 if te.effect.is_optional else 0
            return (player_order, optional_order)

        stack.sort(key=sort_key)
        return stack

    def _resolve_effect_stack(
        self,
        stack: List[TriggeredEffect],
        timing: EffectTiming,
        extra_context: Optional[dict] = None,
    ):
        """Resolve a stack of triggered effects, handling chain triggers.

        After each effect fires, newly-triggered effects are collected and
        resolved first (LIFO), matching DCGO's recursive TriggeredSkillProcess.
        """
        max_chain = 50  # safety bound to prevent infinite loops

        for te in stack:
            # Validate the effect can still fire (permanent may have left field)
            if not self._triggered_effect_still_valid(te):
                continue

            # Re-check OPT: nested effects may have incremented the count
            # since the stack was collected.
            if not te.effect.can_activate_this_turn():
                continue

            if te.effect.can_use_condition is not None and not te.effect.can_use_condition(te.context):
                continue

            self._log_effect_activation(te.effect, timing)
            te.effect.record_activation()
            te.effect.on_process_callback(te.context)

            # Check for chain triggers — effects that just became activatable
            # due to the state change from the effect we just resolved.
            # DCGO does this recursively; we bound with max_chain.
            chain_count = 0
            while chain_count < max_chain:
                chain_stack = self._collect_triggered_effects(timing, extra_context)
                # Filter out effects already in the original stack or already fired
                chain_stack = [
                    cte for cte in chain_stack
                    if cte.effect.can_activate_this_turn()
                    and not any(cte.effect is orig.effect for orig in stack)
                ]
                if not chain_stack:
                    break
                for cte in chain_stack:
                    if not self._triggered_effect_still_valid(cte):
                        continue
                    if cte.effect.can_use_condition is not None and not cte.effect.can_use_condition(cte.context):
                        continue
                    self._log_effect_activation(cte.effect, timing)
                    cte.effect.record_activation()
                    cte.effect.on_process_callback(cte.context)
                chain_count += 1

    @staticmethod
    def _triggered_effect_still_valid(te: TriggeredEffect) -> bool:
        """Check that a triggered effect's source is still in a valid state.

        An effect should be skipped if its source permanent has left the field
        since the effect was collected (e.g., destroyed by a prior effect in
        the same stack resolution).
        """
        perm = te.permanent
        # Check the permanent still has cards (not removed from field)
        if perm.top_card is None:
            return False
        # Check the owning player still has this permanent on field
        owner = te.owner
        if hasattr(owner, 'battle_area') and perm not in owner.battle_area:
            # Also check breeding area
            if hasattr(owner, 'breeding_area') and perm is not owner.breeding_area:
                return False
        return True

    @staticmethod
    def _effect_matches_timing(
        effect, timing: 'EffectTiming', perm: 'Permanent',
        played_card, digivolved_perm,
    ) -> bool:
        """Check whether an effect should fire for the given timing.

        Effects carry timing flags (is_on_play, is_when_digivolving, etc.)
        set by the transpiler.  If an effect has ANY flag set, it must match
        the requested timing; otherwise it is skipped.  Effects with no
        timing flags but a ``timing`` field set via ``set_timing()`` are
        matched exactly.  Effects with neither flags nor a timing field
        default to **not firing** to prevent cross-timing pollution.
        """
        has_flag = (effect.is_on_play or effect.is_when_digivolving
                    or effect.is_on_attack or effect.is_on_deletion)

        if not has_flag:
            # Check the explicit timing field first (set via set_timing()).
            if getattr(effect, 'timing', None) is not None:
                if effect.timing == timing:
                    return True
                # Map script-side OptionSkill to engine-side OnUseOption
                # Only fire for the card that was actually just played,
                # not for other options already sitting in the battle area.
                if (effect.timing == EffectTiming.OptionSkill
                        and timing == EffectTiming.OnUseOption):
                    return (
                        played_card is not None
                        and perm.top_card is played_card
                    )
                # Map script-side SecuritySkill to engine-side OnSecurityCheck
                if (effect.timing == EffectTiming.SecuritySkill
                        and timing == EffectTiming.OnSecurityCheck):
                    return True
                return False

            # No timing flag and no timing field — only allow
            # OnEnterFieldAnyone for the card that just entered.
            if timing == EffectTiming.OnEnterFieldAnyone:
                return (
                    played_card is not None
                    and perm.top_card is played_card
                    and (effect.effect_source_card is None or effect.effect_source_card is perm.top_card)
                )
            # All other timings: unflagged + no timing = do not fire.
            return False

        if timing == EffectTiming.OnEnterFieldAnyone:
            return (
                effect.is_on_play
                and played_card is not None
                and perm.top_card is played_card
                and (effect.effect_source_card is None or effect.effect_source_card is perm.top_card)
            )

        if timing == EffectTiming.WhenDigivolving:
            return (
                effect.is_when_digivolving
                and digivolved_perm is not None
                and perm is digivolved_perm
                and perm.top_card is not None
                and (effect.effect_source_card is None or effect.effect_source_card is perm.top_card)
            )

        if timing in (EffectTiming.OnAllyAttack,):
            return effect.is_on_attack

        if timing == EffectTiming.OnDestroyedAnyone:
            return effect.is_on_deletion

        # Other engine-level timings (OnStartTurn, OnDraw, etc.)
        # Flagged effects must NOT fire on unrelated timings.
        return False

    def execute_deletion_effects(self, deleted_permanent: Permanent, owner: Player):
        """Execute OnDeletion effects for a permanent that was just deleted.

        Uses stack-based ordering: mandatory before optional, sorted by the
        effect's position in the card stack (top card effects first).
        """
        stack: List[TriggeredEffect] = []
        is_tp = (owner is self.turn_player)

        for source in deleted_permanent.card_sources:
            effects = source.effect_list(EffectTiming.OnDestroyedAnyone)
            for effect in effects:
                if not effect.can_activate_this_turn():
                    continue
                if not effect.on_process_callback:
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
                stack.append(TriggeredEffect(
                    effect=effect,
                    permanent=deleted_permanent,
                    owner=owner,
                    context=context,
                    is_turn_player=is_tp,
                ))

        # Sort: mandatory before optional
        stack.sort(key=lambda te: (1 if te.effect.is_optional else 0))

        for te in stack:
            if te.effect.can_use_condition is not None and not te.effect.can_use_condition(te.context):
                continue
            self._log_effect_activation(te.effect, EffectTiming.OnDestroyedAnyone)
            te.effect.record_activation()
            te.effect.on_process_callback(te.context)

    # ─── Modifier Registry Convenience Methods ──────────────────────

    def register_modifier(
        self,
        target_permanent: Permanent,
        modifier_type: ModifierType,
        condition: Optional[Callable] = None,
        value_fn: Optional[Callable] = None,
        source_effect=None,
        expiry: str = 'permanent',
    ) -> ModifierEntry:
        """Register a continuous modifier on a permanent.

        Args:
            target_permanent: The permanent that owns this modifier effect.
            modifier_type: What kind of modifier (from ModifierType enum).
            condition: Optional callable(permanent, context) -> bool.
            value_fn: Optional callable for value modifiers.
            source_effect: The ICardEffect that created this modifier.
            expiry: 'permanent', 'end_of_turn', 'end_of_attack', 'end_of_opponent_turn'.

        Returns:
            The ModifierEntry, which can be used to unregister later.
        """
        entry = ModifierEntry(
            modifier_type=modifier_type,
            condition=condition,
            value_fn=value_fn,
            source_effect=source_effect,
            source_permanent=target_permanent,
            expiry=expiry,
            granting_player=self.turn_player if expiry == 'end_of_opponent_turn' else None,
        )
        self.modifiers.register(entry)
        return entry

    def cleanup_modifiers_for_permanent(self, permanent: Permanent):
        """Remove all modifiers sourced from a permanent that left the field."""
        self.modifiers.unregister_by_source(permanent)

    def force_end_attack(self):
        """Force the current attack to end early (DCGO: IsEndAttack flag)."""
        if self.pending_attack:
            self.pending_attack.is_end_attack = True

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
                for linked in perm.linked_cards:
                    for effect in linked.effect_list(EffectTiming.NoTiming):
                        effect.reset_turn_count()
            if player.breeding_area:
                for source in player.breeding_area.card_sources:
                    for effect in source.effect_list(EffectTiming.NoTiming):
                        effect.reset_turn_count()

    def _clear_temp_dp(self):
        """Clear temporary DP modifiers and expired keyword grants at start of turn."""
        for player in [self.player1, self.player2]:
            for perm in player.battle_area:
                perm.clear_temp_dp()
                perm.clear_expired_grants(self.turn_count)

    # ─── Game Actions ────────────────────────────────────────────────

    def surrender(self, player_id: int) -> None:
        """A player concedes the game."""
        if self.game_over:
            return
        winner = self.player2 if player_id == self.player1.player_id else self.player1
        self.logger.log(f"[Surrender] Player {player_id} surrendered.")
        self._emit('surrender', player=player_id)
        self.declare_winner(winner)

    def declare_winner(self, winner: Player):
        self.game_over = True
        self.winner = winner
        # Clear all pending state so serialization / API clients see clean state
        self.pending_attack = None
        self.pending_selection = None
        self.active_player = None
        if hasattr(self, 'revealed_cards'):
            self.revealed_cards.clear()
        self.logger.log(f"Game Over! Winner: {winner.player_name}")
        self._emit('game_over', player=winner.player_id, winner=winner.player_id)

    # ─── Keyword display mapping ────────────────────────────────────────
    # Maps internal _is_{flag} attribute names to human-readable UI labels.
    _KEYWORD_DISPLAY_MAP = [
        ("_is_rush", "Rush"),
        ("_is_blocker", "Blocker"),
        ("_is_piercing", "Piercing"),
        ("_is_jamming", "Jamming"),
        ("_is_retaliation", "Retaliation"),
        ("_is_collision", "Collision"),
        ("_is_blitz", "Blitz"),
        ("_is_raid", "Raid"),
        ("_is_reboot", "Reboot"),
        ("_is_blast_digivolve", "Blast Digivolve"),
        ("_is_alliance", "Alliance"),
        ("_is_training", "Training"),
        ("_is_progress", "Progress"),
        ("_is_fortitude", "Fortitude"),
        ("_is_save", "Save"),
        ("_is_decoy", "Decoy"),
        ("_is_material_save", "Material Save"),
        ("_is_vortex", "Vortex"),
        ("_is_overclock", "Overclock"),
        ("_is_armor_purge", "Armor Purge"),
        ("_is_evade", "Evade"),
        ("_is_barrier", "Barrier"),
        ("_is_delay", "Delay"),
        ("_is_blast_dna_digivolve", "Blast DNA Digivolve"),
        ("_is_scapegoat", "Scapegoat"),
        ("_is_fragment", "Fragment"),
        ("_is_decode", "Decode"),
        ("_is_execute", "Execute"),
        ("_is_digisorption", "Digisorption"),
        ("_is_iceclad", "Iceclad"),
        ("_is_cannot_attack", "Cannot Attack"),
        ("_is_cannot_attack_player", "Cannot Attack Player"),
        ("_is_cannot_block", "Cannot Block"),
        ("_is_cannot_be_blocked", "Cannot Be Blocked"),
        ("_is_cannot_unsuspend", "Cannot Unsuspend"),
    ]

    def _get_perm_keywords(self, perm: Permanent) -> List[str]:
        """Return list of active keyword display names for a permanent."""
        keywords = []
        for attr, label in self._KEYWORD_DISPLAY_MAP:
            if perm.has_keyword(attr):
                keywords.append(label)
        # Security Attack modifier (special: includes value)
        sa_mod = perm.security_attack_modifier()
        if sa_mod > 0:
            keywords.append(f"Security Attack +{sa_mod}")
        elif sa_mod < 0:
            keywords.append(f"Security Attack {sa_mod}")
        return keywords

    def _get_activatable_effects(self, perm: Permanent, slot_idx: int) -> List[Dict[str, Any]]:
        """Return list of activatable effect descriptions for a permanent in a given slot."""
        effects = []
        # Training (effectIdx=0): unsuspended Digimon with Training and deck cards
        owner = self._find_owner(perm)
        if (perm.is_digimon and not perm.is_suspended
                and perm.has_keyword('_is_training') and owner.library_cards):
            effects.append({
                "effectIdx": 0,
                "actionId": 1000 + slot_idx * EFFECTS_PER_PERM,
                "name": "Training",
                "description": "Suspend to place top deck card at bottom of digi stack",
            })
        # Delay (effectIdx=1): card with _is_delay that wasn't placed this turn
        if (self._has_delay_effect(perm)
                and perm.turn_played < self.turn_count):
            perm_name = perm.top_card.card_names[0] if perm.top_card and perm.top_card.card_names else "this card"
            effects.append({
                "effectIdx": 1,
                "actionId": 1000 + slot_idx * EFFECTS_PER_PERM + 1,
                "name": "Delay",
                "description": f"Trash {perm_name} to activate delayed effect",
            })
        return effects

    def _serialize_perm(self, perm: Permanent, slot_idx: int) -> Dict[str, Any]:
        """Serialize a single permanent for to_json(), including keywords and effects."""
        return {
            "TopCardId": perm.top_card.card_id if perm.top_card else None,
            "TopCardName": perm.top_card.card_names[0] if perm.top_card and perm.top_card.card_names else None,
            "DP": perm.dp,
            "Level": perm.level,
            "IsSuspended": perm.is_suspended,
            "SourceCount": len(perm.card_sources),
            "CardKind": perm.top_card.c_entity_base.card_kind.name if perm.top_card and perm.top_card.c_entity_base and perm.top_card.c_entity_base.card_kind else None,
            "TurnPlayed": perm.turn_played,
            "Keywords": self._get_perm_keywords(perm),
            "ActivatableEffects": self._get_activatable_effects(perm, slot_idx),
        }

    def to_json(self) -> Dict[str, Any]:
        """Serialize game state to a dictionary (mirrors C# Game.ToJson)."""
        def player_data(p: Player) -> Dict[str, Any]:
            ba = p.breeding_area
            return {
                "Id": p.player_id,
                "Memory": self._get_memory_for(p),
                "HandCount": len(p.hand_cards),
                "HandIds": [c.card_id for c in p.hand_cards],
                "SecurityCount": len(p.security_cards),
                "DeckCount": len(p.library_cards),
                "BattleAreaCount": len(p.battle_area),
                "BattleArea": [
                    self._serialize_perm(perm, idx)
                    for idx, perm in enumerate(p.battle_area)
                ],
                "BreedingArea": {
                    "TopCardId": ba.top_card.card_id if ba and ba.top_card else None,
                    "TopCardName": ba.top_card.card_names[0] if ba and ba.top_card and ba.top_card.card_names else None,
                    "Level": ba.level if ba else None,
                    "Keywords": self._get_perm_keywords(ba) if ba else [],
                    "TurnPlayed": ba.turn_played if ba else None,
                } if ba else None,
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

    # ─── Keywords scanned by to_ui_json ──────────────────────────────
    _UI_KEYWORDS = [
        '_is_blocker', '_is_piercing', '_is_jamming', '_is_retaliation',
        '_is_rush', '_is_reboot', '_is_raid', '_is_blitz', '_is_alliance',
        '_is_collision', '_is_training', '_is_progress', '_is_fortitude',
        '_is_save', '_is_decoy', '_is_material_save', '_is_vortex',
        '_is_overclock', '_is_armor_purge', '_is_evade', '_is_barrier',
        '_is_blast_digivolve', '_is_blast_dna_digivolve',
        '_is_delay', '_is_digisorption',
        '_is_scapegoat', '_is_fragment', '_is_iceclad',
        '_is_decode', '_is_execute',
        '_is_cannot_attack', '_is_cannot_attack_player', '_is_cannot_block',
        '_is_cannot_be_blocked', '_is_cannot_unsuspend',
    ]

    def to_ui_json(self) -> Dict[str, Any]:
        """Extended game state for the UI frontend.

        Includes keywords, digivolution stack details, trash, security IDs,
        revealed cards, and pending selection/attack context.
        """
        def clean_text(text: Optional[str]) -> str:
            return (text or "").replace("\r\n", "\n").strip()

        def perm_data(perm: Permanent) -> Dict[str, Any]:
            keywords = [
                kw.replace('_is_', '')
                for kw in self._UI_KEYWORDS
                if perm.has_keyword(kw)
            ]

            keyword_innate: List[str] = []
            keyword_gained: List[str] = []
            grants = getattr(perm, "_granted_keywords", {})
            for kw in self._UI_KEYWORDS:
                if not perm.has_keyword(kw):
                    continue
                key = kw.replace('_is_', '')
                expiry = grants.get(kw)
                is_gained = (
                    expiry is not None
                    and (expiry == -1 or self.turn_count <= expiry)
                )
                if is_gained:
                    keyword_gained.append(key)
                else:
                    keyword_innate.append(key)

            sources = []
            for src in perm.card_sources:
                entity = getattr(src, "c_entity_base", None)
                sources.append({
                    "cardId": src.card_id,
                    "cardName": src.card_names[0] if src.card_names else None,
                    "isTop": src is perm.top_card,
                    "optState": perm.source_opt_state(src),
                    "dpContribution": perm.source_dp_contribution(src),
                    "mainEffectText": clean_text(entity.effect_description_eng if entity else ""),
                    "inheritedEffectText": clean_text(entity.inherited_effect_description_eng if entity else ""),
                    "colors": [c.value for c in getattr(src, 'card_colors', [])],
                })

            inherited_effects = []
            for idx, src in enumerate(perm.card_sources[:-1]):
                entity = getattr(src, "c_entity_base", None)
                inherited_text = clean_text(entity.inherited_effect_description_eng if entity else "")
                if inherited_text:
                    inherited_effects.append({
                        "sourceIndex": idx,
                        "cardId": src.card_id,
                        "cardName": src.card_names[0] if src.card_names else None,
                        "text": inherited_text,
                    })

            top_entity = perm.top_card.c_entity_base if perm.top_card else None
            main_effect_text = clean_text(top_entity.effect_description_eng if top_entity else "")

            dp_base = perm.top_card.base_dp if perm.top_card and perm.top_card.base_dp is not None else None
            dp_sources = [
                {
                    "cardId": src.card_id,
                    "cardName": src.card_names[0] if src.card_names else None,
                    "value": perm.source_dp_contribution(src),
                }
                for src in perm.card_sources
            ]
            temp_mods = getattr(perm, "_dp_modifiers", [])
            if perm.is_immune_to_opponent_effects:
                dp_temporary = float(sum(m for m in temp_mods if m >= 0))
            else:
                dp_temporary = float(sum(temp_mods))

            colors = []
            if perm.top_card:
                colors = [c.value for c in getattr(perm.top_card, 'card_colors', [])]
            return {
                "topCardId": perm.top_card.card_id if perm.top_card else None,
                "topCardName": perm.top_card.card_names[0] if perm.top_card and perm.top_card.card_names else None,
                "dp": perm.dp,
                "level": perm.level,
                "isSuspended": perm.is_suspended,
                "sourceCount": len(perm.card_sources),
                "keywords": keywords,
                "keywordBreakdown": {
                    "innate": keyword_innate,
                    "gained": keyword_gained,
                },
                "securityAttackModifier": perm.security_attack_modifier(),
                "linkedCardIds": [lc.card_id for lc in perm.linked_cards],
                "sources": sources,
                "mainEffectText": main_effect_text,
                "inheritedEffects": inherited_effects,
                "dpBreakdown": {
                    "base": dp_base,
                    "sources": dp_sources,
                    "temporary": dp_temporary,
                    "total": perm.dp,
                },
                "turnPlayed": perm.turn_played,
                "colors": colors,
            }

        def player_ui_data(p: Player) -> Dict[str, Any]:
            breeding = None
            if p.breeding_area:
                breeding = perm_data(p.breeding_area)
            return {
                "id": p.player_id,
                "memory": self._get_memory_for(p),
                "handCount": len(p.hand_cards),
                "handIds": [c.card_id for c in p.hand_cards],
                "handCards": [
                    {
                        "cardId": c.card_id,
                        "cardName": c.card_names[0] if c.card_names else "",
                        "playCost": c.c_entity_base.play_cost if c.c_entity_base else 0,
                        "level": c.c_entity_base.level if c.c_entity_base else None,
                        "dp": c.c_entity_base.dp if c.c_entity_base else None,
                        "colors": [col.value for col in c.card_colors],
                        "cardKind": c.c_entity_base.card_kind.value if c.c_entity_base else 0,
                        "evoCosts": [
                            {"color": ec.card_color.value, "level": ec.level, "cost": ec.memory_cost}
                            for ec in (c.c_entity_base.evo_costs if c.c_entity_base else [])
                        ],
                    }
                    for c in p.hand_cards
                ],
                "securityCount": len(p.security_cards),
                "securityIds": [
                    c.card_id if c in p.face_up_security else None
                    for c in p.security_cards
                ],
                "securityFaceUp": [c in p.face_up_security for c in p.security_cards],
                "deckCount": len(p.library_cards),
                "eggDeckCount": len(p.digitama_library_cards),
                "battleAreaCount": len(p.battle_area),
                "battleArea": [perm_data(perm) for perm in p.battle_area],
                "breedingArea": breeding,
                "trashIds": [c.card_id for c in p.trash_cards],
            }

        revealed = [
            {"cardId": c.card_id, "owner": c.owner.player_id if c.owner else 0}
            for c in self.revealed_cards
        ]

        pending_sel = None
        if self.pending_selection:
            ps = self.pending_selection
            pending_sel = {
                "phase": self.current_phase.value,
                "validIndices": ps.valid_indices,
                "isOptional": ps.is_optional,
                "prompt": ps.prompt,
                "selectingPlayer": ps.selecting_player.player_id,
            }
            if ps.effect_choices:
                pending_sel["effectChoices"] = ps.effect_choices
            if ps.keyword_prompt:
                pending_sel["keywordPrompt"] = ps.keyword_prompt

        pending_atk = None
        if self.pending_attack:
            pa = self.pending_attack
            attacker_slot = -1
            for i, perm in enumerate(self.turn_player.battle_area):
                if perm is pa.attacker:
                    attacker_slot = i
                    break
            target_slot = -1
            if isinstance(pa.effective_target, Permanent):
                enemy = self.turn_player.enemy
                for i, perm in enumerate(enemy.battle_area):
                    if perm is pa.effective_target:
                        target_slot = i
                        break
            else:
                target_slot = SECURITY_TARGET  # security attack
            pending_atk = {
                "attackerSlot": attacker_slot,
                "targetSlot": target_slot,
            }

        return {
            "turnCount": self.turn_count,
            "currentPhase": self.current_phase.value,
            "currentPlayer": self.current_player_id,
            "memoryGauge": self._get_memory_for(self.player1),
            "isGameOver": self.game_over,
            "winner": self.winner.player_id if self.winner else None,
            "player1": player_ui_data(self.player1),
            "player2": player_ui_data(self.player2),
            "revealedCards": revealed,
            "pendingSelection": pending_sel,
            "pendingAttack": pending_atk,
        }

    def resolve_attack(self, attacker: Permanent, target: Union[Permanent, Player],
                       without_suspend: bool = False, is_vortex: bool = False,
                       return_phase: Optional[GamePhase] = None):
        """Begin an attack sequence. May pause for BlockTiming/CounterTiming/AllianceTiming.

        When blockers, counters, or Alliance options exist, the game transitions to
        interrupt phases and returns control to the game loop. The decoders
        for those phases resume the attack flow.

        When no interrupts are available, the entire attack resolves
        synchronously in a single call (backward compatible).
        """
        if not attacker.can_attack(without_tap=without_suspend, is_vortex=is_vortex):
            return

        target_name = target.player_name if isinstance(target, Player) else self._perm_ref(target)
        attacker_ref = self._perm_ref(attacker)
        self.logger.log(f"[Attack] {attacker_ref} attacks {target_name}")
        self._emit(
            'attack_declare',
            source_card_id=self._card_id(attacker.top_card),
            source_slot=self._perm_slot(attacker),
            target_card_id=self._card_id(target.top_card) if isinstance(target, Permanent) else None,
            target_slot=self._perm_slot(target) if isinstance(target, Permanent) else None,
            attacker_name=self._card_name(attacker.top_card),
            target_type='player' if isinstance(target, Player) else 'digimon',
            target_name=target_name,
        )

        # <Progress>: mark attacker as attacking (for effect immunity)
        attacker.is_attacking = True

        # Clear FORCE_ATTACK modifier once the forced Digimon actually attacks
        if self.modifiers.has_modifier(attacker, ModifierType.FORCE_ATTACK):
            # Remove all FORCE_ATTACK modifiers on this attacker
            entries = self.modifiers._modifiers.get(ModifierType.FORCE_ATTACK, [])
            self.modifiers._modifiers[ModifierType.FORCE_ATTACK] = [
                e for e in entries
                if not e.is_active(attacker)
            ]

        if not without_suspend:
            attacker.suspend()

        # Trigger When Attacking (OnAllyAttack for the attacker's effects)
        self.execute_effects(EffectTiming.OnAllyAttack, {"attacker": attacker})

        # Store pending attack context
        self.pending_attack = PendingAttack(
            attacker=attacker,
            original_target=target,
            effective_target=target,
            without_suspend=without_suspend,
            is_vortex=is_vortex,
            return_phase=return_phase,
        )

        # <Alliance>: check if attacker has Alliance and suspendable allies exist
        if attacker.has_keyword('_is_alliance'):
            has_alliance_targets = any(
                perm is not attacker and perm.is_digimon and not perm.is_suspended
                for perm in self.turn_player.battle_area
            )
            if has_alliance_targets:
                self.current_phase = GamePhase.AllianceTiming
                return  # Park for Alliance decision

        # No Alliance — enter counter timing (DCGO: counter before block)
        self._enter_counter_timing()

    def _enter_counter_timing(self):
        """Check for counter opportunities and enter CounterTiming if any exist.

        DCGO attack order: OnAttack → Counter → Block → Battle.
        Counter effects (blast digivolve) resolve before blocker selection,
        allowing the defending player to create stronger blockers.
        """
        pa = self.pending_attack
        if pa is None or pa.is_end_attack:
            self._resolve_battle()
            return

        has_counter = self._opponent_has_counter_options()

        if has_counter:
            self.current_phase = GamePhase.CounterTiming
            self.active_player = self.opponent_player
            return  # Park here; _decode_counter() will resume

        # No counter options — check for blockers
        self._check_blockers_or_continue()

    def _check_blockers_or_continue(self):
        """Check for blockers after counter decisions, then continue attack flow."""
        pa = self.pending_attack
        if pa is None or pa.is_end_attack:
            self._resolve_battle()
            return

        has_blockers = any(
            perm.can_block(pa.attacker) for perm in self.opponent_player.battle_area
        )

        if has_blockers:
            self.current_phase = GamePhase.BlockTiming
            self.active_player = self.opponent_player
            return  # Park here; _decode_block() will resume

        # No blockers — resolve battle immediately
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

    def _execute_security_checks(self, attacker: Permanent, defender_player: Player) -> bool:
        """Run security check loop for an attacker against a defending player.

        Handles <Security Attack +/-X> modifier, game-over detection, and
        attacker deletion from security effects.

        Returns:
            True if the attacker survived all security checks, False otherwise.
        """
        self.execute_effects(EffectTiming.OnSecurityCheck, {"attacker": attacker})
        sa_mod = attacker.security_attack_modifier()
        num_checks = max(0, 1 + sa_mod)
        for _ in range(num_checks):
            if self.game_over:
                return False
            # Check attacker still on field (could be deleted by security effect)
            if attacker not in self.turn_player.battle_area:
                return False
            result = defender_player.security_attack(attacker)
            if result == AttackResolution.AttackerDeleted:
                self.turn_player.delete_permanent(attacker, is_battle=True)
                return False
            elif result == AttackResolution.GameEnd:
                self.declare_winner(self.turn_player)
                return False
        return True

    def _resolve_battle(self):
        """Execute the actual battle resolution after block/counter decisions."""
        pa = self.pending_attack
        if pa is None:
            return

        attacker = pa.attacker
        target = pa.effective_target

        # Clear interrupt state — back to turn_player control
        return_phase = pa.return_phase or GamePhase.Main
        self.active_player = None
        self.pending_attack = None
        self.current_phase = return_phase

        if isinstance(target, Player):
            self._execute_security_checks(attacker, target)
        elif isinstance(target, Permanent):
            self.execute_effects(EffectTiming.OnStartBattle, {"attacker": attacker, "defender": target})
            a_dp = attacker.dp or 0
            t_dp = target.dp or 0
            attacker_wins = a_dp > t_dp
            defender_wins = a_dp < t_dp
            tie = a_dp == t_dp

            if attacker_wins:
                result_str = 'attacker_wins'
            elif defender_wins:
                result_str = 'defender_wins'
            else:
                result_str = 'tie'
            self._emit(
                'battle_result',
                source_card_id=self._card_id(attacker.top_card),
                target_card_id=self._card_id(target.top_card),
                attacker_dp=a_dp,
                defender_dp=t_dp,
                result=result_str,
            )

            if attacker_wins:
                # <Retaliation>: when this Digimon is deleted in battle, delete the winner
                has_retaliation = target.has_keyword('_is_retaliation')
                was_deleted = self.opponent_player.delete_permanent(target, is_battle=True)
                if has_retaliation and was_deleted:
                    self.logger.log(f"[Retaliation] {self._perm_ref(target)} retaliates!")
                    self._emit('keyword_trigger', source_card_id=self._card_id(target.top_card),
                               keyword='Retaliation', card_name=self._card_name(target.top_card))
                    # Retaliation is an opponent's effect — Progress blocks this
                    self.turn_player.delete_permanent(attacker, is_battle=True, is_opponent_effect=True)
                # <Piercing>: after winning battle vs Digimon, check security
                # Only triggers when the target was actually deleted (not prevented)
                elif was_deleted and attacker.has_keyword('_is_piercing') and attacker in self.turn_player.battle_area:
                    self.logger.log(f"[Piercing] {self._perm_ref(attacker)} pierces through!")
                    self._emit('keyword_trigger', source_card_id=self._card_id(attacker.top_card),
                               keyword='Piercing', card_name=self._card_name(attacker.top_card))
                    self._execute_security_checks(attacker, self.opponent_player)
            elif defender_wins:
                # <Retaliation> on attacker: when deleted in battle, delete the winner
                has_retaliation = attacker.has_keyword('_is_retaliation')
                was_deleted = self.turn_player.delete_permanent(attacker, is_battle=True)
                if has_retaliation and was_deleted:
                    self.logger.log(f"[Retaliation] {self._perm_ref(attacker)} retaliates!")
                    self._emit('keyword_trigger', source_card_id=self._card_id(attacker.top_card),
                               keyword='Retaliation', card_name=self._card_name(attacker.top_card))
                    # Retaliation is an opponent's effect — Progress blocks this
                    self.opponent_player.delete_permanent(target, is_battle=True, is_opponent_effect=True)
            else:
                # Tie: both deleted (Retaliation doesn't trigger since neither "wins")
                self.opponent_player.delete_permanent(target, is_battle=True)
                self.turn_player.delete_permanent(attacker, is_battle=True)
            self.execute_effects(EffectTiming.OnEndBattle, {"attacker": attacker, "defender": target})

        # Clear attacker state before end-of-attack effects
        attacker.clear_attack_state()
        self.modifiers.clear_expiry('end_of_attack')

        self.execute_effects(EffectTiming.OnEndAttack)

        # If we returned to EndOfTurnAction (from Vortex/Overclock attack), check for more
        if self.current_phase == GamePhase.EndOfTurnAction:
            if not self._has_end_of_turn_keywords():
                # No more end-of-turn keywords — proceed to switch turn
                self.next_phase()
            # Otherwise stay parked in EndOfTurnAction for next agent decision
            return

        self.check_turn_end()

    def action_play_card(self, card_index: int):
        if self.current_phase != GamePhase.Main:
            self.logger.log(f"[Rejected] action_play_card: not in Main phase (phase={self.current_phase})")
            return
        if card_index < 0 or card_index >= len(self.turn_player.hand_cards):
            self.logger.log(f"[Rejected] action_play_card: index {card_index} out of range (hand size={len(self.turn_player.hand_cards)})")
            return

        card = self.turn_player.hand_cards[card_index]
        cost = self.calculate_play_cost(self.turn_player, card)

        self.logger.log(f"[Play] {self.turn_player.player_name} plays {self._card_ref(card)} (cost: {cost})")
        is_option = card.is_option
        played_perm = self.turn_player.play_card(card)
        self._emit(
            'play_card',
            source_card_id=self._card_id(card),
            source_slot=self._perm_slot(played_perm) if played_perm else None,
            cost=cost,
            card_name=self._card_name(card),
        )
        self.turn_player.lose_memory(cost)
        if hasattr(self.turn_player, "_temp_play_cost_reduction"):
            self.turn_player._temp_play_cost_reduction = 0

        if is_option:
            self.execute_effects(
                EffectTiming.OnUseOption,
                {"played_card": card, "played_permanent": played_perm, "event_player": self.turn_player},
            )
        self.execute_effects(
            EffectTiming.OnEnterFieldAnyone,
            {"played_card": card, "played_permanent": played_perm, "event_player": self.turn_player},
        )
        if is_option and not self._option_stays_on_field(card):
            self._trash_option_after_resolution(self.turn_player, played_perm)
        self.check_turn_end()

    def action_digivolve(self, permanent_index: int, card_index: int):
        if self.current_phase != GamePhase.Main:
            self.logger.log(f"[Rejected] action_digivolve: not in Main phase (phase={self.current_phase})")
            return
        if permanent_index >= len(self.turn_player.battle_area):
            self.logger.log(f"[Rejected] action_digivolve: permanent index {permanent_index} out of range (field size={len(self.turn_player.battle_area)})")
            return
        if card_index >= len(self.turn_player.hand_cards):
            self.logger.log(f"[Rejected] action_digivolve: card index {card_index} out of range (hand size={len(self.turn_player.hand_cards)})")
            return

        perm = self.turn_player.battle_area[permanent_index]
        card = self.turn_player.hand_cards[card_index]

        from_card_id = self._card_id(perm.top_card) if perm.top_card else None
        cost = self.turn_player.digivolve(perm, card)
        self.turn_player.lose_memory(cost)
        self._emit(
            'digivolve',
            source_card_id=self._card_id(card),
            source_slot=permanent_index,
            cost=cost,
            card_name=self._card_name(card),
            from_card_id=from_card_id,
        )

        # Digivolution bonus: draw 1 card (Rule 8-1-4)
        self.turn_player.draw()

        # Track digivolution turn for <Blitz> checks
        perm.turn_digivolved = self.turn_count

        self.execute_effects(EffectTiming.WhenDigivolving, {"digivolved_permanent": perm})
        self.check_turn_end()

    def action_digivolve_breeding(self, card_index: int):
        """Digivolve a hand card onto the breeding area Digimon."""
        if self.current_phase != GamePhase.Main:
            self.logger.log(f"[Rejected] action_digivolve_breeding: not in Main phase (phase={self.current_phase})")
            return
        if self.turn_player.breeding_area is None:
            self.logger.log("[Rejected] action_digivolve_breeding: breeding area is empty")
            return
        if card_index < 0 or card_index >= len(self.turn_player.hand_cards):
            self.logger.log(f"[Rejected] action_digivolve_breeding: card index {card_index} out of range (hand size={len(self.turn_player.hand_cards)})")
            return

        perm = self.turn_player.breeding_area
        card = self.turn_player.hand_cards[card_index]

        cost = self.turn_player.digivolve(perm, card)
        self.turn_player.lose_memory(cost)

        # Digivolution bonus: draw 1 card (Rule 8-1-4)
        # The draw is part of the digivolution procedure, not an effect,
        # so it applies even in the breeding area.
        self.turn_player.draw()

        perm.turn_digivolved = self.turn_count

        # Breeding area effects do not trigger normal timing windows
        # unless explicitly scoped to breeding by card text/rules.
        self.check_turn_end()

    def action_attack_player(self, attacker_index: int):
        if self.current_phase != GamePhase.Main:
            self.logger.log(f"[Rejected] action_attack_player: not in Main phase (phase={self.current_phase})")
            return
        if attacker_index < 0 or attacker_index >= len(self.turn_player.battle_area):
            self.logger.log(f"[Rejected] action_attack_player: attacker index {attacker_index} out of range (field size={len(self.turn_player.battle_area)})")
            return
        attacker = self.turn_player.battle_area[attacker_index]
        self.resolve_attack(attacker, self.opponent_player)

    def action_attack_digimon(self, attacker_index: int, target_index: int):
        """Attack an opponent's digimon (by field index)."""
        if self.current_phase != GamePhase.Main:
            self.logger.log(f"[Rejected] action_attack_digimon: not in Main phase (phase={self.current_phase})")
            return
        if attacker_index < 0 or attacker_index >= len(self.turn_player.battle_area):
            self.logger.log(f"[Rejected] action_attack_digimon: attacker index {attacker_index} out of range (field size={len(self.turn_player.battle_area)})")
            return
        if target_index < 0 or target_index >= len(self.opponent_player.battle_area):
            self.logger.log(f"[Rejected] action_attack_digimon: target index {target_index} out of range (opponent field size={len(self.opponent_player.battle_area)})")
            return
        attacker = self.turn_player.battle_area[attacker_index]
        target = self.opponent_player.battle_area[target_index]
        self.resolve_attack(attacker, target)

    def action_hatch(self):
        """Hatch from digitama deck into breeding area."""
        if self.current_phase != GamePhase.Breeding:
            self.logger.log(f"[Rejected] action_hatch: not in Breeding phase (phase={self.current_phase})")
            return
        self.logger.log(f"[Hatch] {self.turn_player.player_name} hatches from egg deck")
        before_breeding = self.turn_player.breeding_area
        before_egg_count = len(self.turn_player.digitama_library_cards)
        self.turn_player.hatch()
        hatched = (
            before_breeding is None
            and self.turn_player.breeding_area is not None
            and len(self.turn_player.digitama_library_cards) < before_egg_count
        )
        if hatched:
            self.current_phase = GamePhase.Main
            self.phase_main()

    def action_move_from_breeding(self):
        """Move breeding area digimon to battle area."""
        if self.current_phase != GamePhase.Breeding:
            self.logger.log(f"[Rejected] action_move_from_breeding: not in Breeding phase (phase={self.current_phase})")
            return
        self.logger.log(f"[Move] {self.turn_player.player_name} moves from breeding to battle area")
        before_breeding = self.turn_player.breeding_area
        before_battle_count = len(self.turn_player.battle_area)
        self.turn_player.move_from_breeding()
        moved = (
            before_breeding is not None
            and self.turn_player.breeding_area is None
            and len(self.turn_player.battle_area) > before_battle_count
        )
        if moved:
            self.execute_effects(EffectTiming.OnMove, {"moved_permanent": self.turn_player.battle_area[-1]})
            self.current_phase = GamePhase.Main
            self.phase_main()

    def action_breeding_pass(self):
        """Skip breeding phase and advance to main."""
        if self.current_phase != GamePhase.Breeding:
            self.logger.log(f"[Rejected] action_breeding_pass: not in Breeding phase (phase={self.current_phase})")
            return
        self.logger.log_verbose(f"{self.turn_player.player_name} passes breeding phase")
        self.current_phase = GamePhase.Main
        self.phase_main()

    def action_pass_turn(self):
        self.logger.log(f"[Pass] {self.turn_player.player_name} passes turn (memory: {self.memory})")
        self.pass_turn()

    # ─── Board State Tensor ──────────────────────────────────────────

    def get_board_state_tensor(self, player_id: int) -> np.ndarray:
        """Build a flat tensor representing the board from player's perspective.

        Returns a numpy array of shape (TENSOR_SIZE,) with dtype float32.
        Card identities are encoded as integer registry indices (float-cast).
        The nn.Embedding lookup happens inside the FeaturesExtractor on the GPU.

        Layout (TENSOR_SIZE=1375):
          [0-9]         Global data
          [10-569]      My battle area  (14 slots × 40)
          [570-1129]    Opp battle area (14 slots × 40)
          [1130-1149]   My hand  (20 card IDs)
          [1150-1169]   Opp hand (20 card IDs)
          [1170-1214]   My trash (45 card IDs)
          [1215-1259]   Opp trash (45 card IDs)
          [1260-1269]   My security (10 card IDs)
          [1270-1279]   Opp security (10 card IDs)
          [1280-1319]   My breeding (1 slot × 40)
          [1320-1359]   Opp breeding (1 slot × 40)
          [1360-1369]   Revealed cards (10 card IDs)
          [1370-1374]   Selection context (5)
        """
        me = self.player1 if player_id == 1 else self.player2
        opp = self.player2 if player_id == 1 else self.player1

        t = np.zeros(TENSOR_SIZE, dtype=np.float32)

        # --- Global [0-9] ---
        t[0] = min(float(self.turn_count) / 30.0, 1.0)
        t[1] = float(self.current_phase.value)
        t[2] = float(self._get_memory_for(me)) / 10.0
        # [3-9] are reserved (already 0.0)

        # --- My field ---
        off = _GLOBAL
        self._write_field(t, off, me.battle_area, FIELD_SLOTS)

        # --- Opp field ---
        off += _MY_BATTLE
        self._write_field(t, off, opp.battle_area, FIELD_SLOTS)

        # --- My hand ---
        off += _OPP_BATTLE
        self._write_card_ids(t, off, me.hand_cards, MAX_HAND)

        # --- Opp hand ---
        off += _MY_HAND
        self._write_card_ids(t, off, opp.hand_cards, MAX_HAND)

        # --- My trash ---
        off += _OPP_HAND
        self._write_card_ids(t, off, me.trash_cards, MAX_TRASH)

        # --- Opp trash ---
        off += _MY_TRASH
        self._write_card_ids(t, off, opp.trash_cards, MAX_TRASH)

        # --- My security (only face-up cards visible) ---
        off += _OPP_TRASH
        self._write_security_ids(t, off, me)

        # --- Opp security (only face-up cards visible) ---
        off += _MY_SECURITY
        self._write_security_ids(t, off, opp)

        # --- My breeding ---
        off += _OPP_SECURITY
        breeding_list = [me.breeding_area] if me.breeding_area else []
        self._write_field(t, off, breeding_list, 1)

        # --- Opp breeding ---
        off += _MY_BREEDING
        opp_breeding_list = [opp.breeding_area] if opp.breeding_area else []
        self._write_field(t, off, opp_breeding_list, 1)

        # --- Revealed cards ---
        off += _OPP_BREEDING
        self._write_card_ids(t, off, self.revealed_cards, MAX_REVEALED)

        # --- Selection context ---
        off += _REVEALED
        ps = self.pending_selection
        if self.current_phase in (
            GamePhase.SelectTarget, GamePhase.SelectMaterial,
            GamePhase.SelectTrash, GamePhase.SelectSource,
            GamePhase.SelectHand, GamePhase.SelectReveal,
            GamePhase.SelectEffectChoice, GamePhase.SelectSecurity,
        ):
            t[off] = float(self.current_phase.value)

        if ps:
            t[off + 1] = float(len(ps.valid_indices))
            t[off + 2] = float(ps.selecting_player.player_id)

        # [off+3, off+4] are reserved (already 0.0)

        return t

    def _get_memory_for(self, player: Player) -> int:
        """Memory relative to player (positive = their favour)."""
        if player is self.turn_player:
            return self.memory
        return -self.memory

    @staticmethod
    def _write_field(tensor: np.ndarray, start_idx: int, permanents: List[Permanent], slots: int):
        """Write field slot data into tensor starting at start_idx.

        Layout per slot (SLOT_SIZE=40 floats):
          +0:       top card ID (integer registry index)
          +1:       current DP
          +2:       suspended (0/1)
          +3:       OPT total
          +4:       OPT used
          +5:       linked card count
          +6:       source count
          +7..+39:  11 source entries × 3 each:
                    [card_id, opt_state, dp_contribution]
        """
        for i, perm in enumerate(permanents[:slots]):
            base = start_idx + i * SLOT_SIZE
            top = perm.top_card

            # +0: top card ID
            if top:
                tensor[base] = float(CardRegistry.get_id(top.card_id))

            # Scalar fields
            tensor[base + 1] = float(perm.dp or 0) / DP_NORM
            tensor[base + 2] = 1.0 if perm.is_suspended else 0.0
            tensor[base + 3] = float(perm.opt_total)
            tensor[base + 4] = float(perm.opt_used)
            tensor[base + 5] = float(len(perm.linked_cards))
            tensor[base + 6] = float(len(perm.card_sources))

            # Source entries: [card_id, opt_state, dp_contribution] × MAX_SOURCES
            src_base = base + 7
            for j, src in enumerate(perm.card_sources[:MAX_SOURCES]):
                off = src_base + j * SOURCE_ENTRY_SIZE
                tensor[off] = float(CardRegistry.get_id(src.card_id))
                tensor[off + 1] = perm.source_opt_state(src)
                tensor[off + 2] = perm.source_dp_contribution(src) / DP_NORM

    @staticmethod
    def _write_card_ids(tensor: np.ndarray, start_idx: int, cards: list, limit: int):
        """Write card integer IDs into tensor starting at start_idx (1 float per card)."""
        for i, card in enumerate(cards[:limit]):
            tensor[start_idx + i] = float(CardRegistry.get_id(card.card_id))

    @staticmethod
    def _write_security_ids(tensor: np.ndarray, start_idx: int, player: Player):
        """Write security card IDs — only face-up cards are visible, face-down = 0.0."""
        for i, card in enumerate(player.security_cards[:MAX_SECURITY]):
            if card in player.face_up_security:
                tensor[start_idx + i] = float(CardRegistry.get_id(card.card_id))
            # else stays 0.0 (face-down)

    # ─── Action Mask ─────────────────────────────────────────────────

    def get_action_mask(self, player_id: int) -> List[float]:
        """Build an ACTION_SPACE_SIZE-float mask (1.0 = valid, 0.0 = invalid).

        Ranges match C# Digimon.Core.ActionDecoder:
          0-29:      Play card from hand
          30-59:     Trash card from hand (effect-driven)
          60:        Hatch
          61:        Move from breeding
          62:        Pass / end turn
          100-399:   Attack (100 + attacker*15 + target, target 14 = security)
          400-999:   Digivolve (400 + hand*15 + field, field 14 = breeding)
          1000-1999: Activate effect (1000 + perm*10 + effectIdx)
          2000-2167: Source selection (2000 + field*12 + sourceIdx)
        """
        mask = [0.0] * ACTION_SPACE_SIZE
        me = self.player1 if player_id == 1 else self.player2
        opp = self.player2 if player_id == 1 else self.player1

        phase = self.current_phase

        if phase == GamePhase.Mulligan:
            if self.active_player is not None and player_id == self.active_player.player_id:
                # Opening mulligan actions:
                #   0 = keep hand, 1 = mulligan (redraw 5)
                mask[0] = 1.0
                if not self._mulligan_used.get(player_id, False):
                    mask[1] = 1.0

        elif phase == GamePhase.Main:
            # Play cards (0-29)
            for i in range(min(len(me.hand_cards), 30)):
                card = me.hand_cards[i]
                play_cost = self.calculate_play_cost(me, card)
                if self.memory >= 0 and play_cost <= self.memory + 10:
                    # Option color requirement: must have a matching-color
                    # Digimon or Tamer on the field to play an Option card.
                    # Cards with match_color_requirement=False bypass this check.
                    if card.is_option and card.match_color_requirement:
                        option_colors = set(card.card_colors)
                        has_color_match = any(
                            bool(option_colors & set(p.top_card.card_colors))
                            for p in me.battle_area
                            if p.top_card and (p.is_digimon or p.is_tamer)
                        )
                        if (not has_color_match and me.breeding_area and me.breeding_area.top_card
                                and me.breeding_area.is_digimon):
                            has_color_match = bool(
                                option_colors & set(me.breeding_area.top_card.card_colors)
                            )
                        if not has_color_match:
                            continue
                    mask[i] = 1.0

            # Attack (100-399): 100 + attacker*TARGETS_PER_ATTACKER + target
            for i in range(min(len(me.battle_area), FIELD_SLOTS)):
                attacker = me.battle_area[i]
                if not attacker.can_attack():
                    continue
                # <Blitz>: when digivolved this turn, can attack even when memory < 0
                # Normal rule: can only attack during own turn (memory >= 0)
                if self.memory < 0:
                    has_blitz = attacker.has_keyword('_is_blitz')
                    digivolved_this_turn = (attacker.turn_digivolved >= 0 and
                                            attacker.turn_digivolved == self.turn_count)
                    if not (has_blitz and digivolved_this_turn):
                        continue
                # Security attack (target index SECURITY_TARGET) — check cannot_attack_player
                if attacker.can_attack_player():
                    mask[100 + i * TARGETS_PER_ATTACKER + SECURITY_TARGET] = 1.0
                # Digimon attacks (targets 0..FIELD_SLOTS-1, suspended only per rules)
                if attacker.has_keyword('_is_cannot_attack_digimon'):
                    continue
                has_raid = attacker.has_keyword('_is_raid')
                for j in range(min(len(opp.battle_area), FIELD_SLOTS)):
                    target = opp.battle_area[j]
                    if target.is_suspended and target.is_digimon:
                        mask[100 + i * TARGETS_PER_ATTACKER + j] = 1.0
                    elif has_raid and not target.is_suspended and target.is_digimon:
                        unsuspended_dps = [
                            (p.dp or 0) for p in opp.battle_area
                            if not p.is_suspended and p.is_digimon
                        ]
                        if unsuspended_dps and (target.dp or 0) == max(unsuspended_dps):
                            mask[100 + i * TARGETS_PER_ATTACKER + j] = 1.0

            # Digivolve (400-999): 400 + hand*FIELDS_PER_HAND + field
            for h in range(min(len(me.hand_cards), 30)):
                card = me.hand_cards[h]
                if not card.is_digimon:
                    continue
                for f in range(min(len(me.battle_area), FIELD_SLOTS)):
                    base_perm = me.battle_area[f]
                    if can_digivolve(card, base_perm):
                        mask[400 + h * FIELDS_PER_HAND + f] = 1.0
                # Virtual field index BREEDING_SLOT = breeding area digivolve.
                if me.breeding_area and can_digivolve(card, me.breeding_area):
                    mask[400 + h * FIELDS_PER_HAND + BREEDING_SLOT] = 1.0

            # DNA Digivolve (63-92): 63 + hand_idx
            for h in range(min(len(me.hand_cards), 30)):
                card = me.hand_cards[h]
                if not card.is_digimon:
                    continue
                if has_valid_dna_targets(card, me.battle_area):
                    mask[63 + h] = 1.0

            # <Training>: suspend to place top deck card at bottom of digi stack (1000-1999)
            for i in range(min(len(me.battle_area), FIELD_SLOTS)):
                perm = me.battle_area[i]
                if (perm.is_digimon and not perm.is_suspended
                        and perm.has_keyword('_is_training') and me.library_cards):
                    mask[1000 + i * EFFECTS_PER_PERM] = 1.0

            # <Delay>: trash Option card in battle area to activate delayed effect (effectIdx=1)
            for i in range(min(len(me.battle_area), FIELD_SLOTS)):
                perm = me.battle_area[i]
                if (self._has_delay_effect(perm)
                        and perm.turn_played < self.turn_count):  # Not same turn placed
                    mask[1000 + i * EFFECTS_PER_PERM + 1] = 1.0

            # Force Attack: if any of the turn player's Digimon have
            # FORCE_ATTACK modifier, restrict the mask to attack actions
            # for those Digimon only (opponent grants "this Digimon attacks").
            forced_attackers = []
            for i in range(min(len(me.battle_area), FIELD_SLOTS)):
                perm = me.battle_area[i]
                if (perm.is_digimon
                        and self.modifiers.has_modifier(perm, ModifierType.FORCE_ATTACK)):
                    forced_attackers.append(i)

            if forced_attackers:
                # Only allow attack actions for forced Digimon
                forced_mask = [0.0] * ACTION_SPACE_SIZE
                has_any_attack = False
                for i in forced_attackers:
                    attacker = me.battle_area[i]
                    if not attacker.can_attack():
                        continue
                    # Security attack
                    if attacker.can_attack_player():
                        forced_mask[100 + i * TARGETS_PER_ATTACKER + SECURITY_TARGET] = 1.0
                        has_any_attack = True
                    # Digimon attacks
                    has_raid = attacker.has_keyword('_is_raid')
                    for j in range(min(len(opp.battle_area), FIELD_SLOTS)):
                        target = opp.battle_area[j]
                        if target.is_suspended and target.is_digimon:
                            forced_mask[100 + i * TARGETS_PER_ATTACKER + j] = 1.0
                            has_any_attack = True
                        elif has_raid and not target.is_suspended and target.is_digimon:
                            unsuspended_dps = [
                                (p.dp or 0) for p in opp.battle_area
                                if not p.is_suspended and p.is_digimon
                            ]
                            if unsuspended_dps and (target.dp or 0) == max(unsuspended_dps):
                                forced_mask[100 + i * TARGETS_PER_ATTACKER + j] = 1.0
                                has_any_attack = True
                if has_any_attack:
                    return forced_mask
                # If forced Digimon can't attack (suspended etc), skip force
                # and fall through to normal mask

            # Pass (62) - always valid in main
            mask[62] = 1.0

        elif phase == GamePhase.Breeding:
            # Hatch (60)
            if me.breeding_area is None and me.digitama_library_cards:
                mask[60] = 1.0
            # Move (61)
            if me.breeding_area is not None and (me.breeding_area.level or 0) >= 3:
                mask[61] = 1.0
            # <Training> in breeding area
            ba = me.breeding_area
            if (ba is not None and ba.is_digimon and not ba.is_suspended
                    and ba.has_keyword('_is_training') and me.library_cards):
                mask[1000 + BREEDING_SLOT * EFFECTS_PER_PERM] = 1.0
            # Pass (62)
            mask[62] = 1.0

        elif phase == GamePhase.BlockTiming:
            attacker = self.pending_attack.attacker if self.pending_attack else Permanent([])
            has_collision = attacker.has_keyword('_is_collision')
            has_any_blocker = False
            for i in range(min(len(me.battle_area), FIELD_SLOTS)):
                perm = me.battle_area[i]
                if perm.can_block(attacker):
                    mask[100 + i] = 1.0
                    has_any_blocker = True
            # <Collision>: opponent must block (cannot decline) if blockers exist
            if has_collision and has_any_blocker:
                mask[62] = 0.0  # forced block
            else:
                mask[62] = 1.0  # can decline

        elif phase == GamePhase.CounterTiming:
            mask[62] = 1.0  # always can decline
            # Blast digivolve options (400-999): 400 + hand*FIELDS_PER_HAND + field
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
                        mask[400 + h * FIELDS_PER_HAND + f] = 1.0

        elif phase == GamePhase.EndOfTurnAction:
            # End-of-turn keyword actions: Vortex attacks and Overclock activations
            mask[62] = 1.0  # Can always decline / end turn

            for i in range(min(len(me.battle_area), FIELD_SLOTS)):
                perm = me.battle_area[i]
                if not perm.is_digimon:
                    continue

                # <Vortex>: attack opponent Digimon at end of turn
                if perm.has_keyword('_is_vortex') and perm.can_attack(is_vortex=True):
                    for j in range(min(len(opp.battle_area), FIELD_SLOTS)):
                        target = opp.battle_area[j]
                        if target.is_digimon:
                            mask[100 + i * TARGETS_PER_ATTACKER + j] = 1.0

                # <Overclock>: activate to sacrifice + attack player
                if perm.has_keyword('_is_overclock'):
                    has_sacrifice = any(
                        p is not perm and (p.is_token or p.is_digimon)
                        for p in me.battle_area
                    )
                    if has_sacrifice:
                        mask[1000 + i * EFFECTS_PER_PERM] = 1.0

        elif phase == GamePhase.AllianceTiming:
            # Alliance: select an unsuspended ally to suspend for DP + SA+1 bonus
            mask[62] = 1.0  # Can always decline Alliance
            pa = self.pending_attack
            if pa:
                for i in range(min(len(me.battle_area), FIELD_SLOTS)):
                    perm = me.battle_area[i]
                    if perm is not pa.attacker and perm.is_digimon and not perm.is_suspended:
                        mask[100 + i] = 1.0

        elif phase == GamePhase.SelectTrash:
            ps = self.pending_selection
            if ps and ps.valid_indices:
                for idx in ps.valid_indices:
                    if 0 <= idx < ACTION_SPACE_SIZE:
                        mask[idx] = 1.0
            else:
                max_trash_sel = SEL_TRASH_END - SEL_TRASH_START + 1  # 50 slots
                for i in range(min(len(me.trash_cards), max_trash_sel)):
                    mask[SEL_TRASH_START + i] = 1.0
            if ps and getattr(ps, 'is_optional', False):
                mask[62] = 1.0

        elif phase == GamePhase.SelectSource:
            ps = self.pending_selection
            if ps and ps.valid_indices:
                for idx in ps.valid_indices:
                    if 0 <= idx < ACTION_SPACE_SIZE:
                        mask[idx] = 1.0
            else:
                # Source selection: 2000 + field*SOURCES_PER_FIELD + sourceIdx
                for f in range(min(len(me.battle_area), FIELD_SLOTS)):
                    perm = me.battle_area[f]
                    for s in range(min(len(perm.card_sources), MAX_SOURCES)):
                        mask[2000 + f * SOURCES_PER_FIELD + s] = 1.0
            if ps and getattr(ps, 'is_optional', False):
                mask[62] = 1.0

        elif phase in (GamePhase.SelectTarget, GamePhase.SelectMaterial,
                       GamePhase.SelectHand, GamePhase.SelectReveal,
                       GamePhase.SelectEffectChoice, GamePhase.SelectSecurity):
            # Generic selection: use valid_indices from pending selection
            ps = self.pending_selection
            if ps is None:
                # Defensive recovery: selection phase with no pending selection
                # (can happen when chained effects overwrite selections).
                # Recover to Main if it's our turn, else allow pass.
                self.logger.log(f"[Recovery] Selection phase {phase} with no pending selection — recovering")
                self.current_phase = GamePhase.Main
                self.active_player = None
                return self.get_action_mask(player_id)
            if ps.valid_indices:
                for idx in ps.valid_indices:
                    if 0 <= idx < ACTION_SPACE_SIZE:
                        mask[idx] = 1.0
            # Optional selections allow declining (pass)
            if getattr(ps, 'is_optional', False):
                mask[62] = 1.0
            # Safety: if mask is all zeros for selection phase, force recovery
            if not any(mask):
                self.logger.log(
                    f"[Recovery] All-zero mask in selection phase {phase} — recovering to Main")
                self.pending_selection = None
                self.current_phase = GamePhase.Main
                self.active_player = None
                return self.get_action_mask(player_id)

        return mask

    def describe_actions(self, player_id: int) -> Dict[int, str]:
        """Return human-readable descriptions for all currently valid actions.

        Only includes actions where the mask is 1.0, so the frontend can
        render contextual tooltips and effect activation labels.
        """
        mask = self.get_action_mask(player_id)
        me = self.player1 if player_id == 1 else self.player2
        opp = self.player2 if player_id == 1 else self.player1
        descriptions: Dict[int, str] = {}

        for action_id in range(len(mask)):
            if mask[action_id] != 1.0:
                continue

            desc = self._describe_single_action(action_id, me, opp)
            if desc:
                descriptions[action_id] = desc

        return descriptions

    def _describe_single_action(self, action_id: int, me: Player, opp: Player) -> Optional[str]:
        """Return a human-readable description for a single action ID."""
        if self.current_phase == GamePhase.Mulligan:
            if action_id == 0:
                return "Keep opening hand"
            if action_id == 1:
                return "Mulligan (redraw 5)"

        # Play card from hand (0-29)
        if 0 <= action_id <= 29:
            idx = action_id
            if idx < len(me.hand_cards):
                card = me.hand_cards[idx]
                name = card.card_names[0] if card.card_names else card.card_id
                return f"Play {name} from hand"
            return f"Play hand[{idx}]"

        # Trash from hand (30-59)
        if 30 <= action_id <= 59:
            idx = action_id - 30
            if idx < len(me.hand_cards):
                card = me.hand_cards[idx]
                name = card.card_names[0] if card.card_names else card.card_id
                return f"Trash {name} from hand"
            return f"Trash hand[{idx}]"

        # Hatch (60)
        if action_id == 60:
            return "Hatch from egg deck"

        # Move from breeding (61)
        if action_id == 61:
            if me.breeding_area and me.breeding_area.top_card:
                name = me.breeding_area.top_card.card_names[0] if me.breeding_area.top_card.card_names else "Digimon"
                return f"Move {name} from breeding area"
            return "Move from breeding area"

        # Pass / decline (62)
        if action_id == 62:
            phase = self.current_phase
            if phase == GamePhase.Main:
                return "Pass turn"
            elif phase == GamePhase.Breeding:
                return "Pass breeding"
            elif phase == GamePhase.BlockTiming:
                return "Decline to block"
            elif phase == GamePhase.AllianceTiming:
                return "Done selecting allies"
            elif phase == GamePhase.EndOfTurnAction:
                return "End turn"
            return "Decline / Pass"

        # DNA Digivolve (63-92)
        if 63 <= action_id <= 92:
            idx = action_id - 63
            if idx < len(me.hand_cards):
                card = me.hand_cards[idx]
                name = card.card_names[0] if card.card_names else card.card_id
                return f"DNA Digivolve with {name}"
            return f"DNA Digivolve hand[{idx}]"

        # Actions 100-113: phase-dependent (Block / Alliance / Attack)
        if 100 <= action_id <= 100 + FIELD_SLOTS - 1:
            slot = action_id - 100
            if self.current_phase == GamePhase.BlockTiming:
                if slot < len(me.battle_area) and me.battle_area[slot].top_card:
                    name = me.battle_area[slot].top_card.card_names[0]
                    return f"Block with {name} (slot {slot})"
                return f"Block with slot {slot}"
            elif self.current_phase == GamePhase.AllianceTiming:
                if slot < len(me.battle_area) and me.battle_area[slot].top_card:
                    name = me.battle_area[slot].top_card.card_names[0]
                    return f"Alliance with {name} (slot {slot})"
                return f"Alliance with slot {slot}"
            # Fall through to attack formula for Main/EndOfTurnAction

        # Attack (100-399)
        if 100 <= action_id <= 399:
            normalized = action_id - 100
            attacker_idx = normalized // TARGETS_PER_ATTACKER
            target_idx = normalized % TARGETS_PER_ATTACKER
            attacker_name = "?"
            if attacker_idx < len(me.battle_area):
                a = me.battle_area[attacker_idx]
                attacker_name = a.top_card.card_names[0] if a.top_card and a.top_card.card_names else "Digimon"
            if target_idx == SECURITY_TARGET:
                return f"Attack player with {attacker_name}"
            elif target_idx < len(opp.battle_area):
                t = opp.battle_area[target_idx]
                target_name = t.top_card.card_names[0] if t.top_card and t.top_card.card_names else "Digimon"
                return f"Attack {target_name} with {attacker_name}"
            return f"Attack target[{target_idx}] with {attacker_name}"

        # Digivolve (400-999)
        if 400 <= action_id <= 999:
            normalized = action_id - 400
            hand_idx = normalized // FIELDS_PER_HAND
            field_idx = normalized % FIELDS_PER_HAND
            hand_name = "?"
            field_name = "?"
            if hand_idx < len(me.hand_cards):
                c = me.hand_cards[hand_idx]
                hand_name = c.card_names[0] if c.card_names else c.card_id
            if field_idx < len(me.battle_area):
                p = me.battle_area[field_idx]
                field_name = p.top_card.card_names[0] if p.top_card and p.top_card.card_names else "Digimon"
            elif field_idx == BREEDING_SLOT and me.breeding_area is not None:
                p = me.breeding_area
                field_name = p.top_card.card_names[0] if p.top_card and p.top_card.card_names else "Breeding Digimon"
            return f"Digivolve {hand_name} onto {field_name}"

        # Effect activation (1000-1999): Training=0, Delay=1
        if 1000 <= action_id <= 1999:
            normalized = action_id - 1000
            perm_idx = normalized // EFFECTS_PER_PERM
            effect_idx = normalized % EFFECTS_PER_PERM
            perm_name = "?"
            if perm_idx < len(me.battle_area):
                p = me.battle_area[perm_idx]
                perm_name = p.top_card.card_names[0] if p.top_card and p.top_card.card_names else "Permanent"
            elif perm_idx == BREEDING_SLOT and me.breeding_area:
                p = me.breeding_area
                perm_name = p.top_card.card_names[0] if p.top_card and p.top_card.card_names else "Breeding"
            if effect_idx == 0:
                return f"Training: {perm_name}"
            elif effect_idx == 1:
                return f"Delay: {perm_name}"
            return f"Activate effect {effect_idx} on {perm_name}"

        # Source selection (2000+)
        if 2000 <= action_id < ACTION_SPACE_SIZE:
            normalized = action_id - 2000
            field_idx = normalized // SOURCES_PER_FIELD
            source_idx = normalized % SOURCES_PER_FIELD
            return f"Select source[{source_idx}] from slot[{field_idx}]"

        # Selection phases use shared action space
        phase = self.current_phase
        if phase in (GamePhase.SelectTarget, GamePhase.SelectMaterial,
                     GamePhase.SelectHand, GamePhase.SelectReveal,
                     GamePhase.SelectTrash, GamePhase.SelectSecurity):
            return self._describe_selection_action(action_id, me, opp)

        return f"Action {action_id}"

    def _describe_selection_action(self, action_id: int, me: Player, opp: Player) -> str:
        """Describe a selection-phase action."""
        # Hand card selection (0-29)
        if 0 <= action_id <= 29:
            if action_id < len(me.hand_cards):
                c = me.hand_cards[action_id]
                name = c.card_names[0] if c.card_names else c.card_id
                return f"Select {name} from hand"
            return f"Select hand[{action_id}]"

        # Revealed cards (30-39)
        if 30 <= action_id <= 39:
            idx = action_id - 30
            if idx < len(self.revealed_cards):
                c = self.revealed_cards[idx]
                name = c.card_names[0] if c.card_names else c.card_id
                return f"Select {name} from revealed"
            return f"Select revealed[{idx}]"

        # Own security (40-49)
        if 40 <= action_id <= 49:
            return f"Select security[{action_id - 40}]"

        # Opponent security (50-59)
        if 50 <= action_id <= 59:
            return f"Select opponent security[{action_id - 50}]"

        # Decline (62)
        if action_id == 62:
            return "Decline selection"

        # Breeding area (99)
        if action_id == 99:
            if me.breeding_area and me.breeding_area.top_card:
                name = me.breeding_area.top_card.card_names[0] if me.breeding_area.top_card.card_names else "Breeding"
                return f"Select {name} from breeding"
            return "Select breeding area"

        # Own battle area (100-111)
        if 100 <= action_id <= 111:
            idx = action_id - 100
            if idx < len(me.battle_area):
                p = me.battle_area[idx]
                name = p.top_card.card_names[0] if p.top_card and p.top_card.card_names else "Permanent"
                return f"Select own {name}"
            return f"Select own slot[{idx}]"

        # Opponent battle area (112-123)
        if 112 <= action_id <= 123:
            idx = action_id - 112
            if idx < len(opp.battle_area):
                p = opp.battle_area[idx]
                name = p.top_card.card_names[0] if p.top_card and p.top_card.card_names else "Permanent"
                return f"Select opponent {name}"
            return f"Select opponent slot[{idx}]"

        # Trash selection (130-179)
        if 130 <= action_id <= 179:
            idx = action_id - 130
            if idx < len(me.trash_cards):
                c = me.trash_cards[idx]
                name = c.card_names[0] if c.card_names else c.card_id
                return f"Select {name} from trash"
            return f"Select trash[{idx}]"

        # Effect branch choice (1000-1009)
        if 1000 <= action_id <= 1009:
            return f"Choose effect option {action_id - 1000 + 1}"

        return f"Select action {action_id}"

    # ─── Action Decoder ──────────────────────────────────────────────

    def decode_action(self, action_id: int, player_id: int):
        """Decode an integer action and execute the corresponding game action.

        Mirrors C# Digimon.Core.ActionDecoder.DecodeAndExecute.
        """
        phase = self.current_phase

        if phase == GamePhase.Mulligan:
            self._decode_mulligan(action_id)
        elif phase == GamePhase.Main:
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
        elif phase == GamePhase.EndOfTurnAction:
            self._decode_end_of_turn_action(action_id)
        elif phase == GamePhase.AllianceTiming:
            self._decode_alliance(action_id)

    def _decode_mulligan(self, action_id: int):
        # 0 = keep hand, 1 = mulligan/redraw hand
        if action_id == 0:
            self.action_keep_opening_hand()
        elif action_id == 1:
            self.action_mulligan_opening_hand()

    def _decode_main(self, action_id: int):
        if 0 <= action_id <= 29:
            self.action_play_card(action_id)
        elif action_id == 62:
            self.action_pass_turn()
        elif 100 <= action_id <= 399:
            normalized = action_id - 100
            attacker_idx = normalized // TARGETS_PER_ATTACKER
            target_idx = normalized % TARGETS_PER_ATTACKER
            if target_idx == SECURITY_TARGET:
                self.action_attack_player(attacker_idx)
            else:
                self.action_attack_digimon(attacker_idx, target_idx)
        elif 63 <= action_id <= 92:
            hand_idx = action_id - 63
            self._initiate_dna_digivolve(hand_idx)
        elif 400 <= action_id <= 999:
            normalized = action_id - 400
            hand_idx = normalized // FIELDS_PER_HAND
            field_idx = normalized % FIELDS_PER_HAND
            if field_idx == BREEDING_SLOT:
                self.action_digivolve_breeding(hand_idx)
            else:
                self.action_digivolve(field_idx, hand_idx)
        elif 1000 <= action_id <= 1999:
            # Effect activation (Training=0, Delay=1 in main phase)
            normalized = action_id - 1000
            perm_idx = normalized // EFFECTS_PER_PERM
            effect_idx = normalized % EFFECTS_PER_PERM
            if perm_idx < len(self.turn_player.battle_area):
                perm = self.turn_player.battle_area[perm_idx]
                if effect_idx == 0 and perm.has_keyword('_is_training'):
                    self._execute_training(perm, self.turn_player)
                elif effect_idx == 1 and self._has_delay_effect(perm):
                    self._execute_delay(perm, self.turn_player)

    def _decode_breeding(self, action_id: int):
        if action_id == 60:
            self.action_hatch()
        elif action_id == 61:
            self.action_move_from_breeding()
        elif action_id == 62:
            self.action_breeding_pass()
        elif 1000 <= action_id <= 1999:
            # Training in breeding area (virtual slot BREEDING_SLOT)
            normalized = action_id - 1000
            perm_idx = normalized // EFFECTS_PER_PERM
            if perm_idx == BREEDING_SLOT and self.turn_player.breeding_area:
                perm = self.turn_player.breeding_area
                if perm.has_keyword('_is_training'):
                    self._execute_training(perm, self.turn_player)

    def _recover_from_stale_selection(self):
        """Guard against stale selection state after a callback.

        Two conditions are checked:
        1. Selection phase with no pending_selection → fall back to Main
        2. pending_selection with empty valid_indices and not optional → clear to avoid deadlock
        """
        if (self._is_selection_phase(self.current_phase)
                and self.pending_selection is None):
            self.current_phase = GamePhase.Main
        if (self.pending_selection is not None
                and not self.pending_selection.valid_indices
                and not getattr(self.pending_selection, 'is_optional', False)):
            self.logger.log(
                f"[Recovery] Empty valid_indices — clearing")
            self.pending_selection = None
            if self._is_selection_phase(self.current_phase):
                self.current_phase = GamePhase.Main

    @staticmethod
    def _is_selection_phase(phase: 'GamePhase') -> bool:
        """Return True if the phase is a selection/interrupt phase."""
        return phase in (
            GamePhase.SelectTarget, GamePhase.SelectMaterial,
            GamePhase.SelectTrash, GamePhase.SelectSource,
            GamePhase.SelectHand, GamePhase.SelectReveal,
            GamePhase.SelectEffectChoice, GamePhase.SelectSecurity,
        )

    def _decode_selection(self, action_id: int):
        """Handle target or material selection from an effect callback."""
        ps = self.pending_selection
        if ps is None:
            return

        # Optional selection: action 62 = decline/pass
        if action_id == 62 and getattr(ps, 'is_optional', False):
            prev_phase = ps.previous_phase
            on_decline = ps.on_decline
            self.pending_selection = None
            self.revealed_cards = []  # clear any revealed cards
            self.current_phase = prev_phase
            self.active_player = None
            if on_decline:
                on_decline()
            self._recover_from_stale_selection()
            self._check_deferred_turn_end()
            return

        if ps.valid_indices and action_id not in ps.valid_indices:
            return  # invalid selection, ignore

        callback = ps.callback
        prev_phase = ps.previous_phase
        self.pending_selection = None
        self.current_phase = prev_phase
        self.active_player = None
        callback(action_id)
        self._recover_from_stale_selection()
        self._check_deferred_turn_end()

    def _decode_block(self, action_id: int):
        """Handle the defender's blocking decision during an attack.

        DCGO order: counter timing already resolved before blockers.
        After block decision, proceed directly to battle resolution.
        """
        pa = self.pending_attack
        if pa is None:
            return

        if action_id == 62:
            # Decline to block — resolve battle directly
            self.logger.log("[Block] Declined to block")
            self._resolve_battle()

        elif 100 <= action_id <= 100 + FIELD_SLOTS - 1:
            blocker_idx = action_id - 100
            defender = self.opponent_player

            if blocker_idx >= len(defender.battle_area):
                return

            blocker = defender.battle_area[blocker_idx]
            if not blocker.can_block(pa.attacker):
                return

            # Check CanSwitchAttackTarget (DCGO: AttackProcess.SwitchDefender line 495)
            if self.modifiers.has_modifier(pa.attacker, ModifierType.CANNOT_SWITCH_ATTACK_TARGET):
                self.logger.log(f"[Block] Attack target cannot be switched!")
                self._resolve_battle()
                return

            # Suspend the blocker and redirect the attack
            blocker.suspend()
            pa.is_blocked = True
            pa.blocker = blocker
            pa.effective_target = blocker

            self.logger.log(f"[Block] {self._perm_ref(blocker)} blocks the attack")

            # Fire block-related effects
            self.execute_effects(EffectTiming.OnBlockAnyone, {"blocker": blocker})
            self.execute_effects(EffectTiming.OnAttackTargetChanged, {"attacker": pa.attacker, "new_target": blocker})
            self.execute_effects(EffectTiming.OnEndBlockDesignation, {"blocker": blocker})

            # Resolve battle (counter already resolved)
            self._resolve_battle()

    def _decode_counter(self, action_id: int):
        """Handle the defender's counter/blast digivolve decision."""
        pa = self.pending_attack
        if pa is None:
            return

        if action_id == 62:
            # Decline counter — proceed to blocker check (DCGO: counter before block)
            self.logger.log("[Counter] Declined counter")
            self._check_blockers_or_continue()

        elif 400 <= action_id <= 999:
            normalized = action_id - 400
            hand_idx = normalized // FIELDS_PER_HAND
            field_idx = normalized % FIELDS_PER_HAND

            defender = self.opponent_player

            if hand_idx >= len(defender.hand_cards):
                self._check_blockers_or_continue()
                return
            if field_idx >= len(defender.battle_area):
                self._check_blockers_or_continue()
                return

            card = defender.hand_cards[hand_idx]
            perm = defender.battle_area[field_idx]

            # Validate blast digivolve effect exists on card
            effects = card.effect_list(EffectTiming.NoTiming)
            has_blast = any(getattr(e, '_is_blast_digivolve', False) for e in effects)
            if not has_blast:
                self._check_blockers_or_continue()
                return

            # Execute blast digivolve (free cost — no memory change)
            self.logger.log(f"[Counter] Blast Digivolve: {self._card_ref(card)} onto {self._perm_ref(perm)}")

            defender.hand_cards.remove(card)
            perm.add_card_source(card)

            # Fire digivolution-related effects
            self.execute_effects(EffectTiming.OnCounterTiming, {"counter_card": card, "counter_permanent": perm})
            self.execute_effects(EffectTiming.WhenDigivolving, {"digivolved_permanent": perm})

            # Proceed to blocker check (DCGO: counter before block)
            self._check_blockers_or_continue()

    def _decode_trash_selection(self, action_id: int):
        """Handle trash card selection from an effect callback."""
        ps = self.pending_selection
        if ps is None:
            return

        if action_id == 62 and getattr(ps, 'is_optional', False):
            on_decline = ps.on_decline
            prev_phase = ps.previous_phase
            self.pending_selection = None
            self.current_phase = prev_phase
            self.active_player = None
            if on_decline:
                on_decline()
            self._recover_from_stale_selection()
            self._check_deferred_turn_end()
            return

        if SEL_TRASH_START <= action_id <= SEL_TRASH_END:
            idx = action_id - SEL_TRASH_START
            selecting = ps.selecting_player
            if idx < len(selecting.trash_cards):
                callback = ps.callback
                prev_phase = ps.previous_phase
                self.pending_selection = None
                self.current_phase = prev_phase
                self.active_player = None
                callback(idx)
                self._recover_from_stale_selection()
                self._check_deferred_turn_end()

    def _decode_source_selection(self, action_id: int):
        """Handle digivolution source selection from an effect callback."""
        ps = self.pending_selection
        if ps is None:
            return

        if action_id == 62 and getattr(ps, 'is_optional', False):
            on_decline = ps.on_decline
            prev_phase = ps.previous_phase
            self.pending_selection = None
            self.current_phase = prev_phase
            self.active_player = None
            if on_decline:
                on_decline()
            self._recover_from_stale_selection()
            self._check_deferred_turn_end()
            return

        if 2000 <= action_id < ACTION_SPACE_SIZE:
            normalized = action_id - 2000
            field_idx = normalized // SOURCES_PER_FIELD
            source_idx = normalized % SOURCES_PER_FIELD

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
                    self._recover_from_stale_selection()
                    self._check_deferred_turn_end()

    def _execute_training(self, perm: Permanent, owner: Player):
        """Execute <Training>: suspend this Digimon and place top deck card at bottom of digi stack."""
        if not owner.library_cards:
            return
        perm.suspend()
        top_card = owner.library_cards.pop(0)
        perm.add_card_source_bottom(top_card)
        self.logger.log(f"[Training] {self._perm_ref(perm)} trains: placed {self._card_ref(top_card)} at bottom of digi stack")

    def _has_delay_effect(self, perm: Permanent) -> bool:
        """Check if a permanent has a <Delay> effect that can be activated."""
        for source in perm.card_sources:
            # effect_list returns all cached effects (no timing filter on CardSource)
            all_effects = source.effect_list(EffectTiming.NoTiming)
            for effect in all_effects:
                if getattr(effect, '_is_delay', False):
                    return True
        return False

    def _get_delay_callback(self, perm: Permanent):
        """Find the delayed effect callback — the effect immediately after the _is_delay marker."""
        for source in perm.card_sources:
            effects = source.effect_list(EffectTiming.NoTiming)
            for idx, effect in enumerate(effects):
                if getattr(effect, '_is_delay', False):
                    # The delayed effect is the next effect after the marker
                    if idx + 1 < len(effects):
                        next_effect = effects[idx + 1]
                        if next_effect.on_process_callback:
                            return next_effect
        return None

    def _execute_delay(self, perm: Permanent, owner: Player):
        """Execute <Delay>: trash this card from battle area and activate the delayed effect."""
        delay_effect = self._get_delay_callback(perm)
        perm_ref = self._perm_ref(perm)

        # Trash the card from battle area
        if perm in owner.battle_area:
            owner.battle_area.remove(perm)
        for source in perm.card_sources:
            owner.trash_cards.append(source)

        self.logger.log(f"[Delay] {perm_ref} trashed from battle area to activate delayed effect")

        # Execute the delayed effect callback
        if delay_effect and delay_effect.on_process_callback:
            context = {
                "game": self,
                "player": owner,
                "permanent": perm,
                "card": delay_effect.effect_source_card,
                "turn_player": self.turn_player,
                "opponent_player": self.opponent_player,
            }
            # Check can_use_condition if present
            if delay_effect.can_use_condition is None or delay_effect.can_use_condition(context):
                self._log_effect_activation(delay_effect, EffectTiming.AfterEffectsActivate)
                delay_effect.record_activation()
                delay_effect.on_process_callback(context)

    def _decode_end_of_turn_action(self, action_id: int):
        """Handle end-of-turn keyword actions (Vortex, Overclock)."""
        if action_id == 62:
            # Decline — end the turn
            self.next_phase()
            return

        if 100 <= action_id <= 399:
            # Vortex attack against opponent Digimon
            normalized = action_id - 100
            attacker_idx = normalized // TARGETS_PER_ATTACKER
            target_idx = normalized % TARGETS_PER_ATTACKER
            if attacker_idx < len(self.turn_player.battle_area) and target_idx < len(self.opponent_player.battle_area):
                attacker = self.turn_player.battle_area[attacker_idx]
                target = self.opponent_player.battle_area[target_idx]
                self.logger.log(f"[Vortex] End-of-turn attack!")
                self.resolve_attack(attacker, target, is_vortex=True,
                                    return_phase=GamePhase.EndOfTurnAction)

        elif 1000 <= action_id <= 1999:
            # Overclock activation: select sacrifice target, then attack player
            normalized = action_id - 1000
            perm_idx = normalized // EFFECTS_PER_PERM
            if perm_idx < len(self.turn_player.battle_area):
                overclock_perm = self.turn_player.battle_area[perm_idx]
                self._initiate_overclock(overclock_perm)

    def _initiate_overclock(self, overclock_perm: Permanent):
        """Overclock step 1: select a Token or other Digimon to sacrifice."""
        valid = []
        for i, perm in enumerate(self.turn_player.battle_area):
            if perm is not overclock_perm and (perm.is_token or perm.is_digimon):
                valid.append(SEL_MY_FIELD_START + i)
        if not valid:
            return

        def on_sacrifice_selected(action_id: int):
            idx = action_id - SEL_MY_FIELD_START
            if 0 <= idx < len(self.turn_player.battle_area):
                sacrifice = self.turn_player.battle_area[idx]
                self.logger.log(f"[Overclock] Sacrificed {self._perm_ref(sacrifice)}")
                self.turn_player.delete_permanent(sacrifice)
                # Attack player without suspending
                self.logger.log(f"[Overclock] End-of-turn attack on player!")
                self.resolve_attack(overclock_perm, self.opponent_player, without_suspend=True,
                                    return_phase=GamePhase.EndOfTurnAction)

        self.request_selection(
            GamePhase.SelectTarget, self.turn_player, on_sacrifice_selected,
            valid, is_optional=True)

    def _decode_alliance(self, action_id: int):
        """Handle Alliance target selection during attack."""
        pa = self.pending_attack
        if pa is None:
            return

        if action_id == 62:
            # Decline Alliance — proceed to blocker check
            self._check_blockers_or_continue()
            return

        if 100 <= action_id <= 111:
            ally_idx = action_id - 100
            if ally_idx < len(self.turn_player.battle_area):
                ally = self.turn_player.battle_area[ally_idx]
                if ally is not pa.attacker and ally.is_digimon and not ally.is_suspended:
                    ally_dp = ally.dp or 0
                    ally.suspend()
                    pa.attacker.change_dp(ally_dp)
                    pa.attacker._temp_sa_modifier += 1
                    self.logger.log(f"[Alliance] Suspended {self._perm_ref(ally)}, adding {ally_dp} DP and SA+1")

                    # Check for more Alliance-eligible allies
                    has_more = any(
                        perm is not pa.attacker and perm.is_digimon and not perm.is_suspended
                        for perm in self.turn_player.battle_area
                    )
                    if has_more:
                        return  # Stay in AllianceTiming for another choice

            # No more allies or invalid — proceed to blocker check
            self._check_blockers_or_continue()

    # ─── Selection Request Helper ─────────────────────────────────

    def request_selection(self, phase: GamePhase, player: Player,
                          callback: Callable[[int], None],
                          valid_indices: Optional[List[int]] = None,
                          is_optional: bool = False,
                          prompt: str = "",
                          effect_choices: Optional[List[dict]] = None,
                          keyword_prompt: Optional[dict] = None,
                          on_decline: Optional[Callable[[], None]] = None):
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
            prompt: Human-readable prompt text for the UI.
            effect_choices: For SelectEffectChoice: list of {index, cardId, cardName, label}.
            keyword_prompt: For keyword triggers: {keyword, cardId, cardName}.
            on_decline: Called when player declines optional selection (action 62).
        """
        self.pending_selection = PendingSelection(
            callback=callback,
            selecting_player=player,
            previous_phase=self.current_phase,
            valid_indices=valid_indices or [],
            is_optional=is_optional,
            prompt=prompt,
            effect_choices=effect_choices,
            keyword_prompt=keyword_prompt,
            on_decline=on_decline,
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
        prompt: str = "Select an opponent's Digimon.",
    ):
        """Request selection of an opponent's permanent (for delete, suspend, bounce, etc.)."""
        opponent = self.player2 if player is self.player1 else self.player1
        valid = []
        for i, perm in enumerate(opponent.battle_area):
            # Check modifier-based targeting protection
            if not self.modifiers.can_be_selected_by_effect(perm):
                continue
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
            GamePhase.SelectTarget, player, on_select, valid, is_optional,
            prompt=prompt)

    def effect_select_own_permanent(
        self, player: Player, callback: Callable[['Permanent'], None],
        filter_fn: Optional[Callable[['Permanent'], bool]] = None,
        is_optional: bool = False,
        prompt: str = "Select one of your Digimon.",
    ):
        """Request selection of one of the player's own permanents."""
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
            GamePhase.SelectTarget, player, on_select, valid, is_optional,
            prompt=prompt)

    def effect_reveal_and_select(
        self, player: Player, count: int,
        filter_fn: Callable[['CardSource'], bool],
        on_selected: Callable[['CardSource', List['CardSource']], None],
        is_optional: bool = False,
        prompt: str = "",
    ):
        """Reveal top N cards, let agent pick one matching filter, return rest to bottom.

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

        def on_decline_reveal():
            # Declined — move all revealed to bottom of deck
            for card in revealed:
                if card in player.library_cards:
                    player.library_cards.remove(card)
                    player.library_cards.append(card)
            self.revealed_cards = []

        auto_prompt = prompt or f"Select a card from the {count} revealed cards."
        self.request_selection(
            GamePhase.SelectReveal, player, on_select, valid, is_optional,
            prompt=auto_prompt, on_decline=on_decline_reveal)

    def effect_reveal_and_select_multi(
        self, player: Player, count: int,
        passes: list,
        remaining_placement: str = 'deck_bottom',
        is_optional: bool = False,
    ):
        """Reveal top N cards, then run multiple sequential selection passes.

        Args:
            player: The player revealing cards.
            count: Number of cards to reveal from deck top.
            passes: List of (filter_fn, placement) tuples. Each pass selects
                    1 card matching filter_fn from the remaining pool.
                    placement: 'hand' or 'trash'.
            remaining_placement: Where unselected cards go after all passes.
            is_optional: If True, player can decline each pass.
        """
        revealed = player.library_cards[:count]
        if not revealed:
            return

        # Remove from library immediately
        for c in revealed:
            player.library_cards.remove(c)

        pool = list(revealed)

        def run_pass(pass_idx: int):
            if pass_idx >= len(passes) or not pool:
                # All passes done — place remaining
                self.revealed_cards = []
                for c in pool:
                    if remaining_placement == 'deck_bottom':
                        player.library_cards.append(c)
                    elif remaining_placement == 'hand':
                        player.hand_cards.append(c)
                    elif remaining_placement == 'trash':
                        player.trash_cards.append(c)
                return

            filter_fn, placement = passes[pass_idx]
            self.revealed_cards = list(pool)

            valid = []
            for i, card in enumerate(pool):
                if filter_fn(card):
                    valid.append(SEL_REVEALED_START + i)

            if not valid:
                # No valid picks for this pass — skip to next
                run_pass(pass_idx + 1)
                return

            def on_select(action_id: int):
                idx = action_id - SEL_REVEALED_START
                if 0 <= idx < len(pool):
                    selected = pool.pop(idx)
                    if placement == 'hand':
                        player.hand_cards.append(selected)
                    elif placement == 'trash':
                        player.trash_cards.append(selected)
                self.revealed_cards = []
                run_pass(pass_idx + 1)

            def on_decline_pass():
                self.revealed_cards = []
                run_pass(pass_idx + 1)

            self.request_selection(
                GamePhase.SelectReveal, player, on_select, valid, is_optional,
                prompt=f"Select a card from the revealed cards (pass {pass_idx + 1}).",
                on_decline=on_decline_pass)

        run_pass(0)

    def effect_play_token(
        self, player: Player, token_type: str,
        on_opponent_field: bool = False, count: int = 1,
    ):
        """Create and play token permanent(s) onto a player's field.

        Tokens are synthetic Digimon that cease to exist when leaving the field.
        The token's effects (e.g., On Deletion) are pre-attached via _cached_effects.

        Args:
            player: The player whose effect creates the token.
            token_type: Token type key (e.g., 'petrification').
            on_opponent_field: If True, place on opponent's field.
            count: Number of tokens to create.
        """
        from .data.token_registry import create_token_card_source

        target_player = player.enemy if on_opponent_field else player
        if target_player is None:
            return

        for _ in range(count):
            if len(target_player.battle_area) >= FIELD_SLOTS:
                self.logger.log(f"[Token] Field full — cannot play {token_type} token")
                break

            card_source = create_token_card_source(token_type, target_player)
            perm = Permanent([card_source])
            perm.turn_played = self.turn_count
            perm._owner_game = self
            target_player.battle_area.append(perm)

            # Register CANNOT_SUSPEND modifier for petrification ([Your Turn] can't suspend)
            if token_type == 'petrification':
                self.modifiers.register(ModifierEntry(
                    modifier_type=ModifierType.CANNOT_SUSPEND,
                    condition=lambda t, ctx, p=perm, o=target_player: (
                        t is p and o.is_my_turn and p in o.battle_area
                    ),
                    source_permanent=perm,
                    expiry='permanent',
                ))

            self.logger.log(
                f"[Token] {token_type.title()} Token played on "
                f"{target_player.player_name}'s field")

            self.execute_effects(
                EffectTiming.OnEnterFieldAnyone,
                {
                    "played_card": card_source,
                    "played_permanent": perm,
                    "event_permanent": perm,
                    "event_player": target_player,
                },
            )

    def effect_play_from_zone(
        self, player: Player,
        zone: str,
        filter_fn: Callable[['CardSource'], bool],
        free: bool = True,
        manual_reduction: int = 0,
        is_optional: bool = True,
        prompt: str = "",
    ):
        """Let agent pick a card from a zone to play onto the field."""
        if not prompt:
            cost_text = "without paying its cost" if free else "by paying its cost"
            prompt = f"Select a card from {zone} to play {cost_text}."
        if zone == 'hand_or_trash':
            # Combine valid cards from both hand and trash into one selection
            valid = []
            for i, card in enumerate(player.hand_cards):
                if filter_fn(card) and (SEL_HAND_START + i) < ACTION_SPACE_SIZE:
                    valid.append(SEL_HAND_START + i)
            for i, card in enumerate(player.trash_cards):
                if filter_fn(card) and (SEL_TRASH_START + i) < ACTION_SPACE_SIZE:
                    valid.append(SEL_TRASH_START + i)
            if not valid:
                return

            def on_select_hot(action_id: int):
                if SEL_TRASH_START <= action_id:
                    idx = action_id - SEL_TRASH_START
                    source = player.trash_cards
                    src_name = 'trash'
                else:
                    idx = action_id - SEL_HAND_START
                    source = player.hand_cards
                    src_name = 'hand'
                if 0 <= idx < len(source):
                    card = source[idx]
                    played_perm = player.play_card_from_source(card, pay_cost=not free)
                    if free:
                        cost = 0
                    else:
                        cost = self.calculate_play_cost(
                            player,
                            card,
                            source_zone=src_name,
                            free=False,
                            manual_reduction=manual_reduction,
                        )
                        player.lose_memory(cost)
                        if hasattr(player, "_temp_play_cost_reduction"):
                            player._temp_play_cost_reduction = 0
                    self.logger.log(f"[Effect] {player.player_name} played "
                                    f"{self._card_ref(card)} from {src_name}")
                    if card.is_option:
                        self.execute_effects(
                            EffectTiming.OnUseOption,
                            {"played_card": card, "played_permanent": played_perm, "event_player": player},
                        )
                    self.execute_effects(
                        EffectTiming.OnEnterFieldAnyone,
                        {"played_card": card, "played_permanent": played_perm, "event_player": player},
                    )
                    if card.is_option and not self._option_stays_on_field(card):
                        self._trash_option_after_resolution(player, played_perm)

            self.request_selection(GamePhase.SelectTarget, player,
                                   on_select_hot, valid, is_optional,
                                   prompt=prompt)
            return

        if zone == 'hand':
            source_list = player.hand_cards
            offset = SEL_HAND_START
        elif zone == 'trash':
            source_list = player.trash_cards
            offset = SEL_TRASH_START
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
                played_perm = player.play_card_from_source(card, pay_cost=not free)
                if free:
                    cost = 0
                else:
                    cost = self.calculate_play_cost(
                        player,
                        card,
                        source_zone=zone,
                        free=False,
                        manual_reduction=manual_reduction,
                    )
                    player.lose_memory(cost)
                    if hasattr(player, "_temp_play_cost_reduction"):
                        player._temp_play_cost_reduction = 0
                self.logger.log(f"[Effect] {player.player_name} played "
                                f"{self._card_ref(card)} from {zone}")
                if card.is_option:
                    self.execute_effects(
                        EffectTiming.OnUseOption,
                        {"played_card": card, "played_permanent": played_perm, "event_player": player},
                    )
                self.execute_effects(
                    EffectTiming.OnEnterFieldAnyone,
                    {"played_card": card, "played_permanent": played_perm, "event_player": player},
                )
                if card.is_option and not self._option_stays_on_field(card):
                    self._trash_option_after_resolution(player, played_perm)

        phase = GamePhase.SelectReveal if zone == 'revealed' else GamePhase.SelectTarget
        self.request_selection(phase, player, on_select, valid, is_optional,
                               prompt=prompt)

    def effect_digivolve_from_hand(
        self, player: Player, permanent: 'Permanent',
        filter_fn: Callable[['CardSource'], bool],
        cost_override: Optional[int] = None,
        cost_reduction: int = 0,
        ignore_requirements: bool = False,
        is_optional: bool = True,
        prompt: str = "",
    ):
        """Let agent pick a hand card to digivolve a permanent into via effect."""
        if not prompt:
            perm_name = self._perm_ref(permanent)
            prompt = f"Select a card from hand to digivolve {perm_name}."
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
                permanent.turn_digivolved = self.turn_count  # Track for <Blitz>
                player.lose_memory(cost)
                self.logger.log(
                    f"[Effect Digivolve] {self._card_ref(card)} onto "
                    f"{self._perm_ref(permanent)} "
                    f"(cost: {cost})")
                # Draw 1 (digivolution bonus)
                player.draw()
                self.execute_effects(EffectTiming.WhenDigivolving,
                                     {"digivolved_permanent": permanent})

        self.request_selection(
            GamePhase.SelectTarget, player, on_select, valid, is_optional,
            prompt=prompt)

    def effect_select_hand_card(
        self, player: Player,
        filter_fn: Callable[['CardSource'], bool],
        callback: Callable[['CardSource'], None],
        is_optional: bool = False,
        prompt: str = "Select a card from your hand.",
    ):
        """Let agent pick a card from hand (for trash-as-cost, discard, etc.)."""
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
            GamePhase.SelectHand, player, on_select, valid, is_optional,
            prompt=prompt)

    def effect_choose_branch(
        self, player: Player, num_choices: int,
        callback: Callable[[int], None],
        prompt: str = "Choose an effect to activate.",
        branch_labels: Optional[List[str]] = None,
    ):
        """Let agent choose between N effect branches."""
        valid = [SEL_EFFECT_CHOICE_START + i for i in range(num_choices)]

        def on_select(action_id: int):
            branch = action_id - SEL_EFFECT_CHOICE_START
            if 0 <= branch < num_choices:
                callback(branch)

        self.request_selection(
            GamePhase.SelectEffectChoice, player, on_select, valid,
            prompt=prompt)

    def effect_select_own_security(
        self, player: Player,
        filter_fn: Callable[['CardSource'], bool],
        callback: Callable[['CardSource'], None],
        is_optional: bool = True,
        prompt: str = "Select a card from your security stack.",
    ):
        """Let agent select a card from their own security stack."""
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
            GamePhase.SelectSecurity, player, on_select, valid, is_optional,
            prompt=prompt)

    def effect_select_opponent_security(
        self, player: Player,
        filter_fn: Optional[Callable[['CardSource'], bool]],
        callback: Callable[['CardSource'], None],
        is_optional: bool = True,
        prompt: str = "Select a card from opponent's security stack.",
    ):
        """Let agent select a card from the opponent's security stack."""
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
            GamePhase.SelectSecurity, player, on_select, valid, is_optional,
            prompt=prompt)

    def effect_link_to_permanent(
        self, player: Player, card_to_link: 'CardSource',
        filter_fn: Optional[Callable[['Permanent'], bool]] = None,
        is_optional: bool = True,
        prompt: str = "",
    ):
        """Let agent choose a Digimon to link an option card to (sideways attach)."""
        if not prompt:
            prompt = f"Select a Digimon to link {self._card_ref(card_to_link)} to."
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
        if ba is not None and ba.is_digimon and (ba.level or 0) > 2:
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
                f"[Link] {self._card_ref(card_to_link)} linked to "
                f"{self._perm_ref(target)}")

        self.request_selection(
            GamePhase.SelectTarget, player, on_select, valid, is_optional,
            prompt=prompt)

    # ─── DNA Digivolve ────────────────────────────────────────────────

    def effect_dna_digivolve_from_hand(
        self, player: Player,
        filter_fn: Callable[['CardSource'], bool],
        is_optional: bool = True,
        prompt: str = "",
    ):
        """Let an effect trigger a DNA digivolve from hand.

        Searches the player's hand for a card matching filter_fn that has
        dna_costs, then enters the SelectMaterial flow to pick 2 field targets.
        Requires 2+ valid Digimon on the battle area.
        """
        # Find matching DNA cards in hand
        candidates = []
        for i, card in enumerate(player.hand_cards):
            if not filter_fn(card):
                continue
            if not card.is_digimon or not card.c_entity_base:
                continue
            if not card.c_entity_base.dna_costs:
                continue
            if has_valid_dna_targets(card, player.battle_area):
                candidates.append(i)
        if not candidates:
            return

        if len(candidates) == 1:
            # Only one matching card — go straight to target selection
            self._effect_dna_pick_first(player, candidates[0])
        else:
            # Multiple matching cards — let agent pick which one
            valid = [SEL_HAND_START + i for i in candidates]
            if not prompt:
                prompt = "Select a card from hand to DNA digivolve."
            def on_pick_card(action_id: int):
                hand_idx = action_id - SEL_HAND_START
                self._effect_dna_pick_first(player, hand_idx)
            self.request_selection(
                GamePhase.SelectTarget, player, on_pick_card, valid,
                is_optional, prompt=prompt)

    def _effect_dna_pick_first(self, player: Player, hand_idx: int):
        """Effect DNA step 1: pick first field target."""
        if hand_idx >= len(player.hand_cards):
            return
        card = player.hand_cards[hand_idx]
        valid_first = get_valid_dna_first_targets(card, player.battle_area)
        if not valid_first:
            return

        # Offset field indices to SEL_MY_FIELD_START range
        valid = [SEL_MY_FIELD_START + i for i in valid_first]

        def on_first(action_id: int):
            first_field_idx = action_id - SEL_MY_FIELD_START
            self._effect_dna_pick_second(player, hand_idx, first_field_idx)

        self.request_selection(
            GamePhase.SelectMaterial, player, on_first, valid,
            is_optional=False,
            prompt="Select first Digimon for DNA digivolve.")

    def _effect_dna_pick_second(self, player: Player, hand_idx: int,
                                 first_field_idx: int):
        """Effect DNA step 2: pick second field target."""
        if hand_idx >= len(player.hand_cards):
            return
        if first_field_idx >= len(player.battle_area):
            return
        card = player.hand_cards[hand_idx]
        valid_second = get_valid_dna_second_targets(
            card, first_field_idx, player.battle_area)
        if not valid_second:
            return

        valid = [SEL_MY_FIELD_START + i for i in valid_second]

        def on_second(action_id: int):
            second_field_idx = action_id - SEL_MY_FIELD_START
            self._effect_dna_execute(player, hand_idx, first_field_idx,
                                      second_field_idx)

        self.request_selection(
            GamePhase.SelectMaterial, player, on_second, valid,
            is_optional=False,
            prompt="Select second Digimon for DNA digivolve.")

    def _effect_dna_execute(self, player: Player, hand_idx: int,
                             first_field_idx: int, second_field_idx: int):
        """Effect DNA step 3: execute the DNA digivolve."""
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

        self.logger.log(
            f"[Effect DNA Digivolve] {self._card_ref(card)} from "
            f"{self._perm_ref(top_perm)} + {self._perm_ref(bottom_perm)} "
            f"(cost: {dna_cost.memory_cost})")

        cost = player.dna_digivolve(top_perm, bottom_perm, card, dna_cost)
        player.lose_memory(cost)

        new_perm = player.battle_area[-1] if player.battle_area else None
        self.execute_effects(EffectTiming.WhenDigivolving, {
            "digivolved_permanent": new_perm, "is_dna_digivolve": True})

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

        self.logger.log(
            f"[DNA Digivolve] {self._card_ref(card)} from "
            f"{self._perm_ref(top_perm)} + {self._perm_ref(bottom_perm)} (cost: {dna_cost.memory_cost})"
        )

        cost = player.dna_digivolve(top_perm, bottom_perm, card, dna_cost)
        player.lose_memory(cost)

        # Find the new permanent (last in battle area after dna_digivolve)
        new_perm = player.battle_area[-1] if player.battle_area else None

        self.execute_effects(EffectTiming.WhenDigivolving, {
            "digivolved_permanent": new_perm, "is_dna_digivolve": True})
        self.check_turn_end()
