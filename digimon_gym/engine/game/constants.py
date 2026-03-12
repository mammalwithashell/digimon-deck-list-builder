"""Constants and dataclasses for the Digimon game engine.

Extracted from game.py to allow sub-modules to import constants without
pulling in the full Game class.
"""
from __future__ import annotations
from typing import TYPE_CHECKING, Optional, Union, List, Dict, Any, Callable
from dataclasses import dataclass, field

if TYPE_CHECKING:
    from ..core.player import Player
    from ..core.permanent import Permanent

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


# ─── Dataclasses ─────────────────────────────────────────────────────

@dataclass
class TriggeredEffect:
    """An effect that has been collected for stack-based resolution.

    Mirrors DCGO's SkillInfo — captures the effect, its owning permanent,
    and the context at the moment it triggered so resolution can proceed
    even if the board state has since changed.
    """
    effect: Any  # ICardEffect
    permanent: Any  # Permanent that owns this effect
    owner: Any  # Player who owns the permanent
    context: Dict[str, Any]  # full context dict for on_process_callback
    is_turn_player: bool = False  # whether owner is the current turn player


@dataclass
class PendingAttack:
    """Context for an attack in progress (paused for block/counter decisions)."""
    attacker: Any  # Permanent
    original_target: Any  # Union[Permanent, Player]
    effective_target: Any  # Union[Permanent, Player] — changes if blocked
    is_blocked: bool = False
    blocker: Optional[Any] = None  # Optional[Permanent]
    without_suspend: bool = False  # Overclock: attack without suspending
    is_vortex: bool = False  # Vortex: end-of-turn Digimon-only attack
    return_phase: Optional[Any] = None  # Phase to return to after resolution (e.g. EndOfTurnAction)
    is_end_attack: bool = False  # DCGO: effects can force-end the attack early


@dataclass
class PendingSelection:
    """Context for an effect waiting for player selection."""
    callback: Callable[[int], None]  # receives the selected index
    selecting_player: Any  # Player
    previous_phase: Any  # GamePhase
    valid_indices: List[int] = field(default_factory=list)
    is_optional: bool = False  # if True, player can decline with action 62
    prompt: str = ""  # human-readable prompt for the UI
    effect_choices: Optional[List[dict]] = None  # for SelectEffectChoice: [{index, cardId, cardName, label}]
    keyword_prompt: Optional[dict] = None  # for keyword triggers: {keyword, cardId, cardName}
    on_decline: Optional[Callable[[], None]] = None  # called when player declines optional selection
