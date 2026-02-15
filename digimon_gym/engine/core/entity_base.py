import re
from typing import List, Optional
from ..data.enums import CardColor, CardKind, Rarity
from ..data.evo_cost import EvoCost, DnaCost

class CEntity_Base:
    def __init__(self):
        self.card_index: int = 0
        self.card_colors: List[CardColor] = []
        self.play_cost: int = 0
        self.evo_costs: List[EvoCost] = []
        self.dna_costs: List[DnaCost] = []
        self.level: Optional[int] = None  # None for tamers/options and some Digimon (e.g. Eater Bit)
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
        self.dp: Optional[int] = None  # None for eggs/tamers/options; 0+ for digimon
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
    def card_text(self) -> str:
        """Combined effect text (effect + inherited + security) for HasText checks."""
        parts = []
        if self.effect_description_eng:
            parts.append(self.effect_description_eng)
        if self.inherited_effect_description_eng:
            parts.append(self.inherited_effect_description_eng)
        if self.security_effect_description_eng:
            parts.append(self.security_effect_description_eng)
        return " ".join(parts)

    @property
    def is_ace(self) -> bool:
        """Check if this card is an ACE card (has Ace Overflow text)."""
        return "Ace Overflow" in self.inherited_effect_description_eng

    @property
    def ace_overflow_cost(self) -> int:
        """Extract the overflow memory penalty from ACE cards.

        Parses 'Ace Overflow ＜-N＞' or 'Ace Overflow <-N>' from inherited
        effect text. Returns N (positive int) or 0 if not an ACE card.
        """
        if not self.is_ace:
            return 0
        match = re.search(
            r'Ace Overflow\s*[＜<]\s*-(\d+)\s*[＞>]',
            self.inherited_effect_description_eng,
        )
        return int(match.group(1)) if match else self.overflow_memory

    @property
    def set_id(self) -> str:
        return ""

    @property
    def is_permanent(self) -> bool:
        return self.card_kind in (CardKind.Digimon, CardKind.Tamer, CardKind.DigiEgg)

    @property
    def has_level(self) -> bool:
        return self.level is not None and self.level > 0

    @property
    def has_play_cost(self) -> bool:
        return self.play_cost >= 0

    @property
    def has_use_cost(self) -> bool:
        return self.play_cost >= 0
