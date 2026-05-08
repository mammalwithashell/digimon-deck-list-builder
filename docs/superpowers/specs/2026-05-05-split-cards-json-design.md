# Split Cards JSON Design

## Goal

Create a small Python CLI that splits `data/cards.json` into one JSON file per card under the Rust DSL card directory so implementation agents can read a single card record without loading the full card database.

## Output Layout

The tool writes every card in `data/cards.json` to:

```text
code/digimon-engine/cards/<set_lower>/<CARD-ID>.json
```

The set directory is the lowercase card ID prefix before the first hyphen. Cards without a hyphen are written under `_misc`.

Examples:

```text
BT24-001 -> code/digimon-engine/cards/bt24/BT24-001.json
P-229 -> code/digimon-engine/cards/p/P-229.json
PROMO123 -> code/digimon-engine/cards/_misc/PROMO123.json
```

The JSON file lives beside the card YAML when YAML already exists. For cards that do not have YAML yet, the tool still creates the set directory and writes the raw card JSON there.

## Data Shape

Each per-card file preserves the complete card object from `data/cards.json`, including printed effect fields, evolution costs, metadata, and any parser-specific fields. The tool does not trim or reinterpret the card data.

Output is deterministic:

- Cards are processed in sorted card ID order.
- JSON is written as UTF-8 with `indent=2`.
- Files use LF newlines.
- A trailing newline is included.

## CLI

The tool is `code/tools/split_cards_json.py` and supports:

```bash
python code/tools/split_cards_json.py
python code/tools/split_cards_json.py --card BT24-001
python code/tools/split_cards_json.py --set bt24
python code/tools/split_cards_json.py --check
```

Default mode writes all cards. `--card` and `--set` support partial rebuilds. `--check` regenerates expected content in memory and returns a nonzero exit code if any expected file is missing or stale.

## Testing

Tests live in `code/tests/tools/test_split_cards_json.py`.

Coverage includes:

- Card ID to set directory bucketing.
- Writing one card with stable JSON and LF newlines.
- Writing all cards from an injected fixture dictionary.
- `--check` success when files match.
- `--check` failure for missing or stale files.

## Non-Goals

The tool does not change Rust card loading, DSL linting, or YAML semantics. The generated JSON files are source-context companions for agents and humans, not engine-owned card definitions.
