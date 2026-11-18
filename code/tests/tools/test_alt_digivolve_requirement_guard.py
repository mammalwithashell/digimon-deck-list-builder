"""Guard: a YAML-implemented card must carry the alternate digivolution its card
prints.

`cards.json` carries a card's special digivolution only as the free-text
`xros_req` field ("[Digivolve] [Koromon]: Cost 0", "[Digivolve] Lv.2 w/[CS]
trait: Cost 0", ...). The engine reads digivolution routes from `evo_costs` and
the YAML `alt_paths`, never from `xros_req`, so a card implemented without an
`alt_paths` entry for its printed special route silently loses it: a base that
satisfies both the circle and the special route never gets DCGO's "Which
digivolution cost do you pay?" prompt (rule 17), and a base that satisfies only
the special route has no legal digivolve at all (hunt cluster
alt-digivolve-xros-req-cards: EX11-007, BT20-038, BT11-043, EX11-025, BT22-044,
BT22-046, BT16-070).

The official Bandai DB bundle (`data/card_bundles/<ID>.md`, authoritative for
printed digivolution data) prints the special route under "Special digivolution
condition". This test scans every bundle whose special condition is a
`[Digivolve] ... Cost N` / `[Digivolve] N from ...` line and fails when the card
has a YAML spec but the spec declares no `alt_paths`. Cards without a YAML spec
are the unimplemented ones and are out of scope here (they are tracked by the
authoring queue).

Known gaps are listed in KNOWN_GAPS with the reason; remove an entry when the
card is fixed. The list must not grow: a NEW hit means a freshly implemented
card dropped its printed alternate route.
"""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
BUNDLES = ROOT / "data" / "card_bundles"
CARDS = ROOT / "code" / "digimon-engine" / "cards"

# card id -> why the card is exempt today.
KNOWN_GAPS = {
    "BT12-031": (
        "Imperialdramon: Fighter Mode prints '[Digivolve] 2 from [Imperialdramon: "
        "Dragon Mode]' but its YAML only has the printed Lv.5 circle (via cards.json)."
    ),
    "BT15-101": (
        "MetalGarurumon's conditional '[Gabumon] digivolves for 4 while a [Matt Ishida] "
        "Tamer and a 10000+ DP opponent Digimon' path was omitted pending the alt-path "
        "`condition:` gate, which now exists (G-ALT-PATH-CONDITION resolved)."
    ),
}

SPECIAL = re.compile(r"### Special digivolution condition[^\n]*\n(.*?)(?:\n\s*\n|\Z)", re.S)
DIGIVOLVE_LINE = re.compile(r"^\s*\[Digivolve\]\s*(?:.*\bCost\s*\d+|\d+\s+from\b)", re.M)


def _yaml_index() -> dict[str, Path]:
    return {p.stem: p for p in CARDS.glob("*/*.yaml") if not p.parent.name.startswith("_")}


def _has_alt_paths(path: Path) -> bool:
    return re.search(r"^alt_paths:", path.read_text(encoding="utf-8"), re.M) is not None


def test_yaml_cards_keep_their_printed_special_digivolution():
    yamls = _yaml_index()
    missing: list[str] = []
    for bundle in sorted(BUNDLES.glob("*.md")):
        card_id = bundle.stem
        spec = yamls.get(card_id)
        if spec is None:
            continue  # unimplemented: nothing to drop yet
        m = SPECIAL.search(bundle.read_text(encoding="utf-8"))
        if not m or not DIGIVOLVE_LINE.search(m.group(1)):
            continue
        if not _has_alt_paths(spec):
            missing.append(card_id)
    unexpected = sorted(set(missing) - set(KNOWN_GAPS))
    assert not unexpected, (
        "YAML-implemented cards print a special '[Digivolve] ... Cost N' route (official "
        f"bundle) but declare no `alt_paths`: {unexpected}. Add the route as an "
        "`alt_paths` entry (see cards/ex11/EX11-014.yaml) plus a cost-choice test "
        "(tests/cards_behavioral/alt_digivolve_support.rs)."
    )
    fixed = sorted(set(KNOWN_GAPS) - set(missing))
    assert not fixed, f"KNOWN_GAPS entries now fixed; remove them: {fixed}"


def test_the_hunt_cards_carry_their_special_route():
    yamls = _yaml_index()
    for card_id in (
        "EX11-007", "BT20-038", "BT11-043", "EX11-025", "BT22-044", "BT22-046", "BT16-070",
        "BT20-031", "BT20-039", "BT22-012", "BT22-043", "BT22-054",
    ):
        assert card_id in yamls, f"{card_id} must have a YAML spec"
        assert _has_alt_paths(yamls[card_id]), card_id
