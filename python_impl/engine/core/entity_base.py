from typing import List
from ..data.enums import CardColor, CardKind, Rarity
from ..data.evo_cost import EvoCost

class CEntity_Base:
    def __init__(self):
        self.card_index: int = 0
        self.card_colors: List[CardColor] = []
        self.play_cost: int = 0
        self.evo_costs: List[EvoCost] = []
        self.level: int = 0
        self.card_name_jpn: str = ""
        self.card_name_eng: str = ""
        self.form_jpn: List[str] = []
        self.form_eng: List[str] = []
        self.attribute_jpn: List[str] = []
        self.attribute_eng: List[str] = []
        self.type_jpn: List[str] = []
        self.type_eng: List[str] = []
        self.card_sprite_name: str = ""
        self.card_kind: CardKind = CardKind.Digimon
        self.effect_description_jpn: str = ""
        self.effect_description_eng: str = ""
        self.inherited_effect_description_jpn: str = ""
        self.inherited_effect_description_eng: str = ""
        self.security_effect_description_jpn: str = ""
        self.security_effect_description_eng: str = ""
        self.card_effect_class_name: str = ""
        self.dp: int = 0
        self.rarity: Rarity = Rarity.C
        self.overflow_memory: int = 0
        self.link_dp: int = 0
        self.link_effect: str = ""
        self.link_requirement: str = ""
        self.card_id: str = ""
        self.max_count_in_deck: int = 4

    @property
    def has_inherited_effect(self) -> bool:
        return bool(self.inherited_effect_description_eng)

    @property
    def has_security_effect(self) -> bool:
        return bool(self.security_effect_description_eng)

    @property
    def is_ace(self) -> bool:
        return False

    @property
    def set_id(self) -> str:
        return ""

    @property
    def is_permanent(self) -> bool:
        return self.card_kind in (CardKind.Digimon, CardKind.Tamer, CardKind.DigiEgg)

    @property
    def has_level(self) -> bool:
        return self.level > 0

    @property
    def has_play_cost(self) -> bool:
        return self.play_cost >= 0

    @property
    def has_use_cost(self) -> bool:
        return self.play_cost >= 0
