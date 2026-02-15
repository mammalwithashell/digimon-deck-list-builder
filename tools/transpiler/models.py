"""EffectBlock dataclass — core data structure for extracted card effects."""
from dataclasses import dataclass, field
from typing import List, Optional


@dataclass
class EffectBlock:
    """Represents one extracted effect from a timing block."""
    timing: str = ""
    effect_name: str = ""
    description: str = ""
    is_inherited: bool = False
    is_optional: bool = False
    max_count_per_turn: int = -1
    hash_string: str = ""
    is_factory: bool = False
    factory_method: str = ""
    actions: List[str] = field(default_factory=list)
    conditions: List[str] = field(default_factory=list)
    trait_checks: List[str] = field(default_factory=list)
    name_checks: List[str] = field(default_factory=list)
    color_checks: List[str] = field(default_factory=list)
    dp_change: Optional[int] = None
    draw_count: Optional[int] = None
    memory_gain: Optional[int] = None
    cost_reduction_val: Optional[int] = None
    recovery_count: Optional[int] = None
    raw_block: str = ""
    # Enhanced extraction fields for game helper method calls
    target_dp_limit: Optional[int] = None
    target_dp_min: Optional[int] = None
    target_level_limit: Optional[int] = None
    target_level_min: Optional[int] = None
    reveal_count: Optional[int] = None
    play_from_zone: Optional[str] = None
    play_free: bool = False
    digi_cost_override: Optional[int] = None
    digi_ignore_reqs: bool = False
    factory_dp_value: Optional[int] = None
    factory_sa_value: Optional[int] = None
    # Fix 4: De-digivolve count
    degen_count: Optional[int] = None
    # Fix 6: Trash-as-cost ordering
    is_trash_as_cost: bool = False
    # Fix 7: HasText checks (card text search, not name)
    has_text_checks: List[str] = field(default_factory=list)
    # Fix 1: Factory condition closure fields
    factory_cond_owner_turn: bool = False
    factory_cond_on_battle: bool = False
    factory_cond_digi_count: Optional[int] = None
    factory_cond_has_text: List[str] = field(default_factory=list)
    factory_cond_source_name: List[str] = field(default_factory=list)
    factory_cond_source_trait: List[str] = field(default_factory=list)
    factory_cond_perm_name: List[str] = field(default_factory=list)
    factory_cond_perm_trait: List[str] = field(default_factory=list)
    # Fix 5: Non-self DP (applies to all your Digimon)
    is_dp_all: bool = False
    # Fix 11: Destroy security count
    destroy_security_count: Optional[int] = None
    # P2: Mill action fields
    mill_count: Optional[int] = None
    mill_target: str = "self"  # "self" or "enemy"
    # P4: Descriptive tag (non-implementable effect type)
    descriptive_tag: str = ""
    # Fix 10: CanActivateCondition fields for activate effects
    activate_cond_digi_count: Optional[int] = None
    activate_cond_source_name: List[str] = field(default_factory=list)
    activate_cond_source_trait: List[str] = field(default_factory=list)
    activate_cond_has_text: List[str] = field(default_factory=list)
    activate_cond_perm_name: List[str] = field(default_factory=list)
    # P5: Token play name
    token_name: str = ""
