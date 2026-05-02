from __future__ import annotations

import argparse
from pathlib import Path


ALLOWED_SRC_CARDS_CHILDREN = {"test", "tokens", "raw_rust"}
ALLOWED_SRC_CARDS_FILES = {"keyword_effects.rs", "mod.rs"}


def production_rust_cards(src_cards: Path) -> list[Path]:
    found: list[Path] = []
    for path in src_cards.rglob("*.rs"):
        rel = path.relative_to(src_cards)
        if len(rel.parts) == 1 and rel.name in ALLOWED_SRC_CARDS_FILES:
            continue
        if rel.parts[0] in ALLOWED_SRC_CARDS_CHILDREN:
            continue
        found.append(path)
    return sorted(found)


def yaml_cards(cards_dir: Path) -> set[str]:
    return {p.stem.upper() for p in cards_dir.rglob("*.yaml") if "_examples" not in p.parts}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--engine", default="code/digimon-engine")
    args = parser.parse_args()

    engine = Path(args.engine)
    src_cards = engine / "src" / "cards"
    cards_dir = engine / "cards"
    rust = production_rust_cards(src_cards)
    yaml_ids = yaml_cards(cards_dir)

    print("# DSL long-tail report")
    print(f"production_rust_card_modules={len(rust)}")
    print(f"yaml_card_files={len(yaml_ids)}")
    for path in rust:
        print(path.as_posix())

    return 1 if rust else 0


if __name__ == "__main__":
    raise SystemExit(main())
