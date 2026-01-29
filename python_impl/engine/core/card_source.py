from __future__ import annotations
from typing import TYPE_CHECKING, List, Optional
from ..data.enums import CardColor, CardKind, EffectTiming

if TYPE_CHECKING:
    from .entity_base import CEntity_Base
    from .player import Player
    from .permanent import Permanent
    from ..interfaces.card_effect import ICardEffect

class CardSource:
    def __init__(self):
        self.c_entity_base: Optional['CEntity_Base'] = None
        self.owner: Optional['Player'] = None
        self.card_index: int = 0
        # self.c_entity_effect_controller = None # Stub for now
        self.is_flipped: bool = False
        self.base_dp: int = 0
        self.is_token: bool = False
        self.will_be_remove_sources: bool = False
        self.is_being_revealed: bool = False
        self.permanent_just_before_remove_field: Optional['Permanent'] = None

    @property
    def can_play_from_hand_during_main_phase(self) -> bool:
        return True

    @property
    def can_not_play_this_option(self) -> bool:
        return False

    @property
    def match_color_requirement(self) -> bool:
        return True

    @property
    def base_card_colors_from_entity(self) -> List[CardColor]:
        return self.c_entity_base.card_colors if self.c_entity_base else []

    @property
    def base_card_colors(self) -> List[CardColor]:
        return self.base_card_colors_from_entity

    @property
    def card_colors(self) -> List[CardColor]:
        return self.base_card_colors

    @property
    def level(self) -> int:
        return self.c_entity_base.level if self.c_entity_base else 0

    @property
    def card_id(self) -> str:
        return self.c_entity_base.card_id if self.c_entity_base else ""

    @property
    def card_names(self) -> List[str]:
        return [self.c_entity_base.card_name_eng] if self.c_entity_base else []

    @property
    def card_traits(self) -> List[str]:
        return []

    @property
    def is_digimon(self) -> bool:
        return self.c_entity_base.card_kind == CardKind.Digimon if self.c_entity_base else False

    @property
    def is_option(self) -> bool:
        return self.c_entity_base.card_kind == CardKind.Option if self.c_entity_base else False

    @property
    def is_tamer(self) -> bool:
        return self.c_entity_base.card_kind == CardKind.Tamer if self.c_entity_base else False

    @property
    def is_digi_egg(self) -> bool:
        return self.c_entity_base.card_kind == CardKind.DigiEgg if self.c_entity_base else False

    @property
    def is_permanent(self) -> bool:
        return self.is_digimon or self.is_tamer or self.is_digi_egg

    @property
    def get_cost_itself(self) -> int:
        return self.c_entity_base.play_cost if self.c_entity_base else 0

    @property
    def has_play_cost(self) -> bool:
        return self.c_entity_base.has_play_cost if self.c_entity_base else False

    @property
    def has_use_cost(self) -> bool:
        return self.c_entity_base.has_use_cost if self.c_entity_base else False

    def init(self):
        pass

    def set_base_data(self, c_entity_base: 'CEntity_Base', owner: 'Player'):
        self.c_entity_base = c_entity_base
        self.owner = owner
        if self.c_entity_base:
            self.base_dp = self.c_entity_base.dp

    def permanent_of_this_card(self) -> Optional['Permanent']:
        return None

    def effect_list(self, timing: EffectTiming) -> List['ICardEffect']:
        return []

    def paying_cost(self, root: object, target_permanents: List['Permanent'], check_availability: bool = False, ignore_level: bool = False, fixed_cost: int = -1) -> int:
        return self.get_cost_itself
