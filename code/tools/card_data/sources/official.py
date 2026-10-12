"""Official Bandai card catalog discovery and immutable source observations.

The official global card list is an HTML catalog rather than a stable JSON API.
This module keeps acquisition small and explicit: catalog and card-list parsing
are pure functions, callers inject a byte fetcher, and every committed card
printing is represented by a content-addressed :class:`SourceObservation`.

No function in this module performs network I/O at import time.  Tests and
offline reproduction use checked-in representative HTML fixtures.
"""

from __future__ import annotations

import html as html_lib
import re
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import PurePosixPath
from typing import Callable, Iterable, Mapping
from urllib.parse import parse_qs, urljoin, urlparse

from ..errors import CompletenessError, HttpError, NetworkError, SourceSchemaError
from ..models import (
    SCHEMA_VERSION,
    ScopeManifest,
    ScopeObservationRef,
    SourceName,
    SourceObservation,
)
from ..serialization import content_digest, sha256_hex


OFFICIAL_CATALOG_URL = "https://world.digimoncard.com/cards/"
OFFICIAL_PARSER_VERSION = "official-html-v5"
OFFICIAL_SOURCE_SCHEMA_VERSION = "world-card-list-html-v1"
USER_AGENT = "digimon-deck-list-builder/card-data-v1"

_CARD_BLOCK_START = re.compile(
    r"<li\b[^>]*class=[\"'][^\"']*\bimage_lists_item\b[^\"']*[\"'][^>]*>",
    re.IGNORECASE,
)
_CARD_ID = re.compile(r"^[A-Z]+\d*-\d+[A-Z]*$")
_TOKEN_CARD_ID = re.compile(r"^(?:[A-Z]+\d*-)?TOKEN\d*$")
_SET_PREFIX = re.compile(r"^(?P<letters>[A-Z]+)-?0*(?P<number>\d+)$")
_LETTER_ONLY_PREFIX = re.compile(r"^[A-Z]+$")
_BRACKETED_PRODUCT_CODE = re.compile(r"\[([^\]]+)\]")
_PRODUCT_CODE = re.compile(
    r"^(?P<letters>[A-Z]+)-?0*(?P<start>\d+)(?:-(?P<end>\d+))?$"
)
_RULE_TRAIT = re.compile(
    r"(?<![\w\[(])(?:\[\s*Rule\s*\]|\(\s*Rule\s*\)|Rule)\s*:?\s*Trait\s*:\s*Has\s*\[([^\]]+)\]\s*Type\b",
    re.IGNORECASE,
)
_RULE_ATTRIBUTE = re.compile(
    r"(?<![\w\[(])(?:\[\s*Rule\s*\]|\(\s*Rule\s*\)|Rule)\s*:?\s*Trait\s*:\s*Has\s*\[([^\]]+)\]\s*attribute\b",
    re.IGNORECASE,
)


_RULE_FORM = re.compile(
    r"^\s*(?:\[\s*Rule\s*\]|\(\s*Rule\s*\)|Rule)\s*:?\s*Trait\s*:\s*Has\s*\[([^\]]+)\]\s*Form\b",
    re.IGNORECASE | re.MULTILINE,
)


FetchBytes = Callable[[str], bytes]


@dataclass(frozen=True, slots=True)
class OfficialCatalogEntry:
    """One official product/category that can be queried for card printings."""

    category_id: str
    label: str
    source_url: str
    release_prefixes: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class OfficialPrinting:
    """One source-visible printing of a gameplay card ID."""

    card_id: str
    printing_id: str
    category_id: str
    source_url: str
    image_url: str
    alternate_art: bool
    extracted: Mapping[str, object]


@dataclass(frozen=True, slots=True)
class OfficialFetchedPage:
    """A fetched product page and all printing observations parsed from it."""

    category: OfficialCatalogEntry
    source_url: str
    raw_payload: bytes
    printings: tuple[OfficialPrinting, ...]


@dataclass(frozen=True, slots=True)
class OfficialDiscoveryResult:
    """Complete discovered membership for a catalog or release-set scope."""

    scope: str
    gameplay_card_ids: tuple[str, ...]
    pages: tuple[OfficialFetchedPage, ...]
    complete: bool

    @property
    def printings(self) -> tuple[OfficialPrinting, ...]:
        return tuple(printing for page in self.pages for printing in page.printings)

    @property
    def source_locations(self) -> tuple[str, ...]:
        return tuple(dict.fromkeys(page.source_url for page in self.pages))


@dataclass(frozen=True, slots=True)
class OfficialScopeSnapshot:
    manifest: ScopeManifest
    observations: tuple[SourceObservation, ...]


def _decode_payload(payload: bytes | str, *, source_url: str) -> str:
    if isinstance(payload, str):
        return payload
    try:
        return payload.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise SourceSchemaError(
            "official payload is not valid UTF-8",
            source="official",
            details={"source_url": source_url, "offset": exc.start},
        ) from exc


def _clean(fragment: str, *, preserve_breaks: bool = False) -> str:
    value = re.sub(r"<br\s*/?>", "\n" if preserve_breaks else " ", fragment, flags=re.I)
    value = re.sub(r"<[^>]+>", " ", value)
    value = html_lib.unescape(value).replace("\xa0", " ")
    if preserve_breaks:
        lines = [re.sub(r"\s+", " ", line).strip() for line in value.splitlines()]
        return "\n".join(line for line in lines if line)
    return re.sub(r"\s+", " ", value).strip()


def _class_fragment(fragment: str, class_name: str, *, tag: str | None = None) -> str | None:
    tag_expr = re.escape(tag) if tag else r"[a-zA-Z0-9]+"
    pattern = re.compile(
        rf"<(?P<tag>{tag_expr})\b[^>]*class=[\"'][^\"']*\b{re.escape(class_name)}\b[^\"']*[\"'][^>]*>(?P<body>.*?)</(?P=tag)>",
        re.I | re.S,
    )
    match = pattern.search(fragment)
    return match.group("body") if match else None


def _all_class_fragments(fragment: str, class_name: str, *, tag: str | None = None) -> list[str]:
    tag_expr = re.escape(tag) if tag else r"[a-zA-Z0-9]+"
    pattern = re.compile(
        rf"<(?P<tag>{tag_expr})\b[^>]*class=[\"'][^\"']*\b{re.escape(class_name)}\b[^\"']*[\"'][^>]*>(?P<body>.*?)</(?P=tag)>",
        re.I | re.S,
    )
    return [match.group("body") for match in pattern.finditer(fragment)]


def _paired_dl(fragment: str, title_class: str) -> list[tuple[str, str]]:
    pattern = re.compile(
        rf"<dt\b[^>]*class=[\"'][^\"']*\b{re.escape(title_class)}\b[^\"']*[\"'][^>]*>(?P<title>.*?)</dt>\s*"
        r"<dd\b[^>]*>(?P<value>.*?)</dd>",
        re.I | re.S,
    )
    return [
        (_clean(match.group("title")), _clean(match.group("value"), preserve_breaks=True))
        for match in pattern.finditer(fragment)
    ]


def _parse_int(value: str) -> int | None:
    match = re.search(r"\d+", value)
    return int(match.group()) if match else None


def _official_card_type(value: str) -> str:
    cleaned = _clean(value)
    # A historical official catalog row truncates "Digi-Egg" to "Digi-Eg".
    # Normalize the source spelling, not a card ID, and preserve Dual suffixes.
    if cleaned.lower() in {"digi-eg", "digi-egg"}:
        return "Digi-Egg"
    return cleaned


def normalize_release_prefix(value: str) -> str:
    """Normalize ``EX-03``/``EX03``/``EX3`` to the gameplay prefix ``EX3``."""

    raw = value.strip().upper()
    if _LETTER_ONLY_PREFIX.fullmatch(raw):
        return raw
    match = _SET_PREFIX.fullmatch(raw)
    if not match:
        raise ValueError(f"invalid release-set prefix: {value!r}")
    return f"{match.group('letters')}{int(match.group('number'))}"


def gameplay_set_prefix(card_id: str) -> str:
    value = card_id.strip().upper()
    if not _CARD_ID.fullmatch(value):
        raise SourceSchemaError(
            f"invalid official gameplay card ID: {card_id!r}",
            source="official",
            card_id=card_id,
        )
    return normalize_release_prefix(value.rsplit("-", 1)[0])


def _release_prefixes(label: str) -> tuple[str, ...]:
    prefixes: list[str] = []
    for raw_code in _BRACKETED_PRODUCT_CODE.findall(label.upper()):
        match = _PRODUCT_CODE.fullmatch(raw_code.strip())
        if not match:
            continue
        letters = match.group("letters")
        start = int(match.group("start"))
        end = int(match.group("end") or start)
        if end < start or end - start > 20:
            continue
        prefixes.extend(f"{letters}{number}" for number in range(start, end + 1))
    if "P-NUMBERED" in label.upper():
        prefixes.append("P")
    return tuple(dict.fromkeys(prefixes))


def parse_catalog(
    payload: bytes | str,
    *,
    source_url: str = OFFICIAL_CATALOG_URL,
) -> tuple[OfficialCatalogEntry, ...]:
    """Parse the official product selector into deterministic category entries."""

    document = _decode_payload(payload, source_url=source_url)
    select = re.search(
        r"<select\b[^>]*name=[\"']category[\"'][^>]*>(?P<body>.*?)</select>",
        document,
        re.I | re.S,
    )
    if not select:
        raise SourceSchemaError(
            "official catalog is missing the category selector",
            source="official",
            details={"source_url": source_url},
        )
    entries: list[OfficialCatalogEntry] = []
    option_pattern = re.compile(
        r"<option\b[^>]*value=[\"'](?P<value>[^\"']*)[\"'][^>]*>(?P<label>.*?)</option>",
        re.I | re.S,
    )
    for match in option_pattern.finditer(select.group("body")):
        category_id = match.group("value").strip()
        if not category_id:
            continue
        label = _clean(match.group("label"))
        if not label:
            raise SourceSchemaError(
                "official catalog category has no label",
                source="official",
                details={"source_url": source_url, "category_id": category_id},
            )
        category_url = urljoin(source_url, f"?search=true&category={category_id}")
        entries.append(
            OfficialCatalogEntry(
                category_id=category_id,
                label=label,
                source_url=category_url,
                release_prefixes=_release_prefixes(label),
            )
        )
    if not entries:
        raise SourceSchemaError(
            "official catalog category selector is empty",
            source="official",
            details={"source_url": source_url},
        )
    category_ids = [entry.category_id for entry in entries]
    if len(category_ids) != len(set(category_ids)):
        raise SourceSchemaError(
            "official catalog contains duplicate category IDs",
            source="official",
            details={"source_url": source_url},
        )
    return tuple(entries)


def _card_blocks(document: str) -> list[str]:
    matches = list(_CARD_BLOCK_START.finditer(document))
    if not matches:
        return []
    blocks: list[str] = []
    for index, match in enumerate(matches):
        end = matches[index + 1].start() if index + 1 < len(matches) else len(document)
        blocks.append(document[match.start() : end])
    return blocks


def _printing_id(image_url: str, card_id: str) -> str:
    path = PurePosixPath(urlparse(image_url).path)
    stem = path.stem.upper()
    return stem or card_id


def _colors(fragment: str) -> list[str]:
    colors = re.findall(r"cardColor_([a-zA-Z]+)", fragment)
    expanded: list[str] = []
    for color in colors:
        if color.lower() == "all":
            expanded.extend(
                ("Red", "Blue", "Yellow", "Green", "White", "Black", "Purple")
            )
        elif color:
            expanded.append(color.capitalize())
    return list(dict.fromkeys(expanded))


def _standard_evolution_routes(primary: str) -> list[dict[str, object]]:
    routes: list[dict[str, object]] = []
    for label, value in _paired_dl(primary, "cardInfoTit"):
        match = re.fullmatch(r"Digivolve Cost\s+(\d+)", label, re.I)
        if not match:
            continue
        cost_level = re.search(r"(\d+)\s*from\s*Lv\.?\s*(\d+)", value, re.I)
        if not cost_level:
            if re.fullmatch(
                r"Multicolor\s+\d+\s+from\s+(?:(?:[^\x00-\x7f]\s*)?(?:Stnd|Sup|Ult|God)\.?|神)",
                value,
                re.I,
            ) or re.fullmatch(r"[A-Za-z /-]+\s+\d+\s+from\s+TAMER", value, re.I):
                # Appmon grade routes are retained structurally in the
                # separate grade_evolution_routes field below. Hybrid Tamer
                # routes are retained in tamer_evolution_routes. Neither is a
                # legacy level route and neither may be inferred as one.
                continue
            raise SourceSchemaError(
                f"unsupported official evolution-cost layout: {value!r}",
                source="official",
            )
        # Locate the original dd so color classes are not lost by text cleanup.
        raw_pair = re.search(
            rf"<dt\b[^>]*>\s*Digivolve Cost\s+{re.escape(match.group(1))}\s*</dt>\s*<dd\b[^>]*>(?P<body>.*?)</dd>",
            primary,
            re.I | re.S,
        )
        colors = _colors(raw_pair.group("body") if raw_pair else "")
        if not colors:
            raise SourceSchemaError(
                "official evolution-cost route has no color",
                source="official",
                details={"label": label, "value": value},
            )
        routes.append(
            {
                "slot": int(match.group(1)),
                "colors": colors,
                "cost": int(cost_level.group(1)),
                "from_level": int(cost_level.group(2)),
            }
        )
    return routes


def _grade_evolution_routes(primary: str) -> list[dict[str, object]]:
    grades = {
        "stnd": "Standard",
        "sup": "Super",
        "ult": "Ultimate",
        "god": "God",
    }
    routes: list[dict[str, object]] = []
    for label, value in _paired_dl(primary, "cardInfoTit"):
        label_match = re.fullmatch(r"Digivolve Cost\s+(\d+)", label, re.I)
        match = re.fullmatch(
            r"Multicolor\s+(\d+)\s+from\s+(?:(?:[^\x00-\x7f]\s*)?(Stnd|Sup|Ult|God)\.?|(神))",
            value,
            re.I,
        )
        if label_match is None or match is None:
            continue
        routes.append(
            {
                "slot": int(label_match.group(1)),
                "colors": ["Multicolor"],
                "cost": int(match.group(1)),
                "from_grade": grades[match.group(2).lower()] if match.group(2) else "God",
            }
        )
    return routes


def _tamer_evolution_routes(primary: str) -> list[dict[str, object]]:
    routes: list[dict[str, object]] = []
    for label, value in _paired_dl(primary, "cardInfoTit"):
        label_match = re.fullmatch(r"Digivolve Cost\s+(\d+)", label, re.I)
        match = re.fullmatch(r"[A-Za-z /-]+\s+(\d+)\s+from\s+TAMER", value, re.I)
        if label_match is None or match is None:
            continue
        raw_pair = re.search(
            rf"<dt\b[^>]*>\s*Digivolve Cost\s+{re.escape(label_match.group(1))}\s*</dt>\s*<dd\b[^>]*>(?P<body>.*?)</dd>",
            primary,
            re.I | re.S,
        )
        colors = _colors(raw_pair.group("body") if raw_pair else "")
        if not colors:
            raise SourceSchemaError(
                "official Tamer evolution route has no color",
                source="official",
                details={"label": label, "value": value},
            )
        routes.append(
            {
                "slot": int(label_match.group(1)),
                "colors": colors,
                "cost": int(match.group(1)),
                "from_kind": "Tamer",
            }
        )
    return routes


def _text_sections(fragment: str) -> list[dict[str, str]]:
    sections: list[dict[str, str]] = []
    for label, value in _paired_dl(fragment, "cardInfoTitSmall"):
        normalized_label = label.strip().strip("[]")
        if value:
            sections.append({"label": normalized_label, "text": value})
    return sections


def _qa_entries(block: str) -> list[dict[str, str]]:
    # FAQ answers can contain a nested ``relatedList`` with ``<li>`` elements.
    # Splitting on the outer ``cardFaqListItem`` closing tag therefore truncates
    # current pages at the first related card. Match the stable question/answer
    # fields directly and allow arbitrary nested markup inside the answer.
    entries: list[dict[str, str]] = []
    item_pattern = re.compile(
        r"<p\b[^>]*class=[\"'][^\"']*\bcardFaqNum\b[^\"']*[\"'][^>]*>"
        r"(?P<number>.*?)</p>"
        r"(?P<meta>.*?)"
        r"<dt\b[^>]*class=[\"'][^\"']*\bcardFaqQuestion\b[^\"']*[\"'][^>]*>"
        r"(?P<question>.*?)</dt>\s*"
        r"<dd\b[^>]*class=[\"'][^\"']*\bcardFaqAnswer\b[^\"']*[\"'][^>]*>"
        r"(?P<answer>.*?)</dd>",
        re.I | re.S,
    )
    for match in item_pattern.finditer(block):
        date = _class_fragment(match.group("meta"), "cardFaqDate")
        entries.append(
            {
                "number": _clean(match.group("number")),
                "date": _clean(date or ""),
                "question": _clean(match.group("question"), preserve_breaks=True),
                "answer": _clean(match.group("answer"), preserve_breaks=True),
            }
        )
    declared = len(re.findall(r"\bcardFaqNum\b", block))
    if declared != len(entries):
        raise SourceSchemaError(
            "official Q&A item is missing a question or answer",
            source="official",
            details={"declared_items": declared, "parsed_items": len(entries)},
        )
    return entries


def _rule_grants(text_sections: Iterable[Mapping[str, str]]) -> dict[str, list[str]]:
    traits: list[str] = []
    attributes: list[str] = []
    forms: list[str] = []
    for section in text_sections:
        text = section["text"]
        traits.extend(match.strip() for match in _RULE_TRAIT.findall(text))
        attributes.extend(match.strip() for match in _RULE_ATTRIBUTE.findall(text))
        forms.extend(match.strip() for match in _RULE_FORM.findall(text))
    return {
        "traits": list(dict.fromkeys(value for value in traits if value)),
        "attributes": list(dict.fromkeys(value for value in attributes if value)),
        "forms": list(dict.fromkeys(value for value in forms if value)),
    }


def _parse_printing(
    block: str,
    *,
    source_url: str,
    category_id: str,
) -> OfficialPrinting:
    card_no = _class_fragment(block, "cardNo", tag="li")
    if card_no is None:
        raise SourceSchemaError(
            "official card block is missing card number",
            source="official",
            details={"source_url": source_url},
        )
    card_id = _clean(card_no).upper()
    gameplay_set_prefix(card_id)  # validates shape

    dual_start = re.search(
        r"<div\b[^>]*class=[\"'][^\"']*\bdualCardCol\b[^\"']*[\"']",
        block,
        re.I,
    )
    primary = block[: dual_start.start()] if dual_start else block
    dual = block[dual_start.start() :] if dual_start else ""

    image_match = re.search(
        r"<a\b[^>]*class=[\"'][^\"']*\bcard_img\b[^\"']*[\"'][^>]*>.*?"
        r"<img\b[^>]*(?:src|data-src)=[\"'](?P<src>[^\"']+)[\"']",
        block,
        re.I | re.S,
    )
    if not image_match:
        raise SourceSchemaError(
            "official card block is missing printing image identity",
            source="official",
            card_id=card_id,
            details={"source_url": source_url},
        )
    image_url = urljoin(source_url, image_match.group("src"))
    printing_id = _printing_id(image_url, card_id)
    alternate_art = printing_id != card_id
    printing_field_gaps: list[dict[str, object]] = []

    title = _class_fragment(primary, "cardTitle")
    card_type = _class_fragment(primary, "cardType", tag="li")
    rarity = _class_fragment(primary, "cardRarity", tag="li")
    level = _class_fragment(primary, "cardLv", tag="li")
    pairs = {label: value for label, value in _paired_dl(primary, "cardInfoTit")}
    primary_sections = _text_sections(primary)
    primary_color_fragment = ""
    color_pair = re.search(
        r"<dt\b[^>]*>\s*Color\s*</dt>\s*<dd\b[^>]*>(?P<body>.*?)</dd>",
        primary,
        re.I | re.S,
    )
    if color_pair:
        primary_color_fragment = color_pair.group("body")

    normalized_card_type = _official_card_type(card_type or "")
    first_face = {
        "face_id": "primary",
        "kind": normalized_card_type.split("/", 1)[0].strip().lower().replace("-", "_"),
        "name": _clean(title or ""),
        "colors": _colors(primary_color_fragment),
        "play_cost": _parse_int(pairs.get("Cost", "")),
        "level": _parse_int(_clean(level or "")),
        "dp": _parse_int(pairs.get("DP", "")),
        "text_sections": primary_sections,
    }
    faces: list[dict[str, object]] = [first_face]
    dual_sections: list[dict[str, str]] = []
    if dual:
        dual_title = _class_fragment(dual, "cardTitle")
        dual_pairs = {label: value for label, value in _paired_dl(dual, "cardInfoTit")}
        dual_color_pair = re.search(
            r"<dt\b[^>]*>\s*DUAL Color\s*</dt>\s*<dd\b[^>]*>(?P<body>.*?)</dd>",
            dual,
            re.I | re.S,
        )
        dual_sections = _text_sections(dual)
        faces.append(
            {
                "face_id": "dual",
                "kind": "option",
                "name": _clean(dual_title or ""),
                "colors": _colors(dual_color_pair.group("body") if dual_color_pair else ""),
                "use_cost": _parse_int(dual_pairs.get("DUAL Cost", "")),
                "text_sections": dual_sections,
            }
        )
    missing_face_fields = [
        field
        for field, value in (
            ("kind", first_face["kind"]),
            ("colors", first_face["colors"]),
        )
        if not value
    ]
    if alternate_art and missing_face_fields:
        faces = []
        printing_field_gaps.append(
            {
                "field": "faces",
                "reason": "official alternate-art row omitted structural face metadata",
                "missing_fields": missing_face_fields,
            }
        )

    all_sections = primary_sections + dual_sections
    special_routes = [
        section["text"]
        for section in all_sections
        if "digivolution condition" in section["label"].lower()
        or "dna digivolution" in section["label"].lower()
    ]
    try:
        evolution_routes = _standard_evolution_routes(primary)
    except SourceSchemaError:
        route_values = [
            value
            for label, value in _paired_dl(primary, "cardInfoTit")
            if re.fullmatch(r"Digivolve Cost\s+\d+", label, re.I)
        ]
        incomplete_alternate_route = re.compile(
            r"(?:\d+\s+from(?:\s+Lv\.?\s*\d+)?|[A-Za-z /-]+\s+from\s+Lv\.?\s*\d+|from\s+Lv\.?\s*\d+)",
            re.I,
        )
        if alternate_art and route_values and all(
            incomplete_alternate_route.fullmatch(value) for value in route_values
        ):
            evolution_routes = []
            printing_field_gaps.append(
                {
                    "field": "evolution_routes",
                    "reason": "official alternate-art row omitted route color and source level",
                    "source_values": route_values,
                }
            )
        else:
            raise

    extracted: dict[str, object] = {
        "card_id": card_id,
        "printing": {
            "printing_id": printing_id,
            "image_url": image_url,
            "alternate_art": alternate_art,
            "category_id": category_id,
        },
        "name": _clean(title or ""),
        "rarity": _clean(rarity or ""),
        "card_type": normalized_card_type,
        "level": _parse_int(_clean(level or "")),
        "colors": first_face["colors"],
        "play_cost": first_face["play_cost"],
        "dp": first_face["dp"],
        "form": pairs.get("Form", ""),
        "attribute": pairs.get("Attribute", ""),
        "type": pairs.get("Type", ""),
        "evolution_routes": evolution_routes,
        "grade_evolution_routes": _grade_evolution_routes(primary),
        "tamer_evolution_routes": _tamer_evolution_routes(primary),
        "special_evolution_routes": special_routes,
        "text_sections": all_sections,
        "rule_grants": _rule_grants(all_sections),
        "qa": _qa_entries(block),
        "layout": "dual" if dual else "single",
        "faces": faces,
        "printing_field_gaps": printing_field_gaps,
    }
    return OfficialPrinting(
        card_id=card_id,
        printing_id=printing_id,
        category_id=category_id,
        source_url=source_url,
        image_url=image_url,
        alternate_art=alternate_art,
        extracted=extracted,
    )


def parse_card_list(
    payload: bytes | str,
    *,
    source_url: str,
    category_id: str | None = None,
) -> tuple[OfficialPrinting, ...]:
    """Parse every source-visible printing without collapsing gameplay IDs."""

    document = _decode_payload(payload, source_url=source_url)
    if category_id is None:
        category_id = parse_qs(urlparse(source_url).query).get("category", ("",))[0]
    blocks = _card_blocks(document)
    if not blocks:
        raise SourceSchemaError(
            "official card-list page contains no card blocks",
            source="official",
            details={"source_url": source_url, "category_id": category_id},
        )
    printings: list[OfficialPrinting] = []
    for block in blocks:
        raw_card_id = _class_fragment(block, "cardNo", tag="li")
        card_id = _clean(raw_card_id or "").upper()
        # Official token/reference cards are source-visible but are not
        # gameplay deck-card IDs, canonical real cards, or index candidates.
        if _TOKEN_CARD_ID.fullmatch(card_id):
            continue
        printings.append(
            _parse_printing(block, source_url=source_url, category_id=category_id)
        )
    return tuple(printings)


def _as_bytes(payload: bytes | str) -> bytes:
    return payload if isinstance(payload, bytes) else payload.encode("utf-8")


def discover_release_set(
    catalog_payload: bytes | str,
    *,
    release_prefix: str,
    fetch_page: Callable[[str], bytes | str],
    catalog_url: str = OFFICIAL_CATALOG_URL,
) -> OfficialDiscoveryResult:
    """Discover a complete distinct release-set scope and retain all printings."""

    prefix = normalize_release_prefix(release_prefix)
    entries = tuple(
        entry for entry in parse_catalog(catalog_payload, source_url=catalog_url)
        if prefix in entry.release_prefixes
    )
    if not entries:
        raise CompletenessError(
            f"official catalog has no release category for {prefix}",
            source="official",
            details={"scope": f"release-set:{prefix}"},
        )
    pages: list[OfficialFetchedPage] = []
    retained: list[OfficialPrinting] = []
    for entry in entries:
        raw = _as_bytes(fetch_page(entry.source_url))
        parsed = parse_card_list(raw, source_url=entry.source_url, category_id=entry.category_id)
        in_scope = tuple(
            printing for printing in parsed if gameplay_set_prefix(printing.card_id) == prefix
        )
        if not in_scope:
            raise CompletenessError(
                f"official release category {entry.category_id} yielded no {prefix} cards",
                source="official",
                details={"scope": f"release-set:{prefix}", "source_url": entry.source_url},
            )
        pages.append(
            OfficialFetchedPage(
                category=entry,
                source_url=entry.source_url,
                raw_payload=raw,
                printings=in_scope,
            )
        )
        retained.extend(in_scope)
    card_ids = tuple(sorted({printing.card_id for printing in retained}))
    if not card_ids:
        raise CompletenessError(
            f"official release-set discovery returned an empty {prefix} scope",
            source="official",
            details={"scope": f"release-set:{prefix}"},
        )
    return OfficialDiscoveryResult(
        scope=f"release-set:{prefix}",
        gameplay_card_ids=card_ids,
        pages=tuple(pages),
        complete=True,
    )


def discover_catalog(
    catalog_payload: bytes | str,
    *,
    fetch_page: Callable[[str], bytes | str],
    catalog_url: str = OFFICIAL_CATALOG_URL,
) -> OfficialDiscoveryResult:
    """Discover the full official catalog across every declared product category."""

    entries = parse_catalog(catalog_payload, source_url=catalog_url)
    pages: list[OfficialFetchedPage] = []
    for entry in entries:
        raw = _as_bytes(fetch_page(entry.source_url))
        parsed = parse_card_list(raw, source_url=entry.source_url, category_id=entry.category_id)
        pages.append(
            OfficialFetchedPage(
                category=entry,
                source_url=entry.source_url,
                raw_payload=raw,
                printings=parsed,
            )
        )
    ids = tuple(sorted({printing.card_id for page in pages for printing in page.printings}))
    if not ids:
        raise CompletenessError(
            "official full-catalog discovery returned an empty scope",
            source="official",
            details={"scope": "full-pool:official-catalog"},
        )
    return OfficialDiscoveryResult(
        scope="full-pool:official-catalog",
        gameplay_card_ids=ids,
        pages=tuple(pages),
        complete=True,
    )


def raw_payload_digest(payload: bytes | str) -> str:
    """Return the digest of the exact received source bytes."""

    return sha256_hex(_as_bytes(payload))


def build_scope_snapshot(
    discovery: OfficialDiscoveryResult,
    *,
    catalog_payload: bytes | str,
    first_acquired_at: str,
    existing_observations: Mapping[str, SourceObservation] | None = None,
    parser_version: str = OFFICIAL_PARSER_VERSION,
    source_schema_version: str = OFFICIAL_SOURCE_SCHEMA_VERSION,
) -> OfficialScopeSnapshot:
    """Create immutable printing observations and a complete scope manifest."""

    if not discovery.complete:
        raise CompletenessError(
            "cannot snapshot an incomplete official discovery",
            source="official",
            details={"scope": discovery.scope},
        )
    if not first_acquired_at.strip():
        raise ValueError("first_acquired_at must be non-empty")
    existing = existing_observations or {}
    observations_by_id: dict[str, SourceObservation] = {}
    refs_by_id: dict[str, ScopeObservationRef] = {}
    for page in discovery.pages:
        payload_digest = raw_payload_digest(page.raw_payload)
        for printing in page.printings:
            identity = {
                "raw_payload_digest": payload_digest,
                "parser_version": parser_version,
                "source_schema_version": source_schema_version,
                "extracted": printing.extracted,
            }
            observation_id = content_digest(identity, exclude_operational=True)
            prior = existing.get(observation_id)
            acquired_at = prior.first_acquired_at if prior is not None else first_acquired_at
            observation = SourceObservation(
                schema_version=SCHEMA_VERSION,
                observation_id=observation_id,
                source=SourceName.OFFICIAL,
                source_record_key=(
                    f"{printing.category_id}:{printing.card_id}:{printing.printing_id}"
                ),
                source_url=printing.source_url,
                raw_payload_digest=payload_digest,
                parser_version=parser_version,
                source_schema_version=source_schema_version,
                first_acquired_at=acquired_at,
                extracted=printing.extracted,
            )
            observations_by_id.setdefault(observation_id, observation)
            refs_by_id.setdefault(
                observation_id,
                ScopeObservationRef(
                    observation_id=observation_id,
                    gameplay_card_id=printing.card_id,
                    printing_id=printing.printing_id,
                ),
            )
    observations = tuple(observations_by_id[key] for key in sorted(observations_by_id))
    if len(observations) < len(discovery.gameplay_card_ids):
        raise CompletenessError(
            "official snapshot has fewer observations than gameplay IDs",
            source="official",
            details={
                "scope": discovery.scope,
                "observation_count": len(observations),
                "gameplay_card_count": len(discovery.gameplay_card_ids),
            },
        )
    manifest = ScopeManifest(
        schema_version=SCHEMA_VERSION,
        source=SourceName.OFFICIAL,
        scope=discovery.scope,
        source_locations=discovery.source_locations,
        first_acquired_at=min(observation.first_acquired_at for observation in observations),
        parser_version=parser_version,
        source_schema_version=source_schema_version,
        observation_ids=tuple(observation.observation_id for observation in observations),
        observation_refs=tuple(refs_by_id[observation.observation_id] for observation in observations),
        complete=True,
        expected_count=len(discovery.gameplay_card_ids),
        failed_ids=(),
    )
    # The catalog digest is intentionally computed here even though it is not a
    # card observation.  A future transaction plan can bind it without changing
    # the identity of any printing observation.
    raw_payload_digest(catalog_payload)
    return OfficialScopeSnapshot(
        manifest=manifest,
        observations=observations,
    )


class OfficialSourceAdapter:
    """Injected-I/O adapter for official catalog discovery and acquisition."""

    def __init__(
        self,
        fetcher: FetchBytes | None = None,
        *,
        catalog_url: str = OFFICIAL_CATALOG_URL,
        timeout_seconds: float = 30.0,
    ) -> None:
        self.catalog_url = catalog_url
        self.timeout_seconds = timeout_seconds
        self._fetcher = fetcher or self._fetch_live

    def _fetch_live(self, url: str) -> bytes:
        request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        try:
            with urllib.request.urlopen(request, timeout=self.timeout_seconds) as response:
                return response.read()
        except urllib.error.HTTPError as exc:
            raise HttpError(
                f"official source returned HTTP {exc.code}",
                status_code=exc.code,
                source_url=url,
                source="official",
            ) from exc
        except (urllib.error.URLError, TimeoutError, OSError) as exc:
            raise NetworkError(
                f"official source transport failed: {exc}",
                source="official",
                details={"source_url": url},
            ) from exc

    def fetch_url(self, url: str) -> bytes:
        """Fetch one official URL through the configured typed byte transport."""

        return self._fetcher(url)

    def discover_release_set(self, release_prefix: str) -> tuple[bytes, OfficialDiscoveryResult]:
        catalog_payload = self.fetch_url(self.catalog_url)
        discovery = discover_release_set(
            catalog_payload,
            release_prefix=release_prefix,
            fetch_page=self.fetch_url,
            catalog_url=self.catalog_url,
        )
        return catalog_payload, discovery

    def discover_catalog(self) -> tuple[bytes, OfficialDiscoveryResult]:
        catalog_payload = self.fetch_url(self.catalog_url)
        discovery = discover_catalog(
            catalog_payload,
            fetch_page=self.fetch_url,
            catalog_url=self.catalog_url,
        )
        return catalog_payload, discovery

    def acquire_release_set(
        self,
        release_prefix: str,
        *,
        first_acquired_at: str,
        existing_observations: Mapping[str, SourceObservation] | None = None,
    ) -> OfficialScopeSnapshot:
        catalog_payload, discovery = self.discover_release_set(release_prefix)
        return build_scope_snapshot(
            discovery,
            catalog_payload=catalog_payload,
            first_acquired_at=first_acquired_at,
            existing_observations=existing_observations,
        )

    def acquire_catalog(
        self,
        *,
        first_acquired_at: str,
        existing_observations: Mapping[str, SourceObservation] | None = None,
    ) -> OfficialScopeSnapshot:
        catalog_payload, discovery = self.discover_catalog()
        return build_scope_snapshot(
            discovery,
            catalog_payload=catalog_payload,
            first_acquired_at=first_acquired_at,
            existing_observations=existing_observations,
        )


__all__ = [
    "OFFICIAL_CATALOG_URL",
    "OFFICIAL_PARSER_VERSION",
    "OFFICIAL_SOURCE_SCHEMA_VERSION",
    "OfficialCatalogEntry",
    "OfficialDiscoveryResult",
    "OfficialFetchedPage",
    "OfficialPrinting",
    "OfficialScopeSnapshot",
    "OfficialSourceAdapter",
    "build_scope_snapshot",
    "discover_catalog",
    "discover_release_set",
    "gameplay_set_prefix",
    "normalize_release_prefix",
    "parse_card_list",
    "parse_catalog",
    "raw_payload_digest",
]
