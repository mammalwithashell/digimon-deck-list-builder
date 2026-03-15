"""Continuous effect modifier interfaces for the Digimon engine.

Mirrors DCGO's CardEffectInterfaces.cs pattern: effects register typed modifier
callbacks on permanents, and the engine queries them at decision points (deletion,
targeting, cost calculation, DP calculation, etc.).

Each modifier type is a string key mapped to a callable that takes context and
returns a result.  Modifiers can have expiry conditions (end of turn, end of
attack, or permanent until the effect source leaves the field).

Usage in card scripts:
    # In an effect's on_process_callback:
    game.register_modifier(permanent, ModifierType.CANNOT_BE_DESTROYED, {
        'condition': lambda perm, ctx: True,  # always active
        'source_effect': effect,
        'expiry': 'end_of_turn',
    })

Engine queries:
    if game.query_modifier(permanent, ModifierType.CANNOT_BE_DESTROYED):
        # Deletion is prevented
"""
from __future__ import annotations
from enum import Enum, auto
from typing import TYPE_CHECKING, Any, Callable, Dict, List, Optional

if TYPE_CHECKING:
    from ..core.permanent import Permanent
    from ..core.card_source import CardSource
    from .card_effect import ICardEffect


class ModifierType(Enum):
    """Modifier types corresponding to DCGO's CardEffectInterfaces.

    Each entry maps to a DCGO interface (documented in comments).
    """
    # ── Deletion Prevention ─────────────────────────────────────────
    # ICanNotBeDestroyedEffect
    CANNOT_BE_DESTROYED = auto()
    # ICanNotBeDestroyedByBattleEffect
    CANNOT_BE_DESTROYED_BY_BATTLE = auto()
    # ICanNotBeDestroyedBySkillEffect
    CANNOT_BE_DESTROYED_BY_EFFECT = auto()
    # ICanNotBeRemovedEffect
    CANNOT_BE_REMOVED = auto()

    # ── Targeting / Selection Protection ────────────────────────────
    # ICanNotSelectBySkillEffect
    CANNOT_BE_SELECTED_BY_EFFECT = auto()
    # ICanNotAffectedEffect
    CANNOT_BE_AFFECTED = auto()
    # IDisableCardEffect
    DISABLE_EFFECT = auto()

    # ── DP Modification ─────────────────────────────────────────────
    # IChangeDPEffect
    CHANGE_DP = auto()
    # IChangeBaseDPEffect
    CHANGE_BASE_DP = auto()
    # IChangeCardDPEffect
    CHANGE_CARD_DP = auto()
    # IImmuneFromDPMinusEffect
    IMMUNE_FROM_DP_MINUS = auto()
    # IDontHaveDPEffect
    DONT_HAVE_DP = auto()
    # IChangeDPDeleteEffectMaxDPEffect
    CHANGE_DP_DELETE_MAX = auto()
    # DP floor — computed DP cannot go below this value
    DP_FLOOR = auto()

    # ── Cost Modification ───────────────────────────────────────────
    # IChangeCostEffect
    CHANGE_PLAY_COST = auto()
    # Digivolution cost changes (from AddDigivolutionRequirement, etc.)
    CHANGE_DIGIVOLUTION_COST = auto()
    # ICannotReduceCostEffect
    CANNOT_REDUCE_COST = auto()

    # ── Security Attack Modification ────────────────────────────────
    # IChangeSAttackEffect
    CHANGE_SECURITY_ATTACK = auto()
    # IInvertSAttackEffect
    INVERT_SECURITY_ATTACK = auto()

    # ── Suspend / Unsuspend Locks ───────────────────────────────────
    # ICanNotSuspendEffect
    CANNOT_SUSPEND = auto()
    # ICanNotUnsuspendEffect
    CANNOT_UNSUSPEND = auto()

    # ── Movement / Return Prevention ────────────────────────────────
    # ICannotReturnToHandEffect
    CANNOT_RETURN_TO_HAND = auto()
    # ICannotReturnToLibraryEffect
    CANNOT_RETURN_TO_DECK = auto()
    # ICanNotMoveEffect
    CANNOT_MOVE = auto()

    # ── Play / Field Restrictions ───────────────────────────────────
    # ICanNotPlayCardEffect
    CANNOT_PLAY_CARD = auto()
    # ICanNotPlayCardEffect (effect-based plays only — normal hand plays unaffected)
    CANNOT_PLAY_BY_EFFECT = auto()
    # ICanNotPutFieldEffect
    CANNOT_PUT_ON_FIELD = auto()
    # ICanNotDigivolveEffect
    CANNOT_DIGIVOLVE = auto()
    # IIgnoreColorConditionEffect — bypass Option color requirement
    IGNORE_COLOR_REQUIREMENT = auto()

    # ── Attack Restrictions ─────────────────────────────────────────
    # Force a Digimon to attack at start of main phase
    FORCE_ATTACK = auto()
    # ICanNotAttackEffect (general attack prevention)
    CANNOT_ATTACK = auto()
    # ICanNotAttackTargetDefendingPermanentEffect
    CANNOT_ATTACK_TARGET = auto()
    # ICanAttackTargetDefendingPermanentEffect
    CAN_ATTACK_TARGET = auto()
    # ICanAttackTargetDefendingPermanentClass (unsuspended attack)
    CAN_ATTACK_UNSUSPENDED = auto()
    # ICanNotSwitchAttackTargetEffect
    CANNOT_SWITCH_ATTACK_TARGET = auto()
    # ICannotBlockEffect
    CANNOT_BLOCK = auto()

    # ── Attribute Overrides ─────────────────────────────────────────
    # IChangeCardNamesEffect
    CHANGE_CARD_NAMES = auto()
    # IChangeBaseCardNameEffect
    CHANGE_BASE_CARD_NAMES = auto()
    # IChangeCardColorEffect
    CHANGE_CARD_COLORS = auto()
    # IChangeBaseCardColorEffect
    CHANGE_BASE_CARD_COLORS = auto()
    # IChangeTraitsEffect
    CHANGE_TRAITS = auto()
    # IChangePermanentLevelEffect
    CHANGE_PERMANENT_LEVEL = auto()
    # IChangeCardLevelEffect
    CHANGE_CARD_LEVEL = auto()

    # ── Keyword Grants ──────────────────────────────────────────────
    # IBlockerEffect
    GRANT_BLOCKER = auto()
    # IRushEffect
    GRANT_RUSH = auto()
    # IRebootEffect
    GRANT_REBOOT = auto()
    # IAllianceEffect
    GRANT_ALLIANCE = auto()
    # IIcecladEffect
    GRANT_ICECLAD = auto()
    # IScapegoatEffect
    GRANT_SCAPEGOAT = auto()
    # ITreatAsDigimonEffect
    TREAT_AS_DIGIMON = auto()
    # IAddSkillEffect
    ADD_SKILL = auto()

    # ── Memory Restrictions ─────────────────────────────────────────
    # ICannotAddMemoryEffect
    CANNOT_ADD_MEMORY = auto()
    # ICannotAddSecurityEffect
    CANNOT_ADD_SECURITY = auto()
    # IChangeEndTurnMinMemoryEffect
    CHANGE_END_TURN_MIN_MEMORY = auto()

    # ── Digivolution Stack Protection ───────────────────────────────
    # IImmuneFromDeDigivolveEffect
    IMMUNE_FROM_DE_DIGIVOLVE = auto()
    # IImmuneFromStackTrashingEffect
    IMMUNE_FROM_STACK_TRASHING = auto()
    # ICanNotTrashFromDigivolutionCardsEffect
    CANNOT_TRASH_DIGIVOLUTION_CARDS = auto()

    # ── Security Battle ─────────────────────────────────────────────
    # IDontBattleSecurityDigimonEffect
    DONT_BATTLE_SECURITY_DIGIMON = auto()

    # ── Link / DigiXros / Assembly Conditions ───────────────────────
    # IChangeLinkMaxEffect
    CHANGE_LINK_MAX = auto()
    # IVortexCanAttackPlayersEffect
    VORTEX_CAN_ATTACK_PLAYERS = auto()


class ModifierEntry:
    """A single registered modifier on a permanent.

    Attributes:
        modifier_type: The type of modifier (from ModifierType enum).
        condition: Callable(permanent, context) -> bool. The modifier is active
                   only when this returns True. None means always active.
        value_fn: Optional callable for value-producing modifiers (e.g., DP change).
                  Signature depends on modifier type.
        source_effect: The ICardEffect that registered this modifier.
        source_permanent: The permanent that owns the effect (for cleanup).
        expiry: When this modifier expires:
                'permanent' = until source leaves field (default)
                'end_of_turn' = cleared at end of turn
                'end_of_attack' = cleared at end of current attack
                'end_of_opponent_turn' = cleared at end of opponent's next turn
    """
    __slots__ = ('modifier_type', 'condition', 'value_fn', 'source_effect',
                 'source_permanent', 'expiry', 'granting_player')

    def __init__(
        self,
        modifier_type: ModifierType,
        condition: Optional[Callable] = None,
        value_fn: Optional[Callable] = None,
        source_effect: Optional['ICardEffect'] = None,
        source_permanent: Optional['Permanent'] = None,
        expiry: str = 'permanent',
        granting_player=None,
    ):
        self.modifier_type = modifier_type
        self.condition = condition
        self.value_fn = value_fn
        self.source_effect = source_effect
        self.source_permanent = source_permanent
        self.expiry = expiry
        self.granting_player = granting_player

    def is_active(self, target: 'Permanent', context: Optional[Dict[str, Any]] = None) -> bool:
        """Check if this modifier is currently active for the given target."""
        if self.condition is None:
            return True
        try:
            return self.condition(target, context or {})
        except Exception:
            return False


class ModifierRegistry:
    """Central registry for continuous effect modifiers.

    Lives on the Game object. Permanents register modifiers here when their
    continuous effects activate, and the engine queries them at decision points.
    """

    def __init__(self):
        # Map: ModifierType -> list of ModifierEntry
        self._modifiers: Dict[ModifierType, List[ModifierEntry]] = {}

    def register(self, entry: ModifierEntry):
        """Register a new modifier."""
        if entry.modifier_type not in self._modifiers:
            self._modifiers[entry.modifier_type] = []
        self._modifiers[entry.modifier_type].append(entry)

    def unregister(self, entry: ModifierEntry):
        """Remove a specific modifier entry."""
        entries = self._modifiers.get(entry.modifier_type, [])
        if entry in entries:
            entries.remove(entry)

    def unregister_by_source(self, source_permanent: 'Permanent'):
        """Remove all modifiers from a specific source permanent (e.g., when it leaves field)."""
        for mod_type in self._modifiers:
            self._modifiers[mod_type] = [
                e for e in self._modifiers[mod_type]
                if e.source_permanent is not source_permanent
            ]

    def unregister_by_effect(self, source_effect: 'ICardEffect'):
        """Remove all modifiers from a specific effect."""
        for mod_type in self._modifiers:
            self._modifiers[mod_type] = [
                e for e in self._modifiers[mod_type]
                if e.source_effect is not source_effect
            ]

    def clear_expiry(self, expiry: str):
        """Remove all modifiers with a given expiry type."""
        for mod_type in self._modifiers:
            self._modifiers[mod_type] = [
                e for e in self._modifiers[mod_type]
                if e.expiry != expiry
            ]

    def clear_opponent_turn_expiry(self, current_turn_player):
        """Clear 'end_of_opponent_turn' modifiers granted by the current turn player.

        These modifiers last "until your opponent's turn ends", so they expire
        at the start of the granting player's next turn.
        """
        for mod_type in self._modifiers:
            self._modifiers[mod_type] = [
                e for e in self._modifiers[mod_type]
                if not (e.expiry == 'end_of_opponent_turn'
                        and e.granting_player is current_turn_player)
            ]

    def clear_all(self):
        """Remove all modifiers."""
        self._modifiers.clear()

    # ── Boolean Queries (does any active modifier match?) ───────────

    def has_modifier(
        self,
        target: 'Permanent',
        modifier_type: ModifierType,
        context: Optional[Dict[str, Any]] = None,
    ) -> bool:
        """Check if any active modifier of the given type applies to the target."""
        for entry in self._modifiers.get(modifier_type, []):
            if entry.is_active(target, context):
                return True
        return False

    # ── Value Queries (aggregate value from all active modifiers) ───

    def get_int_modifier(
        self,
        target: 'Permanent',
        modifier_type: ModifierType,
        base_value: int = 0,
        context: Optional[Dict[str, Any]] = None,
    ) -> int:
        """Get aggregated int value from all active modifiers of a type.

        For additive modifiers (e.g., DP changes), this sums all active values.
        """
        result = base_value
        for entry in self._modifiers.get(modifier_type, []):
            if entry.is_active(target, context) and entry.value_fn:
                try:
                    result = entry.value_fn(result, target, context or {})
                except Exception:
                    pass
        return result

    def get_list_modifier(
        self,
        target: 'Permanent',
        modifier_type: ModifierType,
        base_list: list,
        context: Optional[Dict[str, Any]] = None,
    ) -> list:
        """Get aggregated list value from all active modifiers (e.g., card names, colors)."""
        result = list(base_list)
        for entry in self._modifiers.get(modifier_type, []):
            if entry.is_active(target, context) and entry.value_fn:
                try:
                    result = entry.value_fn(result, target, context or {})
                except Exception:
                    pass
        return result

    # ── Convenience Methods for Common Queries ──────────────────────

    def can_be_destroyed(
        self,
        target: 'Permanent',
        is_battle: bool = False,
        source_effect: Optional['ICardEffect'] = None,
    ) -> bool:
        """Check if a permanent can be destroyed, considering all active protection modifiers."""
        ctx = {'is_battle': is_battle, 'source_effect': source_effect}

        # Blanket destruction protection
        if self.has_modifier(target, ModifierType.CANNOT_BE_DESTROYED, ctx):
            return False

        # Battle-specific protection
        if is_battle and self.has_modifier(target, ModifierType.CANNOT_BE_DESTROYED_BY_BATTLE, ctx):
            return False

        # Effect-specific protection
        if source_effect and self.has_modifier(target, ModifierType.CANNOT_BE_DESTROYED_BY_EFFECT, ctx):
            return False

        return True

    def can_be_selected_by_effect(
        self,
        target: 'Permanent',
        source_effect: Optional['ICardEffect'] = None,
    ) -> bool:
        """Check if a permanent can be selected/targeted by an effect."""
        ctx = {'source_effect': source_effect}

        if self.has_modifier(target, ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, ctx):
            return False

        if self.has_modifier(target, ModifierType.CANNOT_BE_AFFECTED, ctx):
            return False

        if self.has_modifier(target, ModifierType.GRANT_ICECLAD, ctx):
            return False

        return True

    def effective_dp(
        self,
        target: 'Permanent',
        base_dp: int,
        context: Optional[Dict[str, Any]] = None,
    ) -> int:
        """Calculate effective DP considering all active DP modifiers."""
        ctx = context or {}

        # Check if permanent "doesn't have DP"
        if self.has_modifier(target, ModifierType.DONT_HAVE_DP, ctx):
            return 0

        # Apply base DP changes first
        dp = self.get_int_modifier(target, ModifierType.CHANGE_BASE_DP, base_dp, ctx)

        # Then apply regular DP changes
        dp = self.get_int_modifier(target, ModifierType.CHANGE_DP, dp, ctx)

        return max(0, dp)

    def effective_play_cost(
        self,
        card: 'CardSource',
        base_cost: int,
        target_permanents: Optional[list] = None,
        context: Optional[Dict[str, Any]] = None,
    ) -> int:
        """Calculate effective play cost considering all active cost modifiers.

        Note: This queries modifiers registered globally, not on a specific permanent.
        Cost modifiers typically have conditions based on card properties.
        """
        ctx = context or {}
        ctx['card'] = card
        ctx['target_permanents'] = target_permanents

        # Use a dummy permanent for the query since cost modifiers aren't permanent-specific
        cost = base_cost
        for entry in self._modifiers.get(ModifierType.CHANGE_PLAY_COST, []):
            if entry.value_fn:
                try:
                    # Cost modifier value_fn signature: (current_cost, card, context) -> new_cost
                    cost = entry.value_fn(cost, card, ctx)
                except Exception:
                    pass

        return max(0, cost)
