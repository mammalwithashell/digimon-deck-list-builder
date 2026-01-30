from __future__ import annotations
from typing import TYPE_CHECKING, Callable, Optional, Dict, Any
from abc import ABC

if TYPE_CHECKING:
    from ..core.card_source import CardSource
    from ..core.permanent import Permanent

class ICardEffect(ABC):
    def __init__(self):
        self.effect_source_card: Optional['CardSource'] = None
        self.effect_source_permanent: Optional['Permanent'] = None
        self.max_count_per_turn: int = 0
        self.effect_name: str = ""
        self.effect_description: str = ""
        self.hash_string: str = ""
        self.on_process_callback: Optional[Callable[[], None]] = None
        self.root_card_effect: Optional['ICardEffect'] = None
        self.can_use_condition: Optional[Callable[[Dict[str, Any]], bool]] = None
        self.can_activate_condition: Optional[Callable[[Dict[str, Any]], bool]] = None
        self.is_optional: bool = False
        self.use_optional: bool = False
        self.is_declarative: bool = False
        self.is_inherited_effect: bool = False
        self.is_linked_effect: bool = False
        self.is_security_effect: bool = False
        self.is_counter_effect: bool = False
        self.is_digimon_effect: bool = False
        self.is_tamer_effect: bool = False
        self.chain_activations: int = 0
        self.is_background_process: bool = False
        self.is_not_show_ui: bool = False

    @property
    def is_disabled(self) -> bool:
        return False

    @property
    def is_on_play(self) -> bool:
        return False

    @property
    def is_when_digivolving(self) -> bool:
        return False

    @property
    def is_on_deletion(self) -> bool:
        return False

    @property
    def is_on_attack(self) -> bool:
        return False

    def set_up_icard_effect(self, effect_name: str, can_use_condition: Callable[[Dict[str, Any]], bool], card: 'CardSource'):
        self.effect_name = effect_name
        self.can_use_condition = can_use_condition
        self.effect_source_card = card

    def set_effect_source_card(self, effect_source_card: 'CardSource'):
        self.effect_source_card = effect_source_card

    def set_effect_source_permanent(self, effect_source_permanent: 'Permanent'):
        self.effect_source_permanent = effect_source_permanent

    def set_max_count_per_turn(self, max_count_per_turn: int):
        self.max_count_per_turn = max_count_per_turn

    def set_effect_name(self, effect_name: str):
        self.effect_name = effect_name

    def set_effect_description(self, effect_description: str):
        self.effect_description = effect_description

    def set_hash_string(self, hash_string: str):
        self.hash_string = hash_string

    def set_on_process_callback(self, on_process_callback: Callable[[], None]):
        self.on_process_callback = on_process_callback

    def set_root_card_effect(self, root_card_effect: 'ICardEffect'):
        self.root_card_effect = root_card_effect

    def set_can_use_condition(self, can_use_condition: Callable[[Dict[str, Any]], bool]):
        self.can_use_condition = can_use_condition

    def set_can_activate_condition(self, can_activate_condition: Callable[[Dict[str, Any]], bool]):
        self.can_activate_condition = can_activate_condition

    def can_trigger(self, hashtable: Dict[str, Any]) -> bool:
        return True

    def can_activate(self, hashtable: Dict[str, Any]) -> bool:
        return True

    def can_use(self, hashtable: Dict[str, Any]) -> bool:
        return True

    def is_same_effect(self, card_effect: Optional['ICardEffect']) -> bool:
        if card_effect is None:
            return False
        return self.hash_string == card_effect.hash_string and self.root_card_effect == card_effect.root_card_effect

    def get_change_dp_value(self, permanent: 'Permanent') -> int:
        return 0
