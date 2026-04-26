from __future__ import annotations
from typing import TYPE_CHECKING, Optional, Union, List, Dict, Any, Callable
import random

import numpy as np

from ..data.enums import GamePhase, EffectTiming, AttackResolution, PendingAction
from ..core.player import Player
from ..core.permanent import Permanent
from ..loggers import IGameLogger, SilentLogger
from ..events import GameEvent
from ..interfaces.modifiers import ModifierRegistry, ModifierType, ModifierEntry

# ─── Re-export everything from constants for backward compatibility ──
from .constants import *  # noqa: F401,F403
from .constants import (
    FIELD_SLOTS, EFFECTS_PER_PERM, TENSOR_SIZE, ACTION_SPACE_SIZE,
    BREEDING_SLOT, SECURITY_TARGET, DP_NORM,
    TriggeredEffect, PendingAttack, PendingSelection,
    _GLOBAL, _MY_BATTLE, _OPP_BATTLE, _MY_HAND, _OPP_HAND,
    _MY_TRASH, _OPP_TRASH, _MY_SECURITY, _OPP_SECURITY,
    _MY_BREEDING, _OPP_BREEDING, _REVEALED, _SELECTION,
)

# ─── Import extracted modules ──
from . import serialization as game_serialization
from . import tensor as game_tensor
from . import action_describe as game_action_describe
from . import action_mask as game_action_mask

# ─── Import mixins ──
from .combat import CombatMixin
from .action_decoder import ActionDecoderMixin
from .effects import EffectHelpersMixin

if TYPE_CHECKING:
    from ..core.card_source import CardSource


class Game(CombatMixin, ActionDecoderMixin, EffectHelpersMixin):
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
        self._turn_end_deferred: bool = False
        # Deferred end-phase: set when OnEndTurn effects create pending selections.
        self._end_phase_deferred: bool = False
        self._end_phase_memory_before: int = 0

        # Opening mulligan state (set during start_game()).
        self._mulligan_order: List[Player] = []
        self._mulligan_index: int = 0
        self._mulligan_used: Dict[int, bool] = {}

        # Structured event sequence counter
        self._event_seq: int = 0

        # DigiXros pending state: tracks card index, selected materials, and cost
        self._pending_digixros: Optional[dict] = None

        # Deletion observer recursion depth guard (prevents token chain infinite loops)
        self._deletion_depth: int = 0

        # Deferred move-to-main: set when OnMove effects create pending selections.
        # Cleared by _maybe_complete_move_to_main() once selection resolves.
        self._move_to_main_deferred: bool = False

        # Active effect stack — tracks the ICardEffect currently being resolved
        # so downstream mutations (e.g. add_memory) can inspect the source effect
        # (used by CANNOT_ADD_MEMORY "except Tamer effects" gating, etc.).
        self._active_effect_stack: List['ICardEffect'] = []

    @property
    def current_effect(self):
        """Return the ICardEffect currently being processed (top of stack), or None."""
        return self._active_effect_stack[-1] if self._active_effect_stack else None

    def _invoke_effect_callback(self, effect, context):
        """Invoke an effect's on_process_callback while tracking it on the active stack.

        Scripts can inspect ``game.current_effect`` during processing (or in
        subsequently-triggered hooks like ``Player.add_memory``) to know which
        effect is the source of the mutation. Uses try/finally so that the
        stack is always balanced even if the callback raises.
        """
        if effect is None or effect.on_process_callback is None:
            return
        self._active_effect_stack.append(effect)
        try:
            effect.on_process_callback(context)
        finally:
            # Defensive pop — handle the rare case where the callback itself
            # mutated the stack (should not happen, but keep balanced).
            if self._active_effect_stack and self._active_effect_stack[-1] is effect:
                self._active_effect_stack.pop()
            elif effect in self._active_effect_stack:
                self._active_effect_stack.remove(effect)

    @property
    def current_player_id(self) -> int:
        """Return the player_id of the active player."""
        if self.active_player is not None:
            return self.active_player.player_id
        return self.turn_player.player_id

    # ─── Game Setup & Phase Management ─────────────────────────────

    def start_game(self):
        if random.choice([True, False]):
            self.turn_player = self.player1
            self.opponent_player = self.player2
        else:
            self.turn_player = self.player2
            self.opponent_player = self.player1

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
            self.current_phase = GamePhase.End
            self.switch_turn()
            self.phase_start()

    def phase_start(self):
        self.current_phase = GamePhase.Start
        self.logger.log(f"=== Turn {self.turn_count} — {self.turn_player.player_name} ===")
        self._emit('phase_change', phase='Start', turn=self.turn_count)
        self.turn_player.unsuspend_all(skip_reboot=True)
        self.opponent_player.unsuspend_reboot_only()
        self._reset_effect_turn_counts()
        self._clear_temp_dp()
        self.modifiers.clear_expiry('end_of_turn')
        self.modifiers.clear_opponent_turn_expiry(self.turn_player)
        self.clear_expired_granted_effects()
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
        memory_before = self.memory
        self.execute_effects(EffectTiming.OnEndTurn)
        # If OnEndTurn effects created pending selections, defer phase completion
        if self.pending_selection is not None:
            self._end_phase_deferred = True
            self._end_phase_memory_before = memory_before
            return
        self._complete_end_phase(memory_before)

    def _complete_end_phase(self, memory_before: int):
        """Finish the end phase after all OnEndTurn selections have resolved."""
        # DCGO: if OnEndTurn effects swung memory back (turn player regained memory
        # after it was negative), the turn continues — return to Main Phase.
        # Only applies when memory WAS negative before OnEndTurn effects.
        if memory_before < 0 and self.memory >= 0 and not self.game_over:
            self.logger.log(f"[Memory Swing-Back] Memory restored to {self.memory} during end phase — returning to Main")
            self.current_phase = GamePhase.Main
            self.phase_main()
            return
        if self._has_end_of_turn_keywords():
            self.current_phase = GamePhase.EndOfTurnAction
            return  # Park for agent decision
        self.next_phase()

    def _maybe_complete_end_phase(self):
        """Complete the end phase if it was deferred while waiting for selections."""
        if self._end_phase_deferred and self.pending_selection is None:
            mb = self._end_phase_memory_before
            self._end_phase_deferred = False
            self._complete_end_phase(mb)

    def _maybe_complete_move_to_main(self):
        """Complete the move-to-main transition if it was deferred for OnMove selections."""
        if self._move_to_main_deferred and self.pending_selection is None:
            self._move_to_main_deferred = False
            self.current_phase = GamePhase.Main
            self.phase_main()

    def _has_end_of_turn_keywords(self) -> bool:
        """Check if the turn player has any Digimon with Vortex, Overclock, or MAY_ATTACK."""
        from ..interfaces.modifiers import ModifierType
        for perm in self.turn_player.battle_area:
            if not perm.is_digimon:
                continue
            if perm.has_keyword('_is_vortex') and perm.can_attack(is_vortex=True):
                return True
            if perm.has_keyword('_is_overclock'):
                has_sacrifice = any(
                    p is not perm and (p.is_token or p.is_digimon)
                    for p in self.turn_player.battle_area
                )
                if has_sacrifice:
                    return True
            if self.modifiers.has_modifier(perm, ModifierType.MAY_ATTACK) and perm.can_attack():
                return True
        return False

    def switch_turn(self):
        self.turn_player, self.opponent_player = self.opponent_player, self.turn_player
        self.turn_count += 1
        self.memory = -self.memory
        self.turn_player.is_my_turn = True
        self.opponent_player.is_my_turn = False
        self._turn_end_deferred = False
        self._end_phase_deferred = False
        self._move_to_main_deferred = False

    def pass_turn(self):
        if self.memory >= 0:
            self.memory = -3
        self.execute_effects(EffectTiming.OnEndMainPhase)
        self.current_phase = GamePhase.End
        self.phase_end()

    def check_turn_end(self):
        if self.memory < 0:
            if self.pending_selection is not None:
                self._turn_end_deferred = True
                return
            # DCGO fires OnEndMainPhase when memory crosses, not just on voluntary pass
            self.execute_effects(EffectTiming.OnEndMainPhase)
            self.current_phase = GamePhase.End
            self.phase_end()

    def _check_deferred_turn_end(self):
        """End the turn if it was deferred while waiting for a selection."""
        if self._turn_end_deferred and self.pending_selection is None:
            self._turn_end_deferred = False
            self.check_turn_end()

    # ─── Logging & Reference Utilities ──────────────────────────────

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
        commit: bool = False,
    ) -> int:
        if free:
            return 0

        base_cost = card.get_cost_itself
        reduction = max(0, manual_reduction)
        activated_effects: list = []

        # Check for passive "players can't reduce play costs" declarative
        # effects on either player's battle area (e.g. ST13-08 Chikurimon).
        # If any such effect is active, cost reduction is globally disabled.
        cannot_reduce = False
        for entry in self.modifiers._modifiers.get(ModifierType.CANNOT_REDUCE_COST, []):
            if entry.is_active(None, None):
                cannot_reduce = True
                break
        if not cannot_reduce:
            for scan_player in (self.player1, self.player2):
                for scan_perm in scan_player.battle_area:
                    for scan_effect in scan_perm.effect_list(EffectTiming.NoTiming):
                        if not getattr(scan_effect, '_blocks_cost_reduction', False):
                            continue
                        if scan_effect.can_use_condition and not scan_effect.can_use_condition({}):
                            continue
                        cannot_reduce = True
                        break
                    if cannot_reduce:
                        break
                if cannot_reduce:
                    break

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
                activated_effects.append(effect)
                return
            effect_reduction = getattr(effect, 'cost_reduction', 0)
            if effect_reduction:
                reduction += max(0, int(effect_reduction))
                activated_effects.append(effect)

        for perm in list(player.battle_area):
            for effect in perm.effect_list(EffectTiming.NoTiming):
                effect.set_effect_source_permanent(perm)
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
                effect.set_effect_source_permanent(breeding_perm)
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
            effect.set_effect_source_permanent(card.permanent_of_this_card())
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

        # Apply CANNOT_REDUCE_COST lockout: players can't reduce play costs.
        # Manual reductions (from costs being "paid" via other means) still apply.
        if cannot_reduce:
            reduction = max(0, manual_reduction)

        if commit:
            for effect in activated_effects:
                effect.record_activation()
                # Fire BeforePayCost process callbacks (e.g., trash-to-deck returns)
                if effect.on_process_callback:
                    ctx = {
                        "game": self,
                        "player": player,
                        "card_source": card,
                        "played_card": card,
                    }
                    try:
                        self._invoke_effect_callback(effect, ctx)
                    except Exception:
                        pass

        return max(0, base_cost - reduction)

    def execute_effects(self, timing: EffectTiming, extra_context: Optional[dict] = None):
        """Execute all effects matching the given timing with stack-based resolution."""
        stack = self._collect_triggered_effects(timing, extra_context)
        if not stack:
            return
        self._resolve_effect_stack(stack, timing, extra_context)

    def _collect_triggered_effects(
        self, timing: EffectTiming, extra_context: Optional[dict] = None
    ) -> List[TriggeredEffect]:
        """Collect all activatable effects for a timing into a sorted stack.

        DCGO scans 5 zones per player (turn player first): player effects, field
        permanents, trash cards, hand cards, face-up security. We scan field
        permanents (battle area + breeding) and now also trash/hand/security cards.
        """
        all_perms: List[Permanent] = list(self.turn_player.battle_area) + list(self.opponent_player.battle_area)
        if self.turn_player.breeding_area is not None:
            all_perms.append(self.turn_player.breeding_area)
        if self.opponent_player.breeding_area is not None:
            all_perms.append(self.opponent_player.breeding_area)

        played_card = extra_context.get('played_card') if extra_context else None
        digivolved_perm = extra_context.get('digivolved_permanent') if extra_context else None

        stack: List[TriggeredEffect] = []

        # 1. Field permanents (battle area + breeding) — existing behavior
        for perm in all_perms:
            effects = perm.effect_list(timing)
            for effect in effects:
                effect.set_effect_source_permanent(perm)
                if not effect.on_process_callback:
                    continue
                if not effect.can_activate_this_turn():
                    continue
                if not self._effect_matches_timing(effect, timing, perm, played_card, digivolved_perm, extra_context):
                    continue
                # DCGO: IDisableCardEffect check — skip if effect is disabled
                if self.modifiers.has_modifier(perm, ModifierType.DISABLE_EFFECT, {'effect': effect}):
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

        # 2. Non-field zones: trash, hand, face-up security (DCGO order: turn player first)
        #    Skip OnDeclaration (active [Trash][Main] effects — need action mask, not auto-trigger)
        #    and flag-based timings (on_play, when_digivolving, on_attack) which only fire from field.
        if timing != EffectTiming.OnDeclaration:
            for player in [self.turn_player, self.opponent_player]:
                is_tp = (player is self.turn_player)
                zone_cards = [
                    (player.trash_cards, 'trash'),
                    (player.hand_cards, 'hand'),
                    (player.security_cards, 'security'),
                ]
                for card_list, zone_name in zone_cards:
                    for card_source in list(card_list):
                        effects = card_source.effect_list(timing)
                        for effect in effects:
                            if not effect.on_process_callback:
                                continue
                            if not effect.can_activate_this_turn():
                                continue
                            # Zone cards use direct timing match only — skip flag-based checks
                            if effect.timing != timing:
                                continue
                            # Skip inherited effects (only active under a Digimon on field)
                            if effect.is_inherited_effect:
                                continue

                            effect.set_effect_source_permanent(None)
                            context = {
                                "game": self,
                                "player": player,
                                "permanent": None,
                                "card": card_source,
                                "turn_player": self.turn_player,
                                "opponent_player": self.opponent_player,
                            }
                            if extra_context:
                                for key, value in extra_context.items():
                                    if key in {"player", "permanent"}:
                                        continue
                                    context[key] = value

                            stack.append(TriggeredEffect(
                                effect=effect,
                                permanent=None,
                                owner=player,
                                context=context,
                                is_turn_player=is_tp,
                                source_zone=zone_name,
                                source_card=card_source,
                            ))

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
        """Resolve a stack of triggered effects, handling chain triggers."""
        max_chain = 50

        for te in stack:
            if not self._triggered_effect_still_valid(te):
                continue
            if not te.effect.can_activate_this_turn():
                continue
            if te.effect.can_use_condition is not None and not te.effect.can_use_condition(te.context):
                continue

            self._log_effect_activation(te.effect, timing)
            te.effect.record_activation()
            self._invoke_effect_callback(te.effect, te.context)
            self._rule_process()
            if self.game_over:
                return

            chain_count = 0
            while chain_count < max_chain:
                chain_stack = self._collect_triggered_effects(timing, extra_context)
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
                    self._invoke_effect_callback(cte.effect, cte.context)
                    self._rule_process()
                    if self.game_over:
                        return
                chain_count += 1

    @staticmethod
    def _triggered_effect_still_valid(te: TriggeredEffect) -> bool:
        """Check that a triggered effect's source is still in a valid state."""
        # Non-field zone effects: verify card is still in its source zone
        if te.source_zone != 'field':
            owner = te.owner
            card = te.source_card
            if card is None or owner is None:
                return False
            if te.source_zone == 'trash':
                return card in owner.trash_cards
            elif te.source_zone == 'hand':
                return card in owner.hand_cards
            return True

        # Field effects: verify permanent is still on field
        perm = te.permanent
        if perm is None or perm.top_card is None:
            return False
        owner = te.owner
        if hasattr(owner, 'battle_area') and perm not in owner.battle_area:
            if hasattr(owner, 'breeding_area') and perm is not owner.breeding_area:
                return False
        return True

    @staticmethod
    def _effect_matches_timing(
        effect, timing: 'EffectTiming', perm: 'Permanent',
        played_card, digivolved_perm, extra_context=None,
    ) -> bool:
        """Check whether an effect should fire for the given timing."""
        # Security effects only fire during security checks — never on field enter
        if getattr(effect, 'is_security_effect', False):
            return timing == EffectTiming.OnSecurityCheck or timing == EffectTiming.SecuritySkill

        # Suppress On Play: when _suppress_on_play is set in context, skip on_play effects
        if effect.is_on_play and extra_context and extra_context.get('_suppress_on_play'):
            return False

        has_flag = (effect.is_on_play or effect.is_when_digivolving
                    or effect.is_on_attack or effect.is_on_deletion)

        if not has_flag:
            if getattr(effect, 'timing', None) is not None:
                if effect.timing == timing:
                    return True
                if (effect.timing == EffectTiming.OptionSkill
                        and timing == EffectTiming.OnUseOption):
                    return (
                        played_card is not None
                        and perm.top_card is played_card
                    )
                if (effect.timing == EffectTiming.SecuritySkill
                        and timing == EffectTiming.OnSecurityCheck):
                    return True
                return False

            if timing == EffectTiming.OnEnterFieldAnyone:
                return (
                    played_card is not None
                    and perm.top_card is played_card
                    and (effect.effect_source_card is None or effect.effect_source_card is perm.top_card)
                )
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

        if timing == EffectTiming.OnUseAttack:
            attacker = extra_context.get('attacker') if extra_context else None
            return effect.is_on_attack and attacker is not None and perm is attacker

        if timing == EffectTiming.OnAllyAttack:
            attacker = extra_context.get('attacker') if extra_context else None
            return effect.is_on_attack and attacker is not None and perm is not attacker

        if timing == EffectTiming.OnDestroyedAnyone:
            return effect.is_on_deletion

        return False

    def execute_deletion_effects(self, deleted_permanent: Permanent, owner: Player,
                                  removal_cause: str = 'effect'):
        """Execute OnDeletion effects for a permanent that was just deleted."""
        _MAX_DELETION_DEPTH = 8
        if self._deletion_depth >= _MAX_DELETION_DEPTH:
            self.logger.log(
                f"[WARNING] Deletion observer depth limit ({_MAX_DELETION_DEPTH}) reached, "
                f"skipping effects for {deleted_permanent}")
            return
        self._deletion_depth += 1
        try:
            self._execute_deletion_effects_inner(deleted_permanent, owner, removal_cause=removal_cause)
        finally:
            self._deletion_depth -= 1

    def _execute_deletion_effects_inner(self, deleted_permanent: Permanent, owner: Player,
                                         removal_cause: str = 'effect'):
        """Inner implementation of execute_deletion_effects."""
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
                    "removal_cause": removal_cause,
                }
                stack.append(TriggeredEffect(
                    effect=effect,
                    permanent=deleted_permanent,
                    owner=owner,
                    context=context,
                    is_turn_player=is_tp,
                ))

        stack.sort(key=lambda te: (1 if te.effect.is_optional else 0))

        for te in stack:
            if te.effect.can_use_condition is not None and not te.effect.can_use_condition(te.context):
                continue
            self._log_effect_activation(te.effect, EffectTiming.OnDestroyedAnyone)
            te.effect.record_activation()
            self._invoke_effect_callback(te.effect, te.context)

        self._fire_deletion_observers(deleted_permanent, owner, removal_cause=removal_cause)

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
        """Register a continuous modifier on a permanent."""
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
        """Remove all modifiers sourced from a permanent that left the field.
        Also removes any granted effects that were sourced from this permanent."""
        self.modifiers.unregister_by_source(permanent)
        # Clean granted effects sourced from the leaving permanent
        for pl in [self.player1, self.player2]:
            for p in pl.battle_area:
                p.remove_granted_effects_by_source(permanent)

    def clear_expired_granted_effects(self):
        """Remove expired granted effects from all permanents."""
        for pl in [self.player1, self.player2]:
            for p in pl.battle_area:
                p.clear_expired_effects(self.turn_count)

    def _rule_process(self):
        """DCGO AutoProcessing: run rule-based state checks after every effect resolution.

        Loops until no more rule actions are needed (stable state).
        Checks:
        - Permanents with DP < 0 → raw trash (no destruction event, no prevention)
        - Digimon with DP == 0 → destroy (respects CanBeDestroyed / prevention keywords)
        - Non-Digimon in breeding area → trash
        - Linked cards that no longer meet conditions → unlink and trash

        Note: Deck-out loss is NOT checked here — DCGO only triggers loss when a
        player would draw and cannot (handled in phase_draw / Player.draw).
        """
        if self.game_over:
            return
        max_loops = 20
        for _ in range(max_loops):
            changed = False

            # 1a. DP < 0 → raw trash (DCGO: TrashNoDPProcess)
            #     No destruction event, no prevention keywords. Applies to Digimon
            #     and non-played Options.
            for player in [self.player1, self.player2]:
                for perm in list(player.battle_area):
                    if perm.dp is not None and perm.dp < 0 and (perm.is_digimon or perm.is_option):
                        self.logger.log(f"[Rule Process] {self._perm_ref(perm)} has negative DP — trashed")
                        player.battle_area.remove(perm)
                        self.cleanup_modifiers_for_permanent(perm)
                        if not perm.is_token:
                            player.trash_cards.extend(perm.card_sources)
                        changed = True

            # 1b. DP == 0 → destroy (DCGO: DigimonLackDPProcess)
            #     Proper destruction with prevention keywords (Armor Purge, etc.)
            for player in [self.player1, self.player2]:
                for perm in list(player.battle_area):
                    if perm.is_digimon and perm.dp is not None and perm.dp == 0:
                        self.logger.log(f"[Rule Process] {self._perm_ref(perm)} has 0 DP — destroyed")
                        player.delete_permanent(perm, removal_cause='rule')
                        changed = True

            # 2. Non-Digimon in breeding area → trash (DCGO: TrashNonDigimonProcess)
            for player in [self.player1, self.player2]:
                if player.breeding_area is not None and not player.breeding_area.is_digimon:
                    perm = player.breeding_area
                    self.logger.log(f"[Rule Process] Non-Digimon in breeding area — trashed")
                    player.breeding_area = None
                    player.trash_cards.extend(perm.card_sources)
                    changed = True

            # 3. Linked cards that no longer meet conditions → unlink and trash
            #    (DCGO: DigimonLackLinkCondition + DigimonLackLinkMaxCount)
            for player in [self.player1, self.player2]:
                for perm in list(player.battle_area):
                    if not perm.linked_cards:
                        continue
                    for linked in list(perm.linked_cards):
                        link_condition = getattr(linked, '_link_condition', None)
                        if link_condition and not link_condition(perm):
                            self.logger.log(f"[Rule Process] Linked card condition no longer met — unlinked and trashed")
                            perm.linked_cards.remove(linked)
                            player.trash_cards.append(linked)
                            changed = True

            if not changed:
                break

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

    def _fire_digivolve_observers(self, digivolved_perm):
        """Fire effects on other permanents observing a digivolution event."""
        owner = self._find_owner(digivolved_perm)
        if owner is None:
            return
        for perm in list(owner.battle_area):
            if perm is digivolved_perm:
                continue
            for source in perm.card_sources:
                for effect in source.effect_list(EffectTiming.NoTiming):
                    if not getattr(effect, '_is_digivolve_observer', False):
                        continue
                    if not effect.on_process_callback:
                        continue
                    if not effect.can_activate_this_turn():
                        continue
                    effect.set_effect_source_permanent(perm)
                    context = {
                        'game': self, 'player': owner, 'permanent': perm,
                        'card': effect.effect_source_card,
                        'digivolved_permanent': digivolved_perm,
                        'turn_player': self.turn_player,
                        'opponent_player': self.opponent_player,
                    }
                    if effect.can_use_condition and not effect.can_use_condition(context):
                        continue
                    effect.record_activation()
                    self._invoke_effect_callback(effect, context)

    def _fire_play_observers(self, played_perm, player):
        """Fire effects on other permanents observing a play event."""
        if player is None:
            return
        for perm in list(player.battle_area):
            if perm is played_perm:
                continue
            for source in perm.card_sources:
                for effect in source.effect_list(EffectTiming.NoTiming):
                    if not getattr(effect, '_is_play_observer', False):
                        continue
                    if not effect.on_process_callback:
                        continue
                    if not effect.can_activate_this_turn():
                        continue
                    effect.set_effect_source_permanent(perm)
                    context = {
                        'game': self, 'player': player, 'permanent': perm,
                        'card': effect.effect_source_card,
                        'played_permanent': played_perm,
                        'turn_player': self.turn_player,
                        'opponent_player': self.opponent_player,
                    }
                    if effect.can_use_condition and not effect.can_use_condition(context):
                        continue
                    effect.record_activation()
                    self._invoke_effect_callback(effect, context)

    def _fire_deletion_observers(self, deleted_perm, owner, removal_cause: str = 'effect'):
        """Fire effects on permanents observing a deletion event."""
        if owner is None:
            return
        # Scan owner's battle area
        for perm in list(owner.battle_area):
            if perm is deleted_perm:
                continue
            for source in perm.card_sources:
                for effect in source.effect_list(EffectTiming.NoTiming):
                    if not getattr(effect, '_is_deletion_observer', False):
                        continue
                    if not effect.on_process_callback:
                        continue
                    if not effect.can_activate_this_turn():
                        continue
                    effect.set_effect_source_permanent(perm)
                    context = {
                        'game': self, 'player': owner, 'permanent': perm,
                        'card': effect.effect_source_card,
                        'deleted_permanent': deleted_perm,
                        'removal_cause': removal_cause,
                        'turn_player': self.turn_player,
                        'opponent_player': self.opponent_player,
                    }
                    if effect.can_use_condition and not effect.can_use_condition(context):
                        continue
                    effect.record_activation()
                    self._invoke_effect_callback(effect, context)
        # Also scan opponent's battle area for cross-side watchers
        enemy = owner.enemy if hasattr(owner, 'enemy') and owner.enemy else None
        if enemy:
            for perm in list(enemy.battle_area):
                for source in perm.card_sources:
                    for effect in source.effect_list(EffectTiming.NoTiming):
                        if not getattr(effect, '_is_deletion_observer', False):
                            continue
                        if not effect.on_process_callback:
                            continue
                        if not effect.can_activate_this_turn():
                            continue
                        effect.set_effect_source_permanent(perm)
                        context = {
                            'game': self, 'player': enemy, 'permanent': perm,
                            'card': effect.effect_source_card,
                            'deleted_permanent': deleted_perm,
                            'removal_cause': removal_cause,
                            'turn_player': self.turn_player,
                            'opponent_player': self.opponent_player,
                        }
                        if effect.can_use_condition and not effect.can_use_condition(context):
                            continue
                        effect.record_activation()
                        self._invoke_effect_callback(effect, context)

    def _fire_suspend_observers(self, suspended_perm):
        """Fire effects on permanents observing a suspend event."""
        owner = self._find_owner(suspended_perm)
        if owner is None:
            return
        for perm in list(owner.battle_area):
            if perm is suspended_perm:
                continue
            for source in perm.card_sources:
                for effect in source.effect_list(EffectTiming.NoTiming):
                    if not getattr(effect, '_is_suspend_observer', False):
                        continue
                    if not effect.on_process_callback:
                        continue
                    if not effect.can_activate_this_turn():
                        continue
                    effect.set_effect_source_permanent(perm)
                    context = {
                        'game': self, 'player': owner, 'permanent': perm,
                        'card': effect.effect_source_card,
                        'suspended_permanent': suspended_perm,
                        'turn_player': self.turn_player,
                        'opponent_player': self.opponent_player,
                    }
                    if effect.can_use_condition and not effect.can_use_condition(context):
                        continue
                    effect.record_activation()
                    self._invoke_effect_callback(effect, context)

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
        """Clear temporary DP modifiers, expired keyword grants, and attack counts at start of turn."""
        for player in [self.player1, self.player2]:
            for perm in player.battle_area:
                perm.clear_temp_dp()
                perm.clear_expired_grants(self.turn_count)
                perm.attack_count_this_turn = 0

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
        self.pending_attack = None
        self.pending_selection = None
        self._pending_digixros = None
        self.active_player = None
        if hasattr(self, 'revealed_cards'):
            self.revealed_cards.clear()
        self.logger.log(f"Game Over! Winner: {winner.player_name}")
        self._emit('game_over', player=winner.player_id, winner=winner.player_id)

    def action_play_card(self, card_index: int):
        if self.current_phase != GamePhase.Main:
            self.logger.log(f"[Rejected] action_play_card: not in Main phase (phase={self.current_phase})")
            return
        if card_index < 0 or card_index >= len(self.turn_player.hand_cards):
            self.logger.log(f"[Rejected] action_play_card: index {card_index} out of range (hand size={len(self.turn_player.hand_cards)})")
            return

        card = self.turn_player.hand_cards[card_index]

        # Runtime guard: CANNOT_PLAY_CARD modifier
        if self._is_play_blocked_by_modifier(card):
            self.logger.log(f"[Rejected] action_play_card: {self._card_ref(card)} blocked by CANNOT_PLAY_CARD modifier")
            return

        # DigiXros intercept: enter material selection before cost payment
        if card.has_digixros:
            self._initiate_digixros_play(card_index)
            return

        self._execute_play_card(card)

    def _execute_play_card(self, card: 'CardSource', manual_reduction: int = 0,
                            digixros_count: int = 0):
        """Execute the play-card flow (shared by normal play and DigiXros)."""
        cost = self.calculate_play_cost(self.turn_player, card, commit=True,
                                         manual_reduction=manual_reduction)

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
        ctx = {"played_card": card, "played_permanent": played_perm, "event_player": self.turn_player}
        if digixros_count > 0:
            ctx["digixros_count"] = digixros_count
        self.execute_effects(EffectTiming.OnEnterFieldAnyone, ctx)
        self._fire_play_observers(played_perm, self.turn_player)
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

        # Runtime guard: CANNOT_DIGIVOLVE modifier
        # Pass digivolving_card so conditional restrictions (e.g. "only into X") work
        if self.modifiers.has_modifier(perm, ModifierType.CANNOT_DIGIVOLVE,
                                       {'digivolving_card': card}):
            self.logger.log(f"[Rejected] action_digivolve: {self._perm_ref(perm)} blocked by CANNOT_DIGIVOLVE modifier")
            return

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

        self.turn_player.draw()
        perm.turn_digivolved = self.turn_count
        self.execute_effects(EffectTiming.WhenDigivolving, {"digivolved_permanent": perm})
        self._fire_digivolve_observers(perm)
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
        self.turn_player.draw()
        perm.turn_digivolved = self.turn_count
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
            # Fire OnEnterFieldAnyone with hatch context (DCGO: IsDigiEggHatch)
            hatched_perm = self.turn_player.breeding_area
            self.execute_effects(EffectTiming.OnEnterFieldAnyone, {
                "played_permanent": hatched_perm,
                "played_card": hatched_perm.top_card if hatched_perm else None,
                "event_player": self.turn_player,
                "is_hatch": True,
            })
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
            # Set Main phase BEFORE executing OnMove effects so that any
            # request_selection saves previous_phase=Main (not Breeding).
            # This ensures selections resolve back to Main correctly.
            self.current_phase = GamePhase.Main
            self.execute_effects(EffectTiming.OnMove, {"moved_permanent": self.turn_player.battle_area[-1]})
            # If OnMove effects created a pending selection, defer phase_main()
            # until the selection resolves (picked up by _maybe_complete_move_to_main).
            if self.pending_selection is not None:
                self._move_to_main_deferred = True
                return
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

    # ─── Delegating wrappers to extracted free-function modules ──────

    # Kept as class attributes for backward compatibility with Game._KEYWORD_DISPLAY_MAP
    _KEYWORD_DISPLAY_MAP = game_serialization._KEYWORD_DISPLAY_MAP
    _UI_KEYWORDS = game_serialization._UI_KEYWORDS

    def _get_perm_keywords(self, perm: Permanent) -> List[str]:
        return game_serialization._get_perm_keywords(self, perm)

    def _get_activatable_effects(self, perm: Permanent, slot_idx: int) -> List[Dict[str, Any]]:
        return game_serialization._get_activatable_effects(self, perm, slot_idx)

    def _serialize_perm(self, perm: Permanent, slot_idx: int) -> Dict[str, Any]:
        return game_serialization._serialize_perm(self, perm, slot_idx)

    def to_json(self) -> Dict[str, Any]:
        return game_serialization.to_json(self)

    def to_ui_json(self) -> Dict[str, Any]:
        return game_serialization.to_ui_json(self)

    def get_board_state_tensor(self, player_id: int) -> np.ndarray:
        return game_tensor.build_board_state_tensor(self, player_id)

    def _get_memory_for(self, player: Player) -> int:
        return game_tensor._get_memory_for(self, player)

    @staticmethod
    def _write_field(tensor: np.ndarray, start_idx: int, permanents: List[Permanent], slots: int):
        game_tensor._write_field(tensor, start_idx, permanents, slots)

    @staticmethod
    def _write_card_ids(tensor: np.ndarray, start_idx: int, cards: list, limit: int):
        game_tensor._write_card_ids(tensor, start_idx, cards, limit)

    @staticmethod
    def _write_security_ids(tensor: np.ndarray, start_idx: int, player: Player):
        game_tensor._write_security_ids(tensor, start_idx, player)

    def get_action_mask(self, player_id: int) -> List[float]:
        return game_action_mask.build_action_mask(self, player_id)

    def describe_actions(self, player_id: int) -> Dict[int, str]:
        return game_action_describe.describe_actions(self, player_id)

    def _describe_single_action(self, action_id: int, me: Player, opp: Player) -> Optional[str]:
        return game_action_describe._describe_single_action(self, action_id, me, opp)

    def _describe_selection_action(self, action_id: int, me: Player, opp: Player) -> str:
        return game_action_describe._describe_selection_action(self, action_id, me, opp)
