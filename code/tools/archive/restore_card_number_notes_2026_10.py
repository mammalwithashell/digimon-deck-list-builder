#!/usr/bin/env python3
"""One-time migration: restore the RB1 card-number notes to data/cards.json (2026-10-05).

Three Resurgence Booster reprints print a note in their effect box (card images; the
official Bandai DB text and Q&A in data/card_official.json agree):

    RB1-004 Agumon    ※Card Number: Also treated as [P-009]. A deck may not have more
                      than 4 total copies of this and [P-009].
    RB1-006 Gammamon  ... [P-058] ...
    RB1-007 Greymon   ... [P-010] ...

The official Q&A calls it a special rule, not an effect: the reprint is treated as a card
with the promo's card number in every area, and a deck may hold at most 4 copies of the
two together (general_rule.pdf 2-11-1 and 1-4-1-2-2). The digimoncard.io API drops the
note, so cards.json had no trace of it and deck validation accepted 4 RB1-004 + 4 P-009.
code/digimon-engine/src/deck_tools.rs now reads the note from `effect_description_eng`;
this restores it there, as data/card_overrides.json entries so a re-ingest keeps it (see
the `_note_card_number_notes_2026_10` key). The official mirror prints no other card-number
note; code/digimon-engine/tests/data_official_parity.rs fails if one appears without
reaching deck validation.

Only these cards' existing per-card engine JSON and data/card_meta pages are regenerated.

Usage (repo root):
    python code/tools/archive/restore_card_number_notes_2026_10.py           # dry run
    python code/tools/archive/restore_card_number_notes_2026_10.py --apply
"""
import argparse
import json
import sys
from pathlib import Path

_CODE_DIR = Path(__file__).resolve().parents[2]
if str(_CODE_DIR) not in sys.path:
    sys.path.insert(0, str(_CODE_DIR))

from tools import ingest_cards, split_cards_json  # noqa: E402

NOTE_KEY = "_note_card_number_notes_2026_10"
NOTE = (
    "2026-10-05: RB1-004 Agumon, RB1-006 Gammamon and RB1-007 Greymon print \"※Card Number: "
    "Also treated as [P-009]/[P-058]/[P-010]. A deck may not have more than 4 total copies of "
    "this and [...].\" (card images, official Bandai DB text and Q&A). The digimoncard.io API "
    "drops the note; deck_tools.rs reads it from effect_description_eng to count each reprint "
    "toward its promo's copy limit (general_rule.pdf 2-11-1, 1-4-1-2-2). "
    "code/tools/archive/restore_card_number_notes_2026_10.py applied it."
)

CORRECTIONS = {
    "RB1-004": {
        "effect_description_eng": "※Card Number: Also treated as [P-009]. A deck may not have more than 4 total copies of this and [P-009]."
    },
    "RB1-006": {
        "effect_description_eng": "[Your Turn] While you have a red Tamer in play, this Digimon may also attack your opponent's unsuspended Digimon.\r\n※Card Number: Also treated as [P-058]. A deck may not have more than 4 total copies of this and [P-058]."
    },
    "RB1-007": {
        "effect_description_eng": "[Your Turn] While this Digimon has an [Agumon] digivolution card, it gains ＜Security A. +1＞ (This Digimon checks 1 additional security card.)\r\n※Card Number: Also treated as [P-010]. A deck may not have more than 4 total copies of this and [P-010]."
    },
}


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

    # 1. overrides: the note, then each card's fields merged into its entry (in place on a re-run).
    overrides[NOTE_KEY] = NOTE
    for cid, fields in CORRECTIONS.items():
        overrides.setdefault(cid, {}).update(fields)

    # 2. cards.json: re-apply each touched card's whole override entry, as a re-ingest would.
    touched = []
    for cid in CORRECTIONS:
        before = dict(cards[cid])
        ingest_cards.apply_overrides({cid: cards[cid]}, {cid: overrides[cid]})
        diff = [k for k in sorted(set(before) | set(cards[cid])) if before.get(k) != cards[cid].get(k)]
        if diff:
            touched.append(cid)
        print(f"cards.json  {cid}: {', '.join(diff) or 'no change'}")

    targets = []
    for cid in touched:
        folder = split_cards_json.set_id_from_card_id(cid)
        for path in (engine_cards / folder / f"{cid}.json", card_meta / folder / f"{cid}.md"):
            if path.exists():
                targets.append(path)
                print(f"rewrite     {path.relative_to(root)}")
    print(f"{len(touched)} cards change")
    if not args.apply:
        print("dry run: nothing written")
        return 0

    # Imported before the first write: resolve_deck needs the digimon_engine bindings, and a
    # missing build must fail here rather than after cards.json has been rewritten.
    from tools import build_card_meta, resolve_deck
    write_overrides(overrides)
    write_cards(cards)
    # build_card_meta renders from resolve_deck's cached cards.json; seed it with the
    # corrected data so --root works on a scratch copy too.
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
