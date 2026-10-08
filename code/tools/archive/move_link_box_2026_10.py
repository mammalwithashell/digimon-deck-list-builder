#!/usr/bin/env python3
"""One-time migration: file each link card's link effect under cards.json's `link_effect_description_eng` (2026-10-05).

A link card prints a link box (general_rule.pdf 2-3-12): a link requirement, link DP and
a link effect, which a Digimon gains while the card is one of its link cards (4-2-6). The
digimoncard.io API returns the link effect as `source_effect`, the field ingest_cards files
as the inherited effect, and until this migration all 77 link cards in the official Bandai
DB mirror held it in `inherited_effect_description_eng`:
  - mostly without its timing ("All of your opponent's Security Digimon get -3000 DP." for
    BT21-041's "[Your Turn] ..."; "Draw 2 and trash 2 cards in your hand." for BT21-071's
    "[When Linking] <Draw 2> ..."), sometimes truncated (EX10-024, EX10-029) or with its
    keywords dropped (BT23-052, BT25-100, BT25-101);
  - on nine cards as a copy of the link requirement ("Link Requirements [Link] [Appmon]
    trait: Cost 2 (Plug this card ...)") with no link effect at all;
  - on the seven BT26 link cards as the printed link effect, restored by an override of
    the inherited field.
Production reads the field as CardData.inherited_text, so a Digimon with a link card among
its digivolution cards gained the card's link effect (4-2-4 grants only inherited effects;
4-7-2: a link card isn't a stacked card): BT21-009 Gatchmon gave <Raid>, BT21-047, BT23-007,
BT25-100 and BT26-010 <Piercing>, BT23-022 and BT25-101 <Security A. +1>. And the DSL's
`has_inherited` predicate ("with inherited effects", 2-3-4-2) matched all 77.

This applies ingest_cards.normalize_link_box, which a re-ingest now runs too, to every link
card, then sets the printed link effect as an override wherever the API's text differs from
it, so a re-ingest keeps it. The printed text is the official mirror's "Link Effect" section
(BT21's cards file the whole box under "Inherited Effect"; the requirement is cut off),
except where the mirror is wrong (LINK_EFFECT_CORRECTIONS, read off the card image). The
seven BT26 overrides of the inherited field move to the link field. None of the 77 prints an
inherited effect besides its link box (official mirror; DCGO's card assets agree, except that
EX10-030's asset files its link effect as inherited, which its card image contradicts), so
their inherited field ends up empty. Only existing per-card engine JSON and data/card_meta
pages are regenerated.

Usage (repo root):
    python code/tools/archive/move_link_box_2026_10.py           # dry run
    python code/tools/archive/move_link_box_2026_10.py --apply
"""
import argparse
import json
import re
import sys
from pathlib import Path

_CODE_DIR = Path(__file__).resolve().parents[2]
if str(_CODE_DIR) not in sys.path:
    sys.path.insert(0, str(_CODE_DIR))

from tools import ingest_cards, split_cards_json  # noqa: E402

NOTE_KEY = "_note_link_box_2026_10"
NOTE = (
    "2026-10-05: a link card's link effect lives in `link_effect_description_eng`, not in the inherited "
    "text where the API's `source_effect` puts it (ingest_cards.normalize_link_box moves it; "
    "code/tools/archive/move_link_box_2026_10.py). The link-effect overrides restore the printed text "
    "(official Bandai DB; EX10-016 from the card image) where the API drops its timing, keywords or the "
    "whole effect."
)

INHERITED = "inherited_effect_description_eng"
LINK = "link_effect_description_eng"

# Link effects the official mirror gets wrong, read off the card image (2026-10-05).
#   EX10-016 Mirrormon: the mirror shows EX10-014 Weatherdramon's link box; the card, DCGO's
#   card asset and the API agree on this text.
LINK_EFFECT_CORRECTIONS = {
    "EX10-016": "[When Attacking] By trashing 1 of this Digimon's link cards, suspend 2 of your opponent's Digimon.",
}

_OFFICIAL_LINK_REQUIREMENT = re.compile(r"^＜Link＞.*?\(Plug this card[^)]*\)\s*", re.S)


def read_json(path: Path):
    """(data, writer) where writer(data) reproduces the file's exact serialisation."""
    raw = path.read_bytes()
    data = json.loads(raw.decode("utf-8"))
    eol = "\r\n" if b"\r\n" in raw else "\n"
    for ensure_ascii in (False, True):
        body = json.dumps(data, indent=2, ensure_ascii=ensure_ascii).replace("\n", eol)
        for tail in (eol, ""):
            if (body + tail).encode("utf-8") == raw:
                def write(new, ea=ensure_ascii, t=tail):
                    out = json.dumps(new, indent=2, ensure_ascii=ea).replace("\n", eol) + t
                    path.write_bytes(out.encode("utf-8"))
                return data, write
    raise SystemExit(f"{path}: serialisation is not json.dumps(indent=2); refusing to rewrite it")


def official_link_box(page):
    """(link requirement, link effect) an official page prints, or None without a link box."""
    sections = page.get("text_sections") or []
    requirement = next((s["text"] for s in sections if s.get("label") == "Link Condition"), None)
    for section in sections:
        if section.get("label") == "Link Effect":
            return requirement, section.get("text") or ""
    for section in sections:
        text = (section.get("text") or "").strip()
        if section.get("label") == "Inherited Effect" and text.startswith("＜Link＞"):
            m = _OFFICIAL_LINK_REQUIREMENT.match(text)
            return (m.group(0).strip() if m else requirement), text[m.end():] if m else text
    return None


def _same(a, b):
    def norm(s):
        return re.sub(r"\s+", " ", (s or "").replace("\xa0", " ").replace("’", "'")).strip()
    return norm(a) == norm(b)


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--apply", action="store_true", help="write the changes (default: dry run)")
    ap.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3], help="repo root")
    args = ap.parse_args(argv)
    root = args.root
    engine_cards = root / "code" / "digimon-engine" / "cards"
    card_meta = root / "data" / "card_meta"

    cards, write_cards = read_json(root / "data" / "cards.json")
    overrides, write_overrides = read_json(root / "data" / "card_overrides.json")
    official = json.loads((root / "data" / "card_official.json").read_text(encoding="utf-8"))["cards"]

    boxes = {}
    for cid, page in official.items():
        box = official_link_box(page)
        if box is not None:
            requirement, effect = box
            boxes[cid] = (requirement, LINK_EFFECT_CORRECTIONS.get(cid, effect))
    missing = sorted(cid for cid in boxes if cid not in cards)
    if missing:
        raise SystemExit(f"link cards missing from cards.json: {missing}")
    print(f"official mirror: {len(boxes)} link cards")

    touched, restored, moved = [], [], []
    for cid in sorted(boxes):
        requirement, printed = boxes[cid]
        card = cards[cid]
        patch = overrides.get(cid, {})
        before = (card.get(INHERITED), card.get(LINK))

        # 1. What a re-ingest produces: the API's text moves to the link field. An override of
        #    the inherited field held the printed link effect; it moves to the link field.
        ingest_cards.normalize_link_box(card, {"link_requirements": requirement or "(official mirror)"})
        if INHERITED in patch:
            del patch[INHERITED]
            moved.append(cid)
        # 2. Restore the printed link effect wherever the API's text differs from it.
        if cid in moved or not _same(card.get(LINK), printed):
            patch[LINK] = printed
            overrides[cid] = patch
            restored.append(cid)
        if cid in overrides:
            ingest_cards.apply_overrides({cid: card}, {cid: overrides[cid]})
        if (card.get(INHERITED), card.get(LINK)) != before:
            touched.append(cid)
    overrides[NOTE_KEY] = NOTE

    print(f"cards.json  {len(touched)} cards change")
    print(f"overrides   {len(restored)} link effects restored; inherited override moved for {moved}")
    targets = []
    for cid in touched:
        folder = split_cards_json.set_id_from_card_id(cid)
        for path in (engine_cards / folder / f"{cid}.json", card_meta / folder / f"{cid}.md"):
            if path.exists():
                targets.append(path)
    print(f"rewrite     {len(targets)} per-card files")
    if not args.apply:
        for cid in touched:
            print(f"  {cid}: {cards[cid].get(LINK)!r}")
        print("dry run: nothing written")
        return 0

    # Imported before the first write: resolve_deck needs the digimon_engine bindings, and a
    # missing build must fail here rather than after cards.json has been rewritten.
    from tools import build_card_meta, resolve_deck
    write_overrides(overrides)
    write_cards(cards)
    resolve_deck._CARDS_JSON_CACHE = cards
    for path in targets:
        cid = path.stem
        if path.suffix == ".json":
            split_cards_json.write_card_json(cid, cards[cid], engine_cards)
        else:
            build_card_meta.write_card_meta(cid, card_meta)
    print("applied")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
