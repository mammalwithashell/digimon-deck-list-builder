import json
import logging
import os
import re
import importlib
from typing import Dict, Optional, Any, List
from ..core.entity_base import CEntity_Base
from ..core.card_script import CardScript
from ..core.card_source import CardSource
from .enums import CardColor, CardKind, Rarity
from .evo_cost import EvoCost, DnaCost, DnaRequirement, DigiXrosCost, DigiXrosElement

logger = logging.getLogger(__name__)

# Map color name strings to CardColor enum values
_COLOR_NAME_MAP = {
    "red": CardColor.Red,
    "blue": CardColor.Blue,
    "yellow": CardColor.Yellow,
    "green": CardColor.Green,
    "white": CardColor.White,
    "black": CardColor.Black,
    "purple": CardColor.Purple,
}


def parse_xros_req(xros_req: str) -> List[DnaCost]:
    """Parse the xros_req text from DigimonCard.io API into DnaCost objects.

    Supported formats:
      "[DNA Digivolve] Blue Lv.4 + Green Lv.4: Cost 0"
      "[DNA Digivolve] Lv.6 w/[Greymon] in name + Lv.6 w/[Garurumon] in name : Cost 0"
      "[DNA Digivolve] Blue/Purple Lv.6 + Black/Yellow Lv.6: Cost 0"

    Returns a list of DnaCost (one per [DNA Digivolve] entry found).
    Logs warnings for malformed segments that cannot be parsed.
    """
    if not xros_req:
        return []

    results: List[DnaCost] = []

    # Split by lines and find all DNA Digivolve entries
    lines = xros_req.replace('\r\n', '\n').replace('\r', '\n').split('\n')
    full_text = ' '.join(lines)

    # Find all [DNA Digivolve] blocks
    dna_pattern = r'\[DNA Digivolve\]\s*(.+?):\s*Cost\s*(\d+)'
    for match in re.finditer(dna_pattern, full_text):
        req_text = match.group(1).strip()
        memory_cost = int(match.group(2))

        # Split requirements by '+'
        parts = req_text.split('+')
        if len(parts) != 2:
            logger.warning(
                "DNA Digivolve requirement expected 2 parts separated by '+', "
                "got %d: %r", len(parts), req_text,
            )
            continue

        req1 = _parse_dna_requirement(parts[0].strip())
        req2 = _parse_dna_requirement(parts[1].strip())
        if not req1:
            logger.warning(
                "Failed to parse DNA requirement 1: %r", parts[0].strip(),
            )
        if not req2:
            logger.warning(
                "Failed to parse DNA requirement 2: %r", parts[1].strip(),
            )
        if req1 and req2:
            results.append(DnaCost(
                requirement1=req1,
                requirement2=req2,
                memory_cost=memory_cost,
            ))

    return results


def _parse_dna_requirement(text: str) -> Optional[DnaRequirement]:
    """Parse a single DNA requirement half like 'Blue Lv.4' or 'Lv.6 w/[Greymon] in name'.

    Supported patterns:
      "Blue Lv.4"
      "Blue/Purple Lv.6"
      "Lv.6 w/[Greymon] in name"   → stored as name_contains
      "Lv.6 w/[Greymon] in text"   → stored as text_contains
      "[Plesiomon]"                 → bare name, any level (level=0)
    """
    # Check for bare [Name] pattern (e.g., "[Plesiomon]", "[Piedmon]")
    # This means "any Digimon named <Name>" without level restriction.
    bare_name_match = re.fullmatch(r'\[([^\]]+)\]', text.strip())
    if bare_name_match:
        return DnaRequirement(
            level=0,
            card_color=None,
            name_contains=bare_name_match.group(1).strip(),
            text_contains="",
        )

    # Extract constraint: w/[Name] in name OR w/[Name] in text
    name_contains = ""
    text_contains = ""
    constraint_match = re.search(r'w/\[([^\]]+)\]\s+in\s+(name|text)', text)
    if constraint_match:
        value = constraint_match.group(1).strip()
        kind = constraint_match.group(2)
        if kind == "name":
            name_contains = value
        else:
            text_contains = value

    # Extract level: Lv.N or Lv N
    level = 0
    level_match = re.search(r'Lv\.?\s*(\d+)', text)
    if level_match:
        level = int(level_match.group(1))

    if level == 0:
        logger.warning("No valid level found in DNA requirement: %r", text)
        return None

    # Extract color(s): look for color names before 'Lv'
    card_color = None
    color_text = text.split('Lv')[0].strip() if 'Lv' in text else ""
    if color_text:
        # Handle multi-color like "Blue/Purple"
        color_names = [c.strip().lower() for c in color_text.split('/')]
        for cn in color_names:
            if cn in _COLOR_NAME_MAP:
                card_color = _COLOR_NAME_MAP[cn]
                break  # Use the first color for matching

    return DnaRequirement(
        level=level,
        card_color=card_color,
        name_contains=name_contains,
        text_contains=text_contains,
    )


def parse_digixros_req(xros_req: str) -> List['DigiXrosCost']:
    """Parse DigiXros/Assembly requirements from xros_req text.

    Returns a list of DigiXrosCost (usually one).
    """
    if not xros_req:
        return []

    results: List[DigiXrosCost] = []
    lines = xros_req.replace('\r\n', '\n').replace('\r', '\n').split('\n')
    full_text = ' '.join(lines)

    # Match [DigiXros -N] or [Assembly -N] blocks
    pattern = r'\[(DigiXros|Assembly)\s*\xa0?(-\d+)\]\s*(.*?)(?=When this|$)'
    for match in re.finditer(pattern, full_text, re.DOTALL):
        mechanic = match.group(1)  # DigiXros or Assembly
        reduce_per = abs(int(match.group(2)))
        body = match.group(3).strip()

        is_assembly = mechanic == 'Assembly'
        source_zones = ['trash'] if is_assembly else ['hand', 'field']

        # Parse constraints: different card numbers / different names
        different_card_numbers = 'different card numbers' in body
        different_names = 'different names' in body

        # Parse has_text constraint (e.g., "with ＜Save＞ in text")
        has_text = ""
        text_match = re.search(r'with\s+[＜<]([^＞>]+)[＞>]\s+in text', body)
        if text_match:
            has_text = text_match.group(1)

        elements = _parse_digixros_elements(body)

        # Calculate max_materials from elements
        max_materials = sum(e.count for e in elements)
        # Infinity marker: count of 99 means unlimited
        if any(e.count >= 99 for e in elements):
            max_materials = -1

        results.append(DigiXrosCost(
            elements=elements,
            reduce_cost_per_card=reduce_per,
            max_materials=max_materials,
            different_card_numbers=different_card_numbers,
            different_names=different_names,
            has_text=has_text,
            source_zones=source_zones,
        ))

    return results


def _parse_digixros_elements(body: str) -> List['DigiXrosElement']:
    """Parse the element specification from a DigiXros body string."""
    # Check multi-trait pattern BEFORE stripping constraints (it contains "different names")
    multi_trait_match = re.match(
        r'(\d+)\s+Digimon cards?\s+w/different names and\s+(.*?)\s+in one of their traits',
        body)
    if multi_trait_match:
        count = int(multi_trait_match.group(1))
        trait_text = multi_trait_match.group(2)
        traits = re.findall(r'\[([^\]]+)\]', trait_text)
        return [DigiXrosElement(
            trait_match=traits[0] if traits else '',
            trait_alternatives=traits[1:] if len(traits) > 1 else [],
            count=count,
        )]

    # Strip trailing constraint clauses for simpler parsing
    body = re.sub(r'\s*(?:w/different\s+(?:card numbers|names)|& different\s+(?:card numbers|names)).*$', '', body)
    body = body.strip()

    if not body:
        return []

    # Pattern: ∞ Digimon cards w/[Trait1] or [Trait2] trait
    inf_match = re.match(
        r'[∞]\s+Digimon cards?\s+w/\[([^\]]+)\](?:\s+or\s+\[([^\]]+)\])?\s*trait',
        body)
    if inf_match:
        traits = [inf_match.group(1)]
        if inf_match.group(2):
            traits.append(inf_match.group(2))
        return [DigiXrosElement(
            trait_match=traits[0],
            trait_alternatives=traits[1:],
            count=99,  # unlimited
        )]

    # Pattern: N level L [Trait] trait Digimon cards (Assembly EX9-074)
    level_trait_match = re.match(
        r'(\d+)\s+level\s+(\d+)\s+\[([^\]]+)\]\s*trait\s+Digimon cards?',
        body)
    if level_trait_match:
        return [DigiXrosElement(
            trait_match=level_trait_match.group(3),
            level_max=int(level_trait_match.group(2)),
            count=int(level_trait_match.group(1)),
        )]

    # Pattern: N Lv.X or lower [Trait1] or [Trait2] trait Digimon cards
    lv_trait_match = re.match(
        r'(\d+)\s+Lv\.(\d+)\s+or\s+lower\s+\[([^\]]+)\](?:\s+or\s+\[([^\]]+)\])?\s*trait\s+Digimon cards?',
        body)
    if lv_trait_match:
        traits = [lv_trait_match.group(3)]
        if lv_trait_match.group(4):
            traits.append(lv_trait_match.group(4))
        return [DigiXrosElement(
            trait_match=traits[0],
            trait_alternatives=traits[1:],
            level_max=int(lv_trait_match.group(2)),
            count=int(lv_trait_match.group(1)),
        )]

    # Pattern: N Digimon card(s) w/[Trait] trait
    trait_count_match = re.match(
        r'(\d+)\s+Digimon cards?\s+w/\[([^\]]+)\]\s*\xa0?trait',
        body)
    if trait_count_match:
        return [DigiXrosElement(
            trait_match=trait_count_match.group(2),
            count=int(trait_count_match.group(1)),
        )]

    # Pattern: 1 Digimon card with ＜X＞ in text
    text_match = re.match(
        r'(\d+)\s+Digimon cards?\s+with\s+[＜<]([^＞>]+)[＞>]\s+in text',
        body)
    if text_match:
        return [DigiXrosElement(count=int(text_match.group(1)))]

    # Pattern: N [Name] (e.g., "4 [Vemmon]", "4 [Negamon]")
    count_name_match = re.match(r'(\d+)\s+\[([^\]]+)\]\s*$', body)
    if count_name_match:
        return [DigiXrosElement(
            name_contains=count_name_match.group(2),
            count=int(count_name_match.group(1)),
            is_digimon_only=False,
        )]

    # Pattern: named elements separated by 'x', possibly with 'or' alternatives and color prefixes
    # e.g., "[Shoutmon] x [Ballistamon] x [Dorulumon]"
    # e.g., "Blue [Greymon] x [MailBirdramon]"
    # e.g., "[Agumon] or [Greymon] x [Gabumon] or [Garurumon]"
    # e.g., "[MadLeomon] x 1 Digimon card w/[Bagra Army] trait"
    # e.g., "[Snatchmon] x 4 [Vemmon]"
    # e.g., "[Sanzomon] or [Sagomon] or [Cho-Hakkaimon]"
    elements = []
    # Split by ' x ' first
    parts = re.split(r'\s+x\s+', body)
    for part in parts:
        part = part.strip()
        if not part:
            continue

        # Sub-pattern: N Digimon card(s) w/[Trait] trait
        sub_trait = re.match(r'(\d+)\s+Digimon cards?\s+w/\[([^\]]+)\]\s*\xa0?trait', part)
        if sub_trait:
            elements.append(DigiXrosElement(
                trait_match=sub_trait.group(2),
                count=int(sub_trait.group(1)),
            ))
            continue

        # Sub-pattern: N [Name] (count + name)
        sub_count_name = re.match(r'(\d+)\s+\[([^\]]+)\]', part)
        if sub_count_name:
            elements.append(DigiXrosElement(
                name_contains=sub_count_name.group(2),
                count=int(sub_count_name.group(1)),
                is_digimon_only=False,
            ))
            continue

        # Sub-pattern: [Name1] or [Name2] or ... (OR-group treated as one element with alternatives)
        or_names = re.findall(r'\[([^\]]+)\]', part)
        if or_names and ' or ' in part:
            # Check for optional color prefix
            color = None
            color_match = re.match(r'(Red|Blue|Yellow|Green|White|Black|Purple)\s+', part)
            if color_match:
                color = _COLOR_NAME_MAP.get(color_match.group(1).lower())
            elements.append(DigiXrosElement(
                name_contains=or_names[0],
                trait_alternatives=[n for n in or_names[1:]],  # Store alt names in trait_alternatives for OR-name matching
                count=1,
                is_digimon_only=False,
                color=color,
            ))
            continue

        # Sub-pattern: [Color] [Name] or just [Name]
        color = None
        color_match = re.match(r'(Red|Blue|Yellow|Green|White|Black|Purple)\s+', part)
        if color_match:
            color = _COLOR_NAME_MAP.get(color_match.group(1).lower())
            part = part[color_match.end():]

        name_match = re.match(r'\[([^\]]+)\]', part)
        if name_match:
            elements.append(DigiXrosElement(
                name_contains=name_match.group(1),
                count=1,
                is_digimon_only=False,
                color=color,
            ))
            continue

        logger.warning("Unparsed DigiXros element: %r", part)

    return elements


class CardDatabase:
    _instance = None

    def __new__(cls):
        if cls._instance is None:
            cls._instance = super(CardDatabase, cls).__new__(cls)
            cls._instance.initialized = False
        return cls._instance

    def __init__(self):
        if self.initialized:
            return
        self.cards: Dict[str, CEntity_Base] = {}
        self.scripts: Dict[str, CardScript] = {}
        self.load_cards()
        self.initialized = True

    def load_cards(self):
        module_dir = os.path.dirname(__file__)
        json_path = os.path.join(module_dir, 'cards.json')

        if not os.path.exists(json_path):
            print(f"Warning: {json_path} not found.")
            return

        with open(json_path, 'r', encoding='utf-8') as f:
            data = json.load(f)

        entries = data.values() if isinstance(data, dict) else data
        for entry in entries:
            entity = CEntity_Base()
            entity.card_id = entry.get('card_id', '')
            entity.card_index = entry.get('card_index', 0)
            entity.card_name_eng = entry.get('card_name_eng', '')
            entity.card_name_jpn = entry.get('card_name_jpn', '')
            entity.type_eng = entry.get('type_eng', [])
            entity.type_jpn = entry.get('type_jpn', [])
            entity.card_effect_class_name = entry.get('card_effect_class_name', '')
            entity.play_cost = entry.get('play_cost', 0)
            entity.dp = entry.get('dp')  # None for eggs/tamers/options
            entity.level = entry.get('level')  # None for tamers/options without level
            entity.max_count_in_deck = entry.get('max_count_in_deck', 4)
            entity.card_sprite_name = entry.get('card_sprite_name', '')
            entity.effect_description_eng = entry.get('effect_description_eng', '')
            entity.inherited_effect_description_eng = entry.get('inherited_effect_description_eng', '')
            entity.security_effect_description_eng = entry.get('security_effect_description_eng', '')

            # Evo costs
            for ec in entry.get('evo_costs', []):
                entity.evo_costs.append(EvoCost(
                    card_color=CardColor(ec.get('card_color', 0)),
                    level=ec.get('level', 0),
                    memory_cost=ec.get('memory_cost', 0),
                ))

            # DNA costs (structured format)
            for dc in entry.get('dna_costs', []):
                req1_data = dc.get('requirement1', {})
                req2_data = dc.get('requirement2', {})
                req1 = DnaRequirement(
                    level=req1_data.get('level', 0),
                    card_color=CardColor(req1_data['card_color']) if 'card_color' in req1_data and req1_data['card_color'] is not None else None,
                    name_contains=req1_data.get('name_contains', ''),
                )
                req2 = DnaRequirement(
                    level=req2_data.get('level', 0),
                    card_color=CardColor(req2_data['card_color']) if 'card_color' in req2_data and req2_data['card_color'] is not None else None,
                    name_contains=req2_data.get('name_contains', ''),
                )
                entity.dna_costs.append(DnaCost(
                    requirement1=req1,
                    requirement2=req2,
                    memory_cost=dc.get('memory_cost', 0),
                ))

            # DNA costs from xros_req text (API format)
            xros_req = entry.get('xros_req', '')
            if xros_req and not entity.dna_costs:
                entity.dna_costs = parse_xros_req(xros_req)

            # DigiXros/Assembly costs from xros_req text
            if xros_req:
                digixros = parse_digixros_req(xros_req)
                if digixros:
                    entity.digixros_costs = digixros

            # Form and attribute
            entity.form_eng = entry.get('form_eng', [])
            entity.attribute_eng = entry.get('attribute_eng', [])

            # Enums
            entity.card_kind = CardKind(entry.get('card_kind', 0))
            entity.rarity = Rarity(entry.get('rarity', 0))
            entity.card_colors = [CardColor(c) for c in entry.get('card_colors', [])]

            # Load Script
            if entity.card_effect_class_name:
                self._load_script(entity)

            self.cards[entity.card_id] = entity

        print(f"Loaded {len(self.cards)} cards.")

    def _load_script(self, entity: CEntity_Base):
        script_name = entity.card_effect_class_name
        # Assume set_id is the part before the first underscore or hyphen
        parts = script_name.split('_')
        if len(parts) < 2:
             parts = script_name.split('-')

        if len(parts) >= 1:
             set_id = parts[0].lower()
             module_name = script_name.lower()

             prefix = "digimon_gym.engine.data.scripts"
             module_path = f"{prefix}.{set_id}.{module_name}"
             try:
                 module = importlib.import_module(module_path)
                 script_class = getattr(module, script_name)
                 self.scripts[entity.card_id] = script_class()
             except (ImportError, AttributeError, SyntaxError):
                 # Try without set folder
                 try:
                     module_path = f"{prefix}.{module_name}"
                     module = importlib.import_module(module_path)
                     script_class = getattr(module, script_name)
                     self.scripts[entity.card_id] = script_class()
                 except (ImportError, AttributeError, SyntaxError) as exc:
                     if isinstance(exc, SyntaxError):
                         logger.warning("SyntaxError loading script for %s: %s", entity.card_id, exc)
                     # No script found for this card (vanilla cards with no effects)

    def get_card(self, card_id: str) -> Optional[CEntity_Base]:
        return self.cards.get(card_id)

    def get_script(self, card_id: str) -> Optional[CardScript]:
        return self.scripts.get(card_id)

    def get_all_cards(self) -> Dict[str, CEntity_Base]:
        return self.cards

    def create_card_source(self, card_id: str, owner=None) -> Optional[CardSource]:
        entity = self.get_card(card_id)
        if not entity:
            print(f"Card ID {card_id} not found.")
            return None

        card_source = CardSource()
        card_source.set_base_data(entity, owner)
        return card_source

if __name__ == "__main__":
    db = CardDatabase()
    print("Database contents:", list(db.get_all_cards().keys()))
